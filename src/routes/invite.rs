use axum::{
    extract::{self, Query},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};

use crate::{
    auth::{Authenticated, Authenticator, Invite},
    schnicks::Schnicker,
    state::State,
};

pub async fn invite(
    extract::State(state): extract::State<State>,
    Authenticated { id, .. }: Authenticated,
    Query(invite): Query<Invite>,
) -> Result<impl IntoResponse, StatusCode> {
    Schnicker::request_start_schnick(id, invite.id, &state.schnicker).await?;
    Authenticator::request_renew_invite(id, &state.authenticator).await?;
    Ok(Redirect::temporary("schnick"))
}
