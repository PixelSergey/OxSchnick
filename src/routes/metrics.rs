use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
};

#[derive(Template)]
#[template(path="metrics.html")]
struct MetricsTemplate;

pub async fn metrics() -> Result<impl IntoResponse, StatusCode> {
    Ok(Html(MetricsTemplate.render().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?))
}
