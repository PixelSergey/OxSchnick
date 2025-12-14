use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
};
use axum_extra::extract::CookieJar;
use log::debug;

use crate::app::App;

/// The `/` route.
pub async fn home(
    State(app): State<App>,
    cookies: CookieJar,
) -> Result<impl IntoResponse, StatusCode> {
    debug!(target: "home::home", "cookies={cookies:?}");
    let _ = app.authenticate(&cookies).await?;
    Ok(Html(include_str!("../templates/index.html")))
}
