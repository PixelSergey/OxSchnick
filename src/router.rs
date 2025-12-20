use axum::{
    Router,
    middleware::from_fn_with_state,
    routing::{get, post},
};
use diesel_async::{AsyncPgConnection, pooled_connection::bb8::Pool};
use url::Url;

use crate::{
    auth::Authenticator,
    routes::{about, assets, graphs, home, home_sse, imprint, index, invite, metrics, schnick, schnick_abort, schnick_sse, schnick_submit, settings, settings_submit},
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
    let unauthenticated = Router::<()>::new()
        .route("/", get(index))
        .route("/about", get(about))
        .route("/imprint", get(imprint))
        .route("/assets/{file}", get(assets));
    let authenticated = Router::new()
        .route("/home", get(home))
        .route("/home/sse", get(home_sse))
        .route("/schnick", get(schnick))
        .route("/schnick", post(schnick_submit))
        .route("/schnick/sse", get(schnick_sse))
        .route("/schnick/abort", get(schnick_abort))
        .route("/settings", get(settings))
        .route("/settings", post(settings_submit))
        .route("/graphs", get(graphs))
        .route("/metrics", get(metrics))
        .route_layer(from_fn_with_state(state.clone(), Authenticator::layer))
        .with_state(state.clone());
    let router = Router::new()
        .merge(authenticated_with_registration)
        .merge(authenticated)
        .merge(unauthenticated);
    Ok((router, authenticator, schnicker))
}
