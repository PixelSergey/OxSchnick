use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
};

#[derive(Template)]
#[template(path="graphs.html")]
struct GraphsTemplate;

pub async fn graphs() -> Result<impl IntoResponse, StatusCode> {
    Ok(Html(GraphsTemplate.render().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?))
}
