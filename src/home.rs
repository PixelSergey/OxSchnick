use std::{convert::Infallible};

use askama::Template;
use async_stream::try_stream;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response, Sse, sse::Event},
};
use axum_extra::extract::CookieJar;
use futures::Stream;
use log::{debug, trace};

use crate::app::App;

#[derive(Template)]
#[template(path = "index.html")]
struct Home {
    pub invite: String,
}

pub async fn home_events(
    State(app): State<App>,
    cookies: CookieJar,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let id = app.authenticate(&cookies).await?;
    debug!(target: "app::home::home_events", "subscribed id={id:?}");
    let mut receiver = app.sessions.receiver(id).await?;
    if app.sessions.active_schnick(id).await.is_ok() {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(Sse::new(
        try_stream! {
            trace!(target: "home::home_events", "starting home event handler");
            let _ = receiver.recv().await;
            yield Event::default().event("redirect").data("location.href = 'schnick';")
        }
    ))
}

/// The `/` route.
pub async fn home(State(app): State<App>, cookies: CookieJar) -> Result<Response, StatusCode> {
    debug!(target: "home::home", "cookies={cookies:?}");
    let id = app.authenticate(&cookies).await?;
    if app.sessions.active_schnick(id).await.is_ok() {
        return Ok(Redirect::temporary("schnick").into_response());
    }
    Ok(Html(
        Home {
            invite: app.sessions.get_invite(id).await?.url(&app.base)?,
        }
        .render()
        .map_err(|_| StatusCode::NOT_FOUND)?,
    ).into_response())
}
