use std::convert::Infallible;

use askama::Template;
use axum::{
    Json, extract,
    response::{Html, IntoResponse, Redirect, Sse, sse::Event},
};
use futures::{StreamExt, stream};
use tokio_stream::wrappers::BroadcastStream;

use crate::{
    auth::User, error::{Error, Result}, graphs::Graphs, state::State
};

pub async fn graphs_cache(extract::State(state): extract::State<State>) -> Result<impl IntoResponse> {
    let cache = Graphs::request_cache(&state.graphs).await?;
    Ok(Json(cache.to_string()))
}

pub async fn graphs_sse(extract::State(state): extract::State<State>) -> Result<impl IntoResponse> {
    let (cache, receiver) = Graphs::request_events(&state.graphs).await?;
    let initial = stream::once(async move {
        Ok::<Event, Infallible>(Event::default().data(cache.to_string()))
    });
    let stream = BroadcastStream::new(receiver).map(|update| {
        if let Ok(update) = update {
            Ok::<Event, Infallible>(Event::default().data(update.to_string()))
        } else {
            Ok::<Event, Infallible>(Event::default())
        }
    });
    Ok(Sse::new(initial.chain(stream)))
}

#[derive(Template)]
#[template(path = "tree.html")]
struct TreeTemplate {
    pub id: i32
}

pub async fn graphs_tree(
    User(id): User,
) -> Result<impl IntoResponse> {
    Ok(Html(
        TreeTemplate {
            id,
        }
        .render()
        .map_err(|_| Error::InternalServerError)?,
    ))
}

#[derive(Template)]
#[template(path = "graph.html")]
struct GraphTemplate {
    pub id: i32,
}

pub async fn graphs_graph(
    User(id): User,
) -> Result<impl IntoResponse> {
    Ok(Html(
        GraphTemplate {
            id,
        }
        .render()
        .map_err(|_| Error::InternalServerError)?,
    ))
}

pub async fn graphs() -> impl IntoResponse {
    Redirect::to("graphs/graph")
}

#[derive(Template)]
#[template(path = "global.html")]
struct GlobalTemplate;

pub async fn graphs_global() -> Result<impl IntoResponse> {
    Ok(Html(
        GlobalTemplate
            .render()
            .map_err(|_| Error::InternalServerError)?,
    ))
}
