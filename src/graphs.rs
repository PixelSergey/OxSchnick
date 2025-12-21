use std::sync::Arc;

use axum::response::sse::Event;
use chrono::Local;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl, pooled_connection::bb8::PooledConnection};
use log::{error, trace};
use tokio::sync::{RwLock, broadcast, mpsc};

const GRAPHS_CHANNEL_BUFFER: usize = 128usize;
const GRAPHS_UPDATE_INTERVAL: i64 = 10i64;

#[derive(Debug)]
pub struct Graph {
    persistent_cache: Vec<(i32, i32)>,
    rolling_cache: Vec<(i32, i32)>,
    graph_cache: Arc<RwLock<String>>,
    _sender: mpsc::Sender<(i32, i32)>,
    receiver: mpsc::Receiver<(i32, i32)>,
    update: broadcast::Sender<Arc<Event>>,
    timestamp: i64,
}

impl Graph {
    pub async fn with_connection(
        connection: &mut PooledConnection<'_, AsyncPgConnection>,
    ) -> anyhow::Result<(Self, mpsc::Sender<(i32, i32)>)> {
        use crate::schema::schnicks;
        let persistent_cache = schnicks::table
            .select((schnicks::winner, schnicks::loser))
            .load::<(i32, i32)>(connection)
            .await?;
        let (tx, rx) = mpsc::channel(GRAPHS_CHANNEL_BUFFER);
        let graph_cache = Arc::new(RwLock::new(serde_json::to_string(&persistent_cache)?));
        Ok((
            Self {
                persistent_cache: persistent_cache,
                rolling_cache: vec![],
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
        while let Some(schnick) = self.receiver.recv().await {
            self.rolling_cache.push(schnick);
            trace!(target: "graphs::worker", "got schnick {schnick:?}");
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
                self.persistent_cache.append(&mut self.rolling_cache);
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
