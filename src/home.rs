use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
};
use axum_extra::extract::CookieJar;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use log::debug;

use crate::{app::App, settings::User};

#[derive(Template)]
#[template(path = "home.html")]
struct Home {
    pub username: Option<String>,
    pub invite: String,
}

/// The `/home` route.
pub async fn home(
    State(app): State<App>,
    cookies: CookieJar,
) -> Result<impl IntoResponse, StatusCode> {
    use crate::schema::users;
    debug!(target: "home::home", "cookies={cookies:?}");
    let id = app.authenticate(&cookies).await?;
    let user = User::query()
        .filter(users::id.eq(id))
        .first(
            &mut app
                .connection()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let invite = app.sessions.get_invite(id).await?;
    let invite_url = invite.url(&app.base)?;
    Ok(Html(
        Home {
            username: user.username,
            invite: invite_url,
        }
        .render()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}
