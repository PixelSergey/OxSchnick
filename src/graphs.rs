use std::sync::Arc;

use axum::response::sse::Event;
use chrono::Local;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl, pooled_connection::bb8::PooledConnection};
use log::{error, trace};
use serde::Serialize;
use tokio::sync::{RwLock, broadcast, mpsc};

const GRAPHS_CHANNEL_BUFFER: usize = 128usize;
const GRAPHS_UPDATE_INTERVAL: i64 = 10i64;

#[derive(Debug, Clone, Default, Serialize)]
pub struct GraphData {
    pub vertices: Vec<(i32, i32, String)>,
    pub edges: Vec<(i32, i32)>,
}

#[derive(Debug, Clone)]
pub enum GraphUpdate {
    Schnick((i32, i32)),
    User((i32, i32, String)),
}

impl GraphData {
    pub fn merge_into(&mut self, other: &mut Self) {
        self.vertices.append(&mut other.vertices);
        self.edges.append(&mut other.edges);
    }

    pub fn add_schnick(&mut self, schnick: (i32, i32)) {
        self.edges.push(schnick);
    }

    pub fn add_user(&mut self, user: (i32, i32, String)) {
        self.vertices.push(user);
    }
}

#[derive(Debug)]
pub struct Graph {
    persistent_cache: GraphData,
    rolling_cache: GraphData,
    graph_cache: Arc<RwLock<String>>,
    _sender: mpsc::Sender<GraphUpdate>,
    receiver: mpsc::Receiver<GraphUpdate>,
    update: broadcast::Sender<Arc<Event>>,
    timestamp: i64,
}

impl Graph {
    pub async fn with_connection(
        connection: &mut PooledConnection<'_, AsyncPgConnection>,
    ) -> anyhow::Result<(Self, mpsc::Sender<GraphUpdate>)> {
        use crate::schema::{schnicks, users};
        let persistent_vertices = users::table
            .select((users::id, users::parent, users::username))
            .load::<(i32, i32, String)>(connection)
            .await?;
        let persistent_edges = schnicks::table
            .select((schnicks::winner, schnicks::loser))
            .load::<(i32, i32)>(connection)
            .await?;
        let persistent_cache = GraphData {
            vertices: persistent_vertices,
            edges: persistent_edges,
        };
        let (tx, rx) = mpsc::channel(GRAPHS_CHANNEL_BUFFER);
        let graph_cache = Arc::new(RwLock::new(serde_json::to_string(&persistent_cache)?));
        Ok((
            Self {
                persistent_cache,
                rolling_cache: Default::default(),
                graph_cache,
                _sender: tx.clone(),
                receiver: rx,
                update: broadcast::Sender::new(GRAPHS_CHANNEL_BUFFER),
                timestamp: Local::now().timestamp(),
            },
            tx,
        ))
    }

    pub async fn worker(mut self) {
        while let Some(update) = self.receiver.recv().await {
            match update {
                GraphUpdate::Schnick(schnick) => self.rolling_cache.add_schnick(schnick),
                GraphUpdate::User(user) => self.rolling_cache.add_user(user),
            };
            if Local::now().timestamp() - self.timestamp >= GRAPHS_UPDATE_INTERVAL {
                trace!(target: "graphs::worker", "exceeded timeout, dumping cache");
                let payload = match serde_json::to_string(&self.rolling_cache) {
                    Ok(s) => s,
                    Err(e) => {
                        error!(target: "graphs::worker", "error serializing graph update: {e:?}");
                        continue;
                    }
                };
                if let Err(_) = self.update.send(Arc::new(Event::default().data(payload))) {
                    error!(target: "graphs::worker", "dead channel");
                }
                self.persistent_cache.merge_into(&mut self.rolling_cache);
                let new = match serde_json::to_string(&self.persistent_cache) {
                    Ok(s) => s,
                    Err(e) => {
                        error!(target: "graphs::worker", "error serializing graph cache: {e:?}");
                        continue;
                    }
                };
                self.timestamp = Local::now().timestamp();
                let mut cache = self.graph_cache.write().await;
                cache.clear();
                cache.push_str(&new);
            }
        }
    }

    pub fn graph_cache(&self) -> Arc<RwLock<String>> {
        Arc::clone(&self.graph_cache)
    }

    pub fn update_receiver(&self) -> broadcast::Receiver<Arc<Event>> {
        self.update.subscribe()
    }
}
