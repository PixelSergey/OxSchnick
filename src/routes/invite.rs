use axum::{
    extract::{self, Query},
    response::{IntoResponse, Redirect},
};

use crate::{
    auth::{Authenticator, Invite, User}, error::Result, schnicks::Schnicker, state::State
};

pub async fn invite(
    extract::State(state): extract::State<State>,
    User(id): User,
    Query(invite): Query<Invite>,
) -> Result<impl IntoResponse> {
    Schnicker::request_start_schnick(id, invite.id, &state.schnicker).await?;
    Authenticator::request_renew_invite(invite.id, &state.authenticator).await?;
    Ok(Redirect::to("schnick"))
}
