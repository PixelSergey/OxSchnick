use std::{collections::HashMap, sync::Arc};

use chrono::Local;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl, pooled_connection::bb8::PooledConnection};
use log::{error};
use serde::Serialize;
use serde_json::json;
use tokio::sync::{broadcast, mpsc, oneshot};

use crate::error::{Result, Error};

const GRAPHS_CHANNEL_BUFFER: usize = 128usize;
const GRAPHS_UPDATE_INTERVAL: i64 = 10i64;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum GraphUpdate {
    Schnick { a: i32, b: i32 },
    UserCreated { id: i32, parent: i32, name: String },
    UserRenamed { id: i32, name: String },
}

#[derive(Debug)]
pub enum GraphRequest {
    Update { update: GraphUpdate },
    GetCache { callback: oneshot::Sender<Arc<String>> },
    GetEvents { callback: oneshot::Sender<(Arc<String>, broadcast::Receiver<Arc<String>>)> },
    Tick
}

#[derive(Debug)]
pub struct Graphs {
    users: HashMap<i32, (i32, String)>,
    schnicks: Vec<(i32, i32)>,
    cache: Arc<String>,
    updates: Vec<GraphUpdate>,
    update_cache: Arc<String>,
    sender: mpsc::Sender<GraphRequest>,
    receiver: mpsc::Receiver<GraphRequest>,
    update: broadcast::Sender<Arc<String>>,
    cache_time: i64
}

impl Graphs {
    pub async fn with_connection(
        connection: &mut PooledConnection<'_, AsyncPgConnection>,
    ) -> anyhow::Result<Self> {
        use crate::schema::{schnicks, users};
        let persistent_users = users::table
            .select((users::id, users::parent, users::username))
            .load::<(i32, i32, String)>(connection)
            .await?
            .into_iter().map(|(id, parent, name)| (id, (parent, name))).collect();
        let persistent_schnicks = schnicks::table
            .select((schnicks::winner, schnicks::loser))
            .load::<(i32, i32)>(connection)
            .await?;
        let persistent_cache = Arc::new(Self::build_cache(&persistent_users, &persistent_schnicks));
        let (tx, rx) = mpsc::channel(GRAPHS_CHANNEL_BUFFER);
        Ok(
            Self {
                users: persistent_users,
                schnicks: persistent_schnicks,
                cache: persistent_cache,
                updates: vec![],
                update_cache: Arc::new("[]".to_string()),
                sender: tx,
                receiver: rx,
                update: broadcast::Sender::new(GRAPHS_CHANNEL_BUFFER),
                cache_time: Local::now().timestamp(),
            }
        )
    }

    fn build_cache(users: &HashMap<i32, (i32, String)>, schnicks: &Vec<(i32, i32)>) -> String {
        let value = json!({
            "users": users.iter().map(|(id, (parent, name))| (id, parent, name)).collect::<Vec<(&i32, &i32, &String)>>(),
            "schnicks": schnicks
        });
        value.to_string()
    }

    fn handle_update(&mut self, update: GraphUpdate) {
        match update {
            GraphUpdate::UserCreated { id, parent, name } => {self.users.insert(id, (parent, name));},
            GraphUpdate::Schnick { a, b } => {self.schnicks.push((a, b));},
            GraphUpdate::UserRenamed { id, name } => {
                if let Some((_, old_name)) = self.users.get_mut(&id) {
                    *old_name = name;
                }
            }
        };
    }

    pub async fn worker(mut self) {
        while let Some(request) = self.receiver.recv().await {
            match request {
                GraphRequest::Update { update } => {
                    self.updates.push(update.clone());
                    if let Ok(s) = serde_json::to_string(&self.updates) {
                        self.update_cache = Arc::new(s);
                    } else {
                        error!(target: "graphs::worker", "error building update cache");
                        continue;
                    };
                    if let Err(e) = self.update.send(Arc::new(json!([update]).to_string())) {
                        error!(target: "graphs::worker", "dead channel: {e:?}");
                    };
                }
                GraphRequest::GetCache { callback } => {
                    if let Err(e) = callback.send(Arc::clone(&self.cache)) {
                        error!(target: "graphs::worker", "dead channel: {e:?}");
                    }
                }
                GraphRequest::GetEvents { callback } => {
                    if let Err(_) = callback.send((Arc::clone(&self.update_cache), self.update.subscribe())) {
                        error!(target: "graphs::worker", "dead channel");
                    }
                },
                GraphRequest::Tick => {}
            }
            let now = Local::now().timestamp();
            if Local::now().timestamp() - self.cache_time >= GRAPHS_UPDATE_INTERVAL {
                let updates = self.updates.drain(..).collect::<Vec<GraphUpdate>>();
                for update in updates.into_iter() {
                    self.handle_update(update);
                }
                self.update_cache = Arc::new("[]".to_string());
                self.cache = Arc::new(Self::build_cache(&self.users, &self.schnicks));
                self.cache_time = now;
            }
        }
    }

    pub async fn send_update(update: GraphUpdate, sender: &mpsc::Sender<GraphRequest>) {
        if let Err(e) = sender.send(GraphRequest::Update { update }).await {
            error!(target: "graphs::send_update", "dead channel: {e:?}");
        };
    }

    pub async fn request_cache(
        sender: &mpsc::Sender<GraphRequest>
    ) -> Result<Arc<String>> {
        let (tx, rx) = oneshot::channel();
        sender
            .send(GraphRequest::GetCache { callback: tx })
            .await
            .map_err(|e| {
                error!(target: "auth::request", "dead channel: {:?}", e);
                Error::InternalServerError
            })?;
        rx.await.map_err(|e| {
            error!(target: "auth::request", "dead channel: {:?}", e);
            Error::InternalServerError
        })
    }

    pub async fn request_events(
        sender: &mpsc::Sender<GraphRequest>
    ) -> Result<(Arc<String>, broadcast::Receiver<Arc<String>>)> {
        let (tx, rx) = oneshot::channel();
        sender
            .send(GraphRequest::GetEvents { callback: tx })
            .await
            .map_err(|e| {
                error!(target: "auth::request", "dead channel: {:?}", e);
                Error::InternalServerError
            })?;
        rx.await.map_err(|e| {
            error!(target: "auth::request", "dead channel: {:?}", e);
            Error::InternalServerError
        })
    }

    pub fn sender(&self) -> mpsc::Sender<GraphRequest> {
        self.sender.clone()
    }
}
