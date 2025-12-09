use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Query, State},
    http::{StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::{CookieJar, cookie::Cookie};
use log::{debug, trace};
use qrcode::{QrCode, render::svg};
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, watch::{Receiver, Sender}};
use url::Url;
use uuid::Uuid;

use crate::app::{App, SESSION_COOKIE_NAME};

#[derive(Debug, Clone)]
pub struct InviteHandle {
    token: String,
    channel: Sender<()>
}

#[derive(Debug, Clone, Default)]
pub struct Inviter {
    data: Arc<RwLock<HashMap<i32, InviteHandle>>>
}

impl Inviter {
    /// Checks if a given invite is valid.
    pub async fn check(&self, invite: &Invite) -> Result<(), StatusCode> {
        if let Some(handler) = self.data.read().await.get(&invite.id) {
            if handler.token == invite.token {
                Ok(())
            } else {
                Err(StatusCode::FORBIDDEN)
            }
        } else {
            Err(StatusCode::FORBIDDEN)
        }
    }

    /// Get the active invite for a given user id or create it if it doesnt exist.
    pub async fn get(&self, id: i32) -> Invite {
        let mut data = self.data.write().await;
        if let Some(handler) = data.get(&id) {
            Invite {
                id,
                token: handler.token.clone()
            }
        } else {
            let token = Uuid::new_v4().to_string();
            data.insert(id, InviteHandle { token: token.clone(), channel: Sender::new(()) });
            Invite { id, token }
        }
    }

    /// Invalidate the invite for the given user id.
    pub async fn invalidate(&self, id: i32) {
        self.data.write().await.remove(&id);
    }

    /// Get the receiver end of the channel for this users invite.
    /// 
    /// A `()` will be sent when the invite is used.
    pub async fn receiver(&self, id: i32) -> Option<Receiver<()>> {
        if let Some(handler) = self.data.read().await.get(&id) {
            Some(handler.channel.subscribe())
        } else {
            None
        }
    }

    pub async fn notify(&self, id: i32) {
        if let Some(handler) = self.data.read().await.get(&id) {
            let _ = handler.channel.send(());
        }
    }
}

/// Represents the login information needed to identify and authenticate a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invite {
    pub id: i32,
    pub token: String,
}

impl Invite {
    pub fn url(&self, base: &Url) -> Result<String, StatusCode> {
        let mut url = base
            .join("invite")
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        url.set_query(Some(&format!("id={}&token={}", self.id, self.token)));
        Ok(url.to_string())
    }
    pub fn qrcode(&self, base: &Url) -> Result<String, StatusCode> {
        let code = QrCode::new(&self.url(base)?).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let image = code.render::<svg::Color>().build();
        Ok(image)
    }
}

/// The `/invite` route.
pub async fn invite(
    State(app): State<App>,
    Query(invite): Query<Invite>,
    mut cookies: CookieJar,
) -> Result<(CookieJar, Redirect), StatusCode> {
    debug!(target: "invite::invite", "invite={invite:?}, cookies={cookies:?}");
    trace!(target: "invite::invite", "authenticating invite");
    app.inviter.check(&invite).await?;
    let id = if cookies.get(SESSION_COOKIE_NAME).is_some() {
        trace!(target: "invite::invite", "found session cookie, authenticating");
        app.authenticate(&cookies).await?
    } else {
        trace!(target: "invite::invite", "found no session cookie, registering");
        let session = app.register(invite.id).await?;
        trace!(target: "invite::invite", "registered");
        cookies = cookies.add(Cookie::new(
            SESSION_COOKIE_NAME,
            serde_json::to_string(&session).map_err(|_| StatusCode::INSUFFICIENT_STORAGE)?,
        ));
        session.id
    };
    let other = invite.id;
    if app.have_schnicked(id, other).await? || id == other {
        // TODO: human-readable error here
        return Err(StatusCode::CONFLICT);
    }
    app.start_schnick(id, other).await;
    trace!(target: "invite::invite", "notifying");
    app.inviter.notify(id).await;
    trace!(target: "invite::invite", "invalidating");
    app.inviter.invalidate(invite.id).await;
    trace!(target: "invite::invite", "returning");
    Ok((cookies, Redirect::temporary("schnick")))
}

/// The `/qrcode` route.
pub async fn qrcode(
    State(app): State<App>,
    cookies: CookieJar,
) -> Result<impl IntoResponse, StatusCode> {
    debug!(target: "invite::qrcode", "cookies={cookies:?}");
    let id = app.authenticate(&cookies).await?;
    let invite = app.inviter.get(id).await;
    Ok(([(CONTENT_TYPE, "image/svg+xml")], invite.qrcode(&app.base)?))
}
