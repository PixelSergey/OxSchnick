use askama::Template;
use axum::{http::StatusCode, response::{Html, IntoResponse}};

#[derive(Debug, Clone, Copy)]
pub enum Error {
    NoLogin,
    InvalidLogin,
    InvalidInvite,
    InternalServerError,
    CannotSchnickOneself,
    CannotSchnickTwice,
    AlreadySchnicking,
    NotFound,
    NotInSchnick,
    AlreadySubmitted,
    InvalidSettings,
}

#[derive(Template)]
#[template(path="error.html")]
struct ErrorTemplate<'a> {
    message: &'a str,
    redirect: &'a str
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        let (code, message, redirect) = match self {
            Self::NoLogin => (
                StatusCode::FORBIDDEN,
                "This page is only accessible with a user account. You have to be invited to obtain a user account.",
                "/"
            ),
            Self::InvalidLogin => (
                StatusCode::FORBIDDEN,
                "The login token you saved is invalid. Try restoring your cookies or clear them and get invited again.",
                "/"
            ),
            Self::InvalidInvite => (
                StatusCode::FORBIDDEN,
                "The invite you tried to use is invalid. Ask the person who invited you to show you their invite again.",
                "/"
            ),
            Self::InternalServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An internal error occured while loading this page. Try again later.",
                "/"
            ),
            Self::CannotSchnickOneself => (
                StatusCode::CONFLICT,
                "You cannot start a schnick with yourself. Try inviting another person.",
                "/"
            ),
            Self::CannotSchnickTwice => (
                StatusCode::CONFLICT,
                "You cannot schnick with the same person twice. Try schnicking with another person.",
                "/"
            ),
            Self::AlreadySchnicking => (
                StatusCode::CONFLICT,
                "You are already currently schnicking. Finish or abort your current schnick before starting another one.",
                "/schnick"
            ),
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                "The page you tried to open does not exist.",
                "/"
            ),
            Self::NotInSchnick => (
                StatusCode::NOT_FOUND,
                "You are not currently in a schnick. This page is only accessible while schnicking.",
                "/"
            ),
            Self::AlreadySubmitted => (
                StatusCode::CONFLICT,
                "You have already submitted a result for this schnick. Wait for the other person to submit their result.",
                "/schnick"
            ),
            Self::InvalidSettings => (
                StatusCode::BAD_REQUEST,
                "The settings you tried to submit are not valid. Try again.",
                "/settings"
            )
        };
        let body = match (ErrorTemplate {
            message: message,
            redirect
        }).render() {
            Ok(out) => out,
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response()
        };
        (code, Html(body)).into_response()
    }
}

pub type Result<T> = core::result::Result<T, Error>;