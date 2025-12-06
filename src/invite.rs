use axum::{
    extract::{Query, State},
    http::{StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
};
use axum_extra::extract::{CookieJar, cookie::Cookie};
use diesel::prelude::*;
use log::{debug, trace};
use qrcode::{QrCode, render::svg};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::app::{App, SESSION_COOKIE_NAME};

/// Represents the login information needed to identify and authenticate a user.
#[derive(Debug, Clone, Serialize, Deserialize, HasQuery)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Invite {
    pub id: i32,
    #[diesel(column_name=invite)]
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
) -> Result<(CookieJar, Response), StatusCode> {
    debug!(target: "invite::invite", "invite={invite:?}, cookies={cookies:?}");
    trace!(target: "invite::invite", "authenticating invite");
    app.authenticate_invite(&invite).await?;
    let id = if cookies.get(SESSION_COOKIE_NAME).is_some() {
        trace!(target: "invite::invite", "found session cookie, authenticating");
        app.authenticate(&cookies).await?
    } else {
        trace!(target: "invite::invite", "found no session cookie, registering");
        let session = app.register(invite.id).await?;
        app.renew_invite(invite.id).await?;
        cookies = cookies.add(Cookie::new(
            SESSION_COOKIE_NAME,
            serde_json::to_string(&session).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        ));
        session.id
    };
    Ok((cookies, format!("registered as {id}").into_response()))
}

/// The `/qrcode` route.
pub async fn qrcode(
    State(app): State<App>,
    cookies: CookieJar,
) -> Result<impl IntoResponse, StatusCode> {
    debug!(target: "invite::qrcode", "cookies={cookies:?}");
    let id = app.authenticate(&cookies).await?;
    let invite = app.get_invite(id).await?;
    Ok(([(CONTENT_TYPE, "image/svg+xml")], invite.qrcode(&app.base)?))
}
