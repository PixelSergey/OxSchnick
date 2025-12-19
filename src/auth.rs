use std::collections::HashMap;

use axum::{
    extract::{self, FromRequestParts, Query, Request},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_extra::extract::{CookieJar, cookie::Cookie};
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use log::error;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot, watch};
use uuid::Uuid;

use crate::state::State;

const AUTHENTICATOR_COOKIE_NAME: &'static str = "session";
const AUTHENTICATOR_CHANNEL_BUFFER: usize = 128usize;
const AUTHENTICATOR_ROOT_ID: i32 = 1i32;

#[derive(Debug)]
pub enum AuthenticationRequest {
    Authenticate {
        id: i32,
        token: Uuid,
        callback: oneshot::Sender<Result<Uuid, StatusCode>>,
    },
    Register {
        parent: i32,
        invite: Uuid,
        callback: oneshot::Sender<Result<(Authenticated, Uuid), StatusCode>>,
    },
    RenewInvite {
        id: i32,
        callback: oneshot::Sender<Result<(), StatusCode>>,
    },
}

#[derive(Debug, Clone)]
pub struct AuthenticatorEntry {
    pub token: Uuid,
    pub invite: Uuid,
    pub channel: watch::Sender<()>,
}

pub struct Authenticator {
    cache: HashMap<i32, AuthenticatorEntry>,
    connection: AsyncPgConnection,
    sender: mpsc::Sender<AuthenticationRequest>,
    receiver: mpsc::Receiver<AuthenticationRequest>,
}

#[derive(Debug, Clone, HasQuery, QueryableByName, Identifiable, Serialize, Deserialize)]
#[diesel(table_name=crate::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Authenticated {
    pub id: i32,
    pub token: Uuid,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Invite {
    pub id: i32,
    pub token: Uuid,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name=crate::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewUser {
    pub parent: i32,
}

impl Authenticator {
    pub fn with_connection(connection: AsyncPgConnection) -> Self {
        let (sender, receiver) = mpsc::channel(AUTHENTICATOR_CHANNEL_BUFFER);
        Self {
            cache: Default::default(),
            connection,
            sender,
            receiver,
        }
    }

    async fn register(
        &mut self,
        parent: i32,
        submitted_invite: &Uuid,
    ) -> Result<(Authenticated, Uuid), StatusCode> {
        use crate::schema::users;
        let entry = self
            .cache
            .get(&parent)
            .ok_or(StatusCode::FORBIDDEN)?
            .clone();
        if &entry.invite == submitted_invite {
            let new_user = NewUser { parent: parent };
            let new_authenticated = new_user
                .insert_into(users::table)
                .returning((users::id, users::token))
                .get_result::<Authenticated>(&mut self.connection)
                .await
                .map_err(|e| {
                    error!(target: "auth::register", "{:?}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
            let invite = Uuid::new_v4();
            self.cache.insert(
                new_authenticated.id,
                AuthenticatorEntry {
                    token: new_authenticated.token,
                    invite: Uuid::new_v4(),
                    channel: watch::Sender::new(()),
                },
            );
            Ok((new_authenticated, invite))
        } else {
            Err(StatusCode::FORBIDDEN)
        }
    }

    async fn authenticate(&mut self, id: i32, submitted_token: &Uuid) -> Result<Uuid, StatusCode> {
        if let Some(AuthenticatorEntry { token, invite, .. }) = self.cache.get(&id) {
            if token == submitted_token {
                Ok(*invite)
            } else {
                Err(StatusCode::FORBIDDEN)
            }
        } else {
            let authenticated = Authenticated::query()
                .find(id)
                .first(&mut self.connection)
                .await
                .optional()
                .map_err(|e| {
                    error!(target: "auth::authenticate", "{:?}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
            match authenticated {
                Some(Authenticated { token, .. }) if &token == submitted_token => {
                    let invite = Uuid::new_v4();
                    let _ = self.cache.insert(
                        id,
                        AuthenticatorEntry {
                            token,
                            invite,
                            channel: watch::Sender::new(()),
                        },
                    );
                    Ok(invite)
                }
                _ => Err(StatusCode::FORBIDDEN),
            }
        }
    }

    async fn renew_invite(&mut self, id: i32) -> Result<(), StatusCode> {
        let entry = self.cache.get_mut(&id).ok_or(StatusCode::NOT_FOUND)?;
        entry.invite = Uuid::new_v4();
        Ok(())
    }

    pub async fn worker(mut self) {
        while let Some(request) = self.receiver.recv().await {
            match request {
                AuthenticationRequest::Authenticate {
                    id,
                    token,
                    callback,
                } => {
                    let response = self.authenticate(id, &token).await;
                    if let Err(_) = callback.send(response) {
                        error!(target: "auth::worker", "dead receiver");
                    }
                }
                AuthenticationRequest::Register {
                    parent,
                    invite,
                    callback,
                } => {
                    let response = self.register(parent, &invite).await;
                    if let Err(_) = callback.send(response) {
                        error!(target: "auth::worker", "dead receiver");
                    }
                }
                AuthenticationRequest::RenewInvite { id, callback } => {
                    let response = self.renew_invite(id).await;
                    if let Err(_) = callback.send(response) {
                        error!(target: "auth::worker", "dead receiver");
                    }
                }
            }
        }
    }

    pub fn sender(&self) -> mpsc::Sender<AuthenticationRequest> {
        self.sender.clone()
    }

    pub async fn request_authenticate(
        id: i32,
        submitted_token: &Uuid,
        sender: &mpsc::Sender<AuthenticationRequest>,
    ) -> Result<Uuid, StatusCode> {
        let (tx, rx) = oneshot::channel();
        sender
            .send(AuthenticationRequest::Authenticate {
                id,
                token: *submitted_token,
                callback: tx,
            })
            .await
            .map_err(|e| {
                error!(target: "auth::request", "dead channel: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        rx.await.map_err(|e| {
            error!(target: "auth::request", "dead channel: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    }

    pub async fn request_register(
        parent: i32,
        submitted_invite: &Uuid,
        sender: &mpsc::Sender<AuthenticationRequest>,
    ) -> Result<(Authenticated, Uuid), StatusCode> {
        let (tx, rx) = oneshot::channel();
        sender
            .send(AuthenticationRequest::Register {
                parent,
                invite: *submitted_invite,
                callback: tx,
            })
            .await
            .map_err(|e| {
                error!(target: "auth::request", "dead channel: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        rx.await.map_err(|e| {
            error!(target: "auth::request", "dead channel: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    }

    pub async fn request_renew_invite(
        id: i32,
        sender: &mpsc::Sender<AuthenticationRequest>,
    ) -> Result<(), StatusCode> {
        let (tx, rx) = oneshot::channel();
        sender
            .send(AuthenticationRequest::RenewInvite { id, callback: tx })
            .await
            .map_err(|e| {
                error!(target: "auth::request", "dead channel: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        rx.await.map_err(|e| {
            error!(target: "auth::request", "dead channel: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    }

    pub async fn layer_with_registration(
        extract::State(state): extract::State<State>,
        cookies: CookieJar,
        invite: Query<Invite>,
        mut request: Request,
        next: Next,
    ) -> Result<Response, StatusCode> {
        if let Some(session) = cookies
            .get(AUTHENTICATOR_COOKIE_NAME)
            .map(|cookie| cookie.value().as_bytes())
        {
            let submitted_entry = serde_json::from_slice::<Authenticated>(session)
                .map_err(|_| StatusCode::FORBIDDEN)?;
            let invite = Self::request_authenticate(
                submitted_entry.id,
                &submitted_entry.token,
                &state.authenticator,
            )
            .await?;
            request.extensions_mut().insert(submitted_entry);
            request.extensions_mut().insert(UserInvite(invite));
            Ok(next.run(request).await)
        } else {
            let (new_entry, invite) =
                Self::request_register(invite.id, &invite.token, &state.authenticator).await?;
            let cookies = cookies.add(Cookie::new(
                AUTHENTICATOR_COOKIE_NAME,
                // TODO: remove this unwrap
                serde_json::to_string(&new_entry).unwrap(),
            ));
            request.extensions_mut().insert(new_entry);
            request.extensions_mut().insert(UserInvite(invite));
            Ok((cookies, next.run(request).await).into_response())
        }
    }

    pub async fn layer(
        extract::State(state): extract::State<State>,
        cookies: CookieJar,
        mut request: Request,
        next: Next,
    ) -> Result<Response, StatusCode> {
        let session = cookies
            .get(AUTHENTICATOR_COOKIE_NAME)
            .map(|cookie| cookie.value().as_bytes())
            .ok_or(StatusCode::FORBIDDEN)?;
        let submitted_entry =
            serde_json::from_slice::<Authenticated>(session).map_err(|_| StatusCode::FORBIDDEN)?;
        let invite = Self::request_authenticate(
            submitted_entry.id,
            &submitted_entry.token,
            &state.authenticator,
        )
        .await?;
        request.extensions_mut().insert(submitted_entry);
        request.extensions_mut().insert(UserInvite(invite));
        Ok(next.run(request).await)
    }

    pub async fn root_invite(&mut self) -> Option<Invite> {
        let authenticated = Authenticated::query()
            .find(AUTHENTICATOR_ROOT_ID)
            .first(&mut self.connection)
            .await
            .ok()?;
        self.authenticate(authenticated.id, &authenticated.token)
            .await
            .ok()?;
        self.cache
            .get(&authenticated.id)
            .map(|AuthenticatorEntry { invite, .. }| Invite {
                id: authenticated.id,
                token: *invite,
            })
    }
}

impl<S: Send + Sync + 'static> FromRequestParts<S> for Authenticated {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts.extensions.get::<Self>().ok_or_else(|| {
            error!(target: "auth::from_request_parts", "did not get Authenticated in extension");
            StatusCode::INTERNAL_SERVER_ERROR
        }).cloned()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct UserInvite(pub Uuid);

impl<S: Send + Sync + 'static> FromRequestParts<S> for UserInvite {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Self>()
            .ok_or_else(|| {
                error!(target: "auth::from_request_parts", "did not get UserInvite in extension");
                StatusCode::INTERNAL_SERVER_ERROR
            })
            .cloned()
    }
}
