use std::convert::Infallible;

use askama::{Template};
use axum::{
    Form, extract,
    http::StatusCode,
    response::{Html, IntoResponse, Sse, sse::Event},
};
use futures::{FutureExt};

use crate::{
    auth::Authenticated,
    schnicks::{Interaction, SchnickOutcomeReceiver, Schnicker},
    state::State,
};

pub async fn schnick_submit(
    extract::State(state): extract::State<State>,
    Authenticated { id, .. }: Authenticated,
    Form(interaction): Form<Interaction>,
) -> Result<impl IntoResponse, StatusCode> {
    Schnicker::request_handle_interaction(id, interaction, &state.schnicker).await?;
    Ok(StatusCode::OK)
}

pub async fn schnick_sse(
    SchnickOutcomeReceiver(mut receiver): SchnickOutcomeReceiver,
) -> impl IntoResponse {
    let stream = (async move {
        let _ = receiver.changed().await;
        Ok::<Event, Infallible>(Event::default())
    })
    .into_stream();
    Sse::new(stream)
}

#[derive(Template)]
#[template(path="schnick.html")]
struct SchnickTemplate;

#[derive(Template)]
#[template(path="waiting.html")]
struct WaitingTemplate;

pub async fn schnick(
    extract::State(state): extract::State<State>,
    Authenticated { id, .. }: Authenticated,
) -> Result<impl IntoResponse, StatusCode> {
    let active = Schnicker::request_in_schnick(id, &state.schnicker).await?;
    if active {
        Ok(Html(SchnickTemplate.render().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?))
    } else {
        Ok(Html(WaitingTemplate.render().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?))
    }
}
