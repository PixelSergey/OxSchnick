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
    // TODO: think about what to do if no invite
    if let Some(mut receiver) = app.inviter.receiver(id).await {
        Ok(Sse::new(
            try_stream! {
                trace!(target: "home::home_events", "starting home event handler");
                let _ = receiver.changed().await;
                yield Event::default().event("redirect").data("location.href = 'schnick';")
            }
        ))
    } else {
        trace!(target: "home::home_events", "no invite receiver found");
        Err(StatusCode::NOT_FOUND)
    }
}

/// The `/` route.
pub async fn home(State(app): State<App>, cookies: CookieJar) -> Result<Response, StatusCode> {
    debug!(target: "home::home", "cookies={cookies:?}");
    let id = app.authenticate(&cookies).await?;
    if app.active_schnick(id).await.is_ok() {
        return Ok(Redirect::temporary("schnick").into_response());
    }
    Ok(Html(
        Home {
            invite: app.inviter.get(id).await.url(&app.base)?,
        }
        .render()
        .map_err(|_| StatusCode::NOT_FOUND)?,
    ).into_response())
}
