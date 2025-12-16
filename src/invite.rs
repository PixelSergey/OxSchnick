use askama::Template;
use axum::{
    extract::{Query, State},
    http::{StatusCode, header::CONTENT_TYPE},
    response::{Html, IntoResponse, Redirect},
};
use axum_extra::extract::{CookieJar, cookie::Cookie};
use log::{debug, trace};
use qrcode::{QrCode, render::svg};
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use crate::{
    app::{App, SESSION_COOKIE_NAME},
    session::{SessionManager, SessionUpdate},
};

impl SessionManager {
    pub async fn check_invite(&self, invite: &Invite) -> Result<(), StatusCode> {
        if let Some(handle) = self.data.read().await.get(&invite.id) {
            if handle.invite == invite.token {
                Ok(())
            } else {
                Err(StatusCode::FORBIDDEN)
            }
        } else {
            Err(StatusCode::FORBIDDEN)
        }
    }

    pub async fn get_invite(&self, id: i32) -> Result<Invite, StatusCode> {
        if let Some(handle) = self.data.read().await.get(&id) {
            Ok(Invite {
                id: id,
                token: handle.invite.clone(),
            })
        } else {
            Err(StatusCode::NOT_FOUND)
        }
    }

    pub async fn renew_invite(&self, id: i32) -> Result<(), StatusCode> {
        if let Some(handle) = self.data.write().await.get_mut(&id) {
            handle.invite = Uuid::new_v4().to_string();
            Ok(())
        } else {
            Err(StatusCode::NOT_FOUND)
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
    app.sessions.check_invite(&invite).await?;
    let id = if cookies.get(SESSION_COOKIE_NAME).is_some() {
        trace!(target: "invite::invite", "found session cookie, authenticating");
        app.authenticate(&cookies).await?
    } else {
        trace!(target: "invite::invite", "found no session cookie, registering");
        let session = app.register(invite.id).await?;
        trace!(target: "invite::invite", "registered");
        cookies = cookies.add(Cookie::new(
            SESSION_COOKIE_NAME,
            serde_json::to_string(&session).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        ));
        session.id
    };
    let other = invite.id;
    if app.have_schnicked(id, other).await? || id == other {
        // TODO: human-readable error here
        return Err(StatusCode::CONFLICT);
    }
    app.sessions.start_schnick(id, other).await?;
    trace!(target: "invite::invite", "notifying");
    let _ = app
        .sessions
        .sender(invite.id)
        .await?
        .send(SessionUpdate::SchnickStarted);
    trace!(target: "invite::invite", "invalidating");
    app.sessions.renew_invite(invite.id).await?;
    trace!(target: "invite::invite", "returning");
    Ok((cookies, Redirect::temporary("..")))
}

#[derive(Debug, Clone, Template)]
#[template(path="qrcode.html")]
pub struct Qrcode {
    pub qrcode: String,
}

/// The `/qrcode` route.
pub async fn qrcode(
    State(app): State<App>,
    cookies: CookieJar,
) -> Result<impl IntoResponse, StatusCode> {
    debug!(target: "invite::qrcode", "cookies={cookies:?}");
    let id = app.authenticate(&cookies).await?;
    let invite = app.sessions.get_invite(id).await?;
    let qrcode = invite.qrcode(&app.base)?;
    Ok(Html(Qrcode {
        qrcode
    }.render().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?))
}
