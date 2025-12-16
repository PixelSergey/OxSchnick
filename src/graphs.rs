use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
};
use axum_extra::extract::CookieJar;

use crate::app::App;

pub async fn graphs(
    State(app): State<App>,
    cookies: CookieJar,
) -> Result<impl IntoResponse, StatusCode> {
    let _ = app.authenticate(&cookies).await?;
    Ok(Html(include_str!("../templates/graphs.html")))
}