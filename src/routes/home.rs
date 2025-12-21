use std::convert::Infallible;

use askama::Template;
use axum::{
    extract,
    http::StatusCode,
    response::{Html, IntoResponse, Sse, sse::Event},
};
use futures::FutureExt;
use qrcode::{QrCode, render::svg};
use url::Url;
use uuid::Uuid;

use crate::{
    auth::{AuthenticatorEntry, User},
    schnicks::Weapon,
    state::State,
    users::{Settings, Stats},
};

pub async fn home_sse(AuthenticatorEntry { channel, .. }: AuthenticatorEntry) -> impl IntoResponse {
    let mut receiver = channel.subscribe();
    let stream = (async move {
        let _ = receiver.changed().await;
        Ok::<Event, Infallible>(Event::default().data("schnick"))
    })
    .into_stream();
    Sse::new(stream)
}

fn invite_url(base: &Url, id: i32, token: &Uuid) -> Option<Url> {
    let mut url = base.join("invite").ok()?;
    url.set_query(Some(&format!("id={id}&token={token}")));
    Some(url)
}

#[derive(Template)]
#[template(path = "invite.html")]
struct HomeInviteTemplate<'a> {
    qrcode: &'a str,
}

pub async fn home_invite(
    extract::State(state): extract::State<State>,
    User(id): User,
    AuthenticatorEntry { invite, .. }: AuthenticatorEntry,
) -> Result<impl IntoResponse, StatusCode> {
    let invite_url =
        invite_url(&state.base_url, id, &invite).ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let qrcode = QrCode::new(invite_url.as_str()).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let svg = qrcode.render::<svg::Color>().build();
    Ok(Html(
        HomeInviteTemplate { qrcode: &svg }
            .render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate<'a> {
    pub user: &'a Settings,
    pub stats: &'a Stats,
    pub invite: &'a str,
}

pub async fn home(
    extract::State(state): extract::State<State>,
    (user, stats): (Settings, Stats),
    AuthenticatorEntry { invite, .. }: AuthenticatorEntry,
) -> Result<impl IntoResponse, StatusCode> {
    Ok(Html(
        HomeTemplate {
            user: &user,
            stats: &stats,
            invite: invite_url(&state.base_url, user.id, &invite)
                .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
                .as_str(),
        }
        .render()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}
