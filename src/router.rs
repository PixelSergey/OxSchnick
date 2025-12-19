use axum::{
    Router,
    middleware::from_fn_with_state,
    routing::{get, post},
};
use diesel_async::{AsyncPgConnection, pooled_connection::bb8::Pool};
use url::Url;

use crate::{
    auth::Authenticator,
    routes::{assets, home, home_sse, invite, schnick, schnick_sse, schnick_submit},
    schnicks::Schnicker,
    state::State,
};

pub async fn router(
    base_url: Url,
    pool: Pool<AsyncPgConnection>,
) -> anyhow::Result<(Router, Authenticator, Schnicker)> {
    let authenticator = Authenticator::with_connection(pool.dedicated_connection().await?);
    let schnicker = Schnicker::with_connection(pool.dedicated_connection().await?);
    let state = State {
        base_url,
        pool,
        authenticator: authenticator.sender(),
        schnicker: schnicker.sender(),
    };
    let authenticated_with_registration = Router::new()
        .route("/invite", get(invite))
        .route_layer(from_fn_with_state(
            state.clone(),
            Authenticator::layer_with_registration,
        ))
        .with_state(state.clone());
    let authenticated = Router::new()
        .route("/home", get(home))
        .route("/home/sse", get(home_sse))
        .route("/schnick", get(schnick))
        .route("/schnick", post(schnick_submit))
        .route("/schnick/sse", get(schnick_sse))
        .route_layer(from_fn_with_state(state.clone(), Authenticator::layer))
        .with_state(state.clone());
    let router = Router::new()
        .route("/assets/{file}", get(assets))
        .merge(authenticated_with_registration)
        .merge(authenticated);
    Ok((router, authenticator, schnicker))
}
