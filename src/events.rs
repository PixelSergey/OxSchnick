use std::convert::Infallible;

use askama::Template;
use async_stream::try_stream;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Sse, sse::Event},
};
use axum_extra::extract::CookieJar;
use futures::Stream;

use crate::app::App;

#[derive(Template)]
#[template(path = "home.html")]
struct Home {
    pub invite: String,
}

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
        let invite = app.sessions.get_invite(id).await.unwrap();
        let url = invite.url(&app.base).unwrap();
        Event::default().data(&Home { invite: url}.render().unwrap())
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
