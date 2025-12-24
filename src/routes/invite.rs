use askama::Template;
use axum::{
    extract::{self, Query},
    response::{Html, IntoResponse, Redirect},
};
use url::Url;
use uuid::Uuid;

use crate::{
    auth::{Authenticator, Invite, User},
    error::{Error, Result},
    schnicks::Schnicker,
    state::State,
};

#[derive(Debug, Template)]
#[template(path = "accept_invite.html")]
struct InviteTemplate<'a> {
    base_url: &'a Url,
    id: i32,
    token: &'a Uuid,
}

pub async fn invite(
    extract::State(state): extract::State<State>,
    Query(invite): Query<Invite>,
) -> Result<impl IntoResponse> {
    Ok(Html(
        InviteTemplate {
            base_url: &state.base_url,
            id: invite.id,
            token: &invite.token,
        }
        .render()
        .map_err(|_| Error::InternalServerError),
    ))
}

pub async fn invite_accept(
    extract::State(state): extract::State<State>,
    User(id): User,
    Query(invite): Query<Invite>,
) -> Result<impl IntoResponse> {
    Schnicker::request_start_schnick(id, invite.id, &state.schnicker).await?;
    Authenticator::request_renew_invite(invite.id, &state.authenticator).await?;
    Ok(Redirect::to("../schnick"))
}
