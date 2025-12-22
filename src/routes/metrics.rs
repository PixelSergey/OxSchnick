use askama::Template;
use axum::{
    response::{Html, IntoResponse},
};

use crate::error::{Error, Result};

#[derive(Template)]
#[template(path = "metrics.html")]
struct MetricsTemplate;

pub async fn metrics() -> Result<impl IntoResponse> {
    Ok(Html(
        MetricsTemplate
            .render()
            .map_err(|_| Error::InternalServerError)?,
    ))
}
