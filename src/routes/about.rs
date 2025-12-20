use askama::Template;
use axum::{http::StatusCode, response::{Html, IntoResponse}};

#[derive(Template)]
#[template(path="about_us.html")]
struct AboutTemplate;

#[derive(Template)]
#[template(path="imprint.html")]
struct ImprintTemplate;

pub async fn about() -> Result<impl IntoResponse, StatusCode> {
    Ok(Html(AboutTemplate.render().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?))
}

pub async fn imprint() -> Result<impl IntoResponse, StatusCode> {
    Ok(Html(ImprintTemplate.render().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?))
}