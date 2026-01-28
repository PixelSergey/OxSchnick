use std::convert::Infallible;

use askama::Template;
use axum::{
    Form, extract,
    response::{Html, IntoResponse, Redirect, Sse, sse::Event},
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use futures::FutureExt;

use crate::{
    auth::User,
    error::{Error, Result},
    schnicks::{Interaction, Outcome, SchnickOutcomeReceiver, Schnicker},
    state::State,
};

pub async fn schnick_abort(
    extract::State(state): extract::State<State>,
    User(id): User,
) -> Result<impl IntoResponse> {
    Schnicker::request_abort_schnick(id, &state.schnicker).await?;
    Ok(Redirect::to("../home?banner=aborted"))
}

pub async fn schnick_submit(
    extract::State(state): extract::State<State>,
    User(id): User,
    Form(interaction): Form<Interaction>,
) -> Result<impl IntoResponse> {
    match Schnicker::request_handle_interaction(id, interaction, &state.schnicker).await? {
        Some(Outcome::Concluded) => {
            // Check if user is a new invited user (hasn't completed setup)
            use crate::schema::users;
            let user_college: Option<Option<i32>> = users::table
                .select(users::college)
                .find(id)
                .first::<Option<i32>>(
                    &mut state
                        .pool
                        .get()
                        .await
                        .map_err(|_| Error::InternalServerError)?,
                )
                .await
                .ok();
            
            if user_college.flatten().is_none() {
                // User hasn't set a college, redirect to setup
                Ok(Redirect::to("../setup").into_response())
            } else {
                // User has completed setup, redirect to home
                Ok(Redirect::to("home?banner=concluded").into_response())
            }
        },
        Some(Outcome::Retry) => Ok(Redirect::to("schnick?banner=retry").into_response()),
        Some(Outcome::Aborted) => Ok(Redirect::to("home?banner=aborted").into_response()),
        None => Ok(Html(
            WaitingTemplate
                .render()
                .map_err(|_| Error::InternalServerError)?,
        )
        .into_response()),
    }
}

pub async fn schnick_sse(
    extract::State(state): extract::State<State>,
    User(id): User,
    SchnickOutcomeReceiver(mut receiver): SchnickOutcomeReceiver,
) -> impl IntoResponse {
    // Check if user has completed setup
    use crate::schema::users;
    let has_college = if let Ok(mut conn) = state.pool.get().await {
        users::table
            .select(users::college)
            .find(id)
            .first::<Option<i32>>(&mut conn)
            .await
            .ok()
            .flatten()
            .is_some()
    } else {
        true // Fail safe - if we can't check, assume they've completed setup
    };

    let stream = (async move {
        let _ = receiver.changed().await;
        let outcome = *receiver.borrow();
        let redirect = match outcome {
            Outcome::Concluded => {
                if has_college {
                    "home?banner=concluded"
                } else {
                    "../setup"
                }
            },
            Outcome::Retry => "schnick?banner=retry",
            Outcome::Aborted => "home?banner=aborted"
        };
        Ok::<Event, Infallible>(Event::default().data(redirect))
    })
    .into_stream();
    Sse::new(stream)
}

#[derive(Template)]
#[template(path = "schnick.html")]
struct SchnickTemplate;

#[derive(Template)]
#[template(path = "waiting.html")]
struct WaitingTemplate;

pub async fn schnick(
    extract::State(state): extract::State<State>,
    User(id): User,
) -> Result<impl IntoResponse> {
    let active = Schnicker::request_in_schnick(id, &state.schnicker).await?;
    if active {
        Ok(Html(
            SchnickTemplate
                .render()
                .map_err(|_| Error::InternalServerError)?,
        ))
    } else {
        Ok(Html(
            WaitingTemplate
                .render()
                .map_err(|_| Error::InternalServerError)?,
        ))
    }
}
