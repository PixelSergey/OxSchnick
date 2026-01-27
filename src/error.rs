use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
};

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
    InvalidCollege,
    DuplicateUsername,
    NotActive,
}

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorTemplate<'a> {
    message: &'a str,
    redirect: &'a str,
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        let (code, message, redirect) = match self {
            Self::NoLogin => (
                StatusCode::FORBIDDEN,
                "This page is only accessible with a user account, but you can call #5000 to find a partner to schnick with to get invited. <br><br> If you already have an account in a different browser, you can go to the settings to copy your account to this new one.",
                "/",
            ),
            Self::InvalidLogin => (
                StatusCode::FORBIDDEN,
                "Your login token is invalid. Try restoring your cookies or clear them and get invited again.",
                "/",
            ),
            Self::InvalidInvite => (
                StatusCode::FORBIDDEN,
                "The invite you tried to use is invalid. Ask the person who invited you to show you their invite again.",
                "/",
            ),
            Self::InternalServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An internal error occured while loading this page, sorry! Try again later.",
                "/",
            ),
            Self::CannotSchnickOneself => (
                StatusCode::CONFLICT,
                "You cannot start a schnick with yourself. Nice try though!",
                "/",
            ),
            Self::CannotSchnickTwice => (
                StatusCode::CONFLICT,
                "You cannot schnick with the same person twice. Have you tried meeting new people?",
                "/",
            ),
            Self::AlreadySchnicking => (
                StatusCode::CONFLICT,
                "You are already in a schnick. Finish or abort your current schnick before starting another one.",
                "/schnick",
            ),
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                "The page you tried to open does not exist. We are happy to receive your bug report via our hotline (#5000).",
                "/",
            ),
            Self::NotInSchnick => (
                StatusCode::NOT_FOUND,
                "You are not currently in a schnick. Maybe your opponent was scared of you and aborted the schnick?",
                "/",
            ),
            Self::AlreadySubmitted => (
                StatusCode::CONFLICT,
                "You have already submitted a result for this schnick. Please wait for the other person to submit their result.",
                "/schnick",
            ),
            Self::InvalidSettings => (
                StatusCode::BAD_REQUEST,
                "The settings you tried to submit are not valid. Try again.",
                "/settings",
            ),
            Self::InvalidCollege => (
                StatusCode::BAD_REQUEST,
                "The college you tried to submit is not valid.",
                "/settings",
            ),
            Self::DuplicateUsername => (
                StatusCode::BAD_REQUEST,
                "The username you tried to set is already taken, sorry! You can try another one.",
                "/settings",
            ),
            Self::NotActive => (
                StatusCode::BAD_REQUEST,
                "You need to finish a schnick initiated by another person before you can invite new users.",
                "/"
            )
        };
        let body = match (ErrorTemplate {
            message: message,
            redirect,
        })
        .render()
        {
            Ok(out) => out,
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };
        (code, Html(body)).into_response()
    }
}

pub type Result<T> = core::result::Result<T, Error>;
