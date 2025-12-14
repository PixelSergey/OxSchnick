use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
};
use axum_extra::extract::CookieJar;
use log::debug;

use crate::app::App;

#[derive(Template)]
#[template(path = "home.html")]
struct Home {
    pub invite: String,
}

/// The `/home` route.
pub async fn home(
    State(app): State<App>,
    cookies: CookieJar,
) -> Result<impl IntoResponse, StatusCode> {
    debug!(target: "home::home", "cookies={cookies:?}");
    let id = app.authenticate(&cookies).await?;
    let invite = app.sessions.get_invite(id).await?;
    let invite_url = invite.url(&app.base)?;
    Ok(Html(
        Home { invite: invite_url }
            .render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}
