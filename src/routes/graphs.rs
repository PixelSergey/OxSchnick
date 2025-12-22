use std::convert::Infallible;

use askama::Template;
use axum::{
    extract,
    response::{Html, IntoResponse, Redirect, Sse, sse::Event},
};
use futures::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use crate::{auth::User, error::{Error, Result}, state::State};

pub async fn graphs_graph_sse(extract::State(state): extract::State<State>) -> impl IntoResponse {
    let receiver = state.graph_updates.resubscribe();
    let stream = BroadcastStream::new(receiver).map(|update| {
        if let Ok(update) = update {
            Ok::<Event, Infallible>(Event::clone(&update))
        } else {
            Ok::<Event, Infallible>(Event::default())
        }
    });
    Sse::new(stream)
}

#[derive(Template)]
#[template(path = "graph.html")]
struct GraphTemplate<'a> {
    pub id: i32,
    pub cache: &'a str,
}

pub async fn graphs_graph(
    User(id): User,
    extract::State(state): extract::State<State>,
) -> Result<impl IntoResponse> {
    let cache = state.graph_cache.read().await;
    Ok(Html(
        GraphTemplate {
            id,
            cache: cache.as_str(),
        }
        .render()
        .map_err(|_| Error::InternalServerError)?,
    ))
}

pub async fn graphs() -> impl IntoResponse {
    Redirect::to("graphs/graph")
}
