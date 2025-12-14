use std::convert::Infallible;

use async_stream::try_stream;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Sse, sse::Event},
};
use axum_extra::extract::CookieJar;
use futures::Stream;

use crate::app::App;

pub async fn body(app: &App, id: i32) -> Event {
    if let Ok(schnick) = app.sessions.active_schnick(id).await {
        if let Some((previous, _)) = *schnick.partial.lock().await {
            if previous == id {
                Event::default().data(include_str!("../templates/waiting.html"))
            } else {
                Event::default().data(include_str!("../templates/schnick.html"))
            }
        } else {
            Event::default().data(include_str!("../templates/schnick.html"))
        }
    } else {
        Event::default().data(include_str!("../templates/nav.html"))
    }
}

pub async fn events(
    State(app): State<App>,
    cookies: CookieJar,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let id = app.authenticate(&cookies).await?;
    let mut receiver = app.sessions.receiver(id).await?;
    Ok(Sse::new(try_stream! {
        yield body(&app, id).await;
        while let Ok(_) = receiver.recv().await {
            yield body(&app, id).await;
        }
    }))
}
