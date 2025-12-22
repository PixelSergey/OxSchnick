use askama::Template;
use axum::{
    response::{Html, IntoResponse},
};

use crate::error::{Error, Result};

#[derive(Template)]
#[template(path = "about_us.html")]
struct AboutTemplate;

#[derive(Template)]
#[template(path = "imprint.html")]
struct ImprintTemplate;

pub async fn about() -> Result<impl IntoResponse> {
    Ok(Html(
        AboutTemplate
            .render()
            .map_err(|_| Error::InternalServerError)?,
    ))
}

pub async fn imprint() -> Result<impl IntoResponse> {
    Ok(Html(
        ImprintTemplate
            .render()
            .map_err(|_| Error::InternalServerError)?,
    ))
}
