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
#[template(path = "metrics_streak.html")]
struct MetricsStreakTemplate<'a> {
    winning_streaks: &'a Vec<(MetricsUser, i32)>,
    losing_streaks: &'a Vec<(MetricsUser, i32)>
}

pub async fn metrics_streak(
    extract::State(state): extract::State<State>
) -> Result<impl IntoResponse> {
    let winning_streaks = state.metrics.read().await.winning_streaks.clone();
    let losing_streaks = state.metrics.read().await.losing_streaks.clone();
    Ok(Html(MetricsStreakTemplate {
        winning_streaks: &winning_streaks,
        losing_streaks: &losing_streaks
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
    Redirect::to("metrics/score")
}
