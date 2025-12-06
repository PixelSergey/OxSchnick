use askama::Template;
use axum::{extract::State, http::StatusCode, response::Html};
use axum_extra::extract::CookieJar;
use log::debug;

use crate::app::App;

#[derive(Template)]
#[template(path="index.html")]
struct Home {
    pub invite: String
}

/// The `/` route.
pub async fn home(
    State(app): State<App>,
    cookies: CookieJar
) -> Result<Html<String>, StatusCode> {
    debug!(target: "home::home", "cookies={cookies:?}");
    let id = app.authenticate(&cookies).await?;
    Ok(Html(Home {
        invite: app.get_invite(id).await?.url(&app.base)?
    }.render().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?))
}