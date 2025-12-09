use std::{convert::Infallible, sync::Arc};

use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response, Sse, sse::Event},
};
use axum_extra::extract::CookieJar;
use futures::{FutureExt, Stream};
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
    Ok(Sse::new(
        App::redirect(id, Arc::clone(&app.redirects))
            .map(move |_| {
                trace!(target: "home::home_events", "sending for id={id:?}");
                Ok(Event::default().event("redirect").data("location.href = 'schnick';"))
            })
            .into_stream(),
    ))
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
            invite: app.get_invite(id).await?.url(&app.base)?,
        }
        .render()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ).into_response())
}
