use askama::Template;
use axum::{extract, response::{Html, IntoResponse, Redirect}};

use crate::{error::{Error, Result}, metrics::MetricsUser, state::State};

#[derive(Template)]
#[template(path = "metrics_score.html")]
struct MetricsScoreTemplate<'a> {
    data: &'a Vec<(MetricsUser, i32, i32, i32)>
}

pub async fn metrics_score(
    extract::State(state): extract::State<State>
) -> Result<impl IntoResponse> {
    let data = state.metrics.read().await.score.clone();
    Ok(Html(MetricsScoreTemplate {
        data: &data
    }.render().map_err(|_| Error::InternalServerError)?))
}

#[derive(Template)]
#[template(path = "metrics_num_schnicks.html")]
struct MetricsNumSchnicksTemplate<'a> {
    data: &'a Vec<(MetricsUser, i32)>
}

pub async fn metrics_num_schnicks(
    extract::State(state): extract::State<State>
) -> Result<impl IntoResponse> {
    let data = state.metrics.read().await.num_schnicks.clone();
    Ok(Html(MetricsNumSchnicksTemplate {
        data: &data
    }.render().map_err(|_| Error::InternalServerError)?))
}

#[derive(Template)]
#[template(path = "metrics_num_invites.html")]
struct MetricsNumInvitesTemplate<'a> {
    data: &'a Vec<(MetricsUser, i32)>
}

pub async fn metrics_num_invites(
    extract::State(state): extract::State<State>
) -> Result<impl IntoResponse> {
    let data = state.metrics.read().await.num_children.clone();
    Ok(Html(MetricsNumInvitesTemplate {
        data: &data
    }.render().map_err(|_| Error::InternalServerError)?))
}

pub async fn metrics() -> impl IntoResponse {
    Redirect::to("metrics/num_invites")
}
