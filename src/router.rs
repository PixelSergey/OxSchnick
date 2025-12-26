use std::sync::Arc;

use axum::{
    Router, extract::{self, Request}, middleware::{Next, from_fn_with_state}, response::{IntoResponse, Redirect}, routing::{get, post}
};
use diesel_async::{AsyncPgConnection, pooled_connection::bb8::Pool};
use tokio::sync::RwLock;
use url::Url;

use crate::{
    auth::{Authenticator, User}, error::Error, graphs::Graphs, metrics::Metrics, routes::{
        about, assets, graphs, graphs_cache, graphs_global, graphs_graph, graphs_sse, graphs_tree, home, home_invite, home_sse, imprint, index, invite, invite_accept, metrics, metrics_num_invites, metrics_num_schnicks, metrics_score, metrics_streak, recovery, schnick, schnick_abort, schnick_sse, schnick_submit, settings, settings_dect, settings_username
    }, schnicks::Schnicker, state::State
};

pub async fn redirect_if_in_schnick(
    extract::State(state): extract::State<State>,
    User(id): User,
    request: Request,
    next: Next
) -> impl IntoResponse {
    if let Ok(_) = Schnicker::request_in_schnick(id, &state.schnicker).await {
        Redirect::to("/schnick").into_response()
    } else {
        next.run(request).await
    }
}

pub async fn router(
    base_url: Url,
    pool: Pool<AsyncPgConnection>,
) -> anyhow::Result<(Router, Authenticator, Schnicker, Graphs)> {
    let graphs_o = Graphs::with_connection(&mut pool.get().await?).await?;
    let authenticator = Authenticator::with_connection_and_graphs(
        pool.dedicated_connection().await?,
        graphs_o.sender(),
    );
    let mut connection = pool.dedicated_connection().await?;
    let metrics_o = Arc::new(RwLock::new(Metrics::new(&mut connection).await?));
    let schnicker =
        Schnicker::with_connection_graphs_metrics_and_auth(connection, graphs_o.sender(), Arc::clone(&metrics_o), authenticator.sender());
    let state = State {
        base_url,
        pool,
        authenticator: authenticator.sender(),
        schnicker: schnicker.sender(),
        graphs: graphs_o.sender(),
        metrics: metrics_o
    };
    let authenticated_with_registration = Router::new()
        .route("/invite/accept", get(invite_accept))
        .route_layer(from_fn_with_state(
            state.clone(),
            Authenticator::layer_with_registration,
        ))
        .with_state(state.clone());
    let unauthenticated = Router::new()
        .route("/", get(index))
        .route("/about", get(about))
        .route("/imprint", get(imprint))
        .route("/invite", get(invite))
        .route("/assets/{file}", get(assets))
        .route("/graphs/cache", get(graphs_cache))
        .route("/graphs/sse", get(graphs_sse))
        .route("/graphs/global", get(graphs_global))
        .route("/recovery", get(recovery))
        .with_state(state.clone());
    let authenticated = Router::new()
        .route("/home", get(home))
        .route("/settings", get(settings))
        .route("/graphs", get(graphs))
        .route("/graphs/graph", get(graphs_graph))
        .route("/graphs/tree", get(graphs_tree))
        .route("/metrics", get(metrics))
        .route("/metrics/score", get(metrics_score))
        .route("/metrics/num_schnicks", get(metrics_num_schnicks))
        .route("/metrics/streak", get(metrics_streak))
        .route("/metrics/num_invites", get(metrics_num_invites))
        .route_layer(from_fn_with_state(state.clone(), redirect_if_in_schnick))
        .route("/schnick", get(schnick))
        .route("/home/sse", get(home_sse))
        .route("/home/invite", get(home_invite))
        .route("/schnick", post(schnick_submit))
        .route("/schnick/sse", get(schnick_sse))
        .route("/schnick/abort", get(schnick_abort))
        .route("/settings/dect", post(settings_dect))
        .route("/settings/username", post(settings_username))
        .route_layer(from_fn_with_state(state.clone(), Authenticator::layer))
        .with_state(state.clone());
    let router = Router::new()
        .merge(authenticated_with_registration)
        .merge(authenticated)
        .merge(unauthenticated)
        .fallback(get(async || Error::NotFound));
    Ok((router, authenticator, schnicker, graphs_o))
}
