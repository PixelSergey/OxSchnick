use std::convert::Infallible;

use askama::Template;
use axum::{extract, http::StatusCode, response::{Html, IntoResponse, Sse, sse::Event}};
use futures::FutureExt;
use url::Url;
use uuid::Uuid;

use crate::{
    auth::AuthenticatorEntry, schnicks::Weapon, state::State, users::{Stats, User}
};

#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate<'a> {
    pub user: &'a User,
    pub stats: &'a Stats,
    pub invite: &'a str,
}

pub async fn home_sse(
    AuthenticatorEntry { channel, ..}: AuthenticatorEntry
) -> impl IntoResponse {
    let mut receiver = channel.subscribe();
    let stream = (async move {
        let _ = receiver.changed().await;
        Ok::<Event, Infallible>(Event::default().data("schnick"))
    })
    .into_stream();
    Sse::new(stream)
}

fn invite_url(base: &Url, id: i32, token: &Uuid) -> Option<Url>  {
    let mut url = base.join("invite").ok()?;
    url.set_query(Some(&format!("id={id}&token={token}")));
    Some(url)
}

pub async fn home(
    extract::State(state): extract::State<State>,
    (user, stats): (User, Stats),
    AuthenticatorEntry { invite, ..}: AuthenticatorEntry,
) -> Result<impl IntoResponse, StatusCode> {
    Ok(Html(HomeTemplate {
        user: &user,
        stats: &stats,
        invite: invite_url(&state.base_url, user.id, &invite).ok_or(StatusCode::INTERNAL_SERVER_ERROR)?.as_str(),
    }
    .render()
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?))
}
