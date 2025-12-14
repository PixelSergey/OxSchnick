use std::{collections::HashMap, sync::Arc};

use axum::http::StatusCode;
use tokio::sync::{
    RwLock,
    broadcast::{Receiver, Sender},
};
use uuid::Uuid;

use crate::schnick::SchnickHandle;

#[derive(Debug, Clone, Copy)]
pub enum SessionUpdate {
    SchnickStarted,
    SchnickRetried,
    SchnickEnded,
    SchnickUpdated,
}

/// Represents the server state of a single logged-in user.
#[derive(Debug, Clone)]
pub struct SessionHandle {
    /// The schnick this user is currently involved in, if any.
    pub(crate) schnick: Option<Arc<SchnickHandle>>,
    /// The login token.
    pub token: String,
    /// The current invite token of this user.
    pub invite: String,
    /// The channel used to notify a client of any updates.
    channel: Sender<SessionUpdate>,
}

impl SessionHandle {
    pub fn with_token(token: String) -> Self {
        Self {
            schnick: None,
            token,
            invite: Uuid::new_v4().to_string(),
            channel: Sender::new(4),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SessionManager {
    pub(crate) data: Arc<RwLock<HashMap<i32, SessionHandle>>>,
}

impl SessionManager {
    pub async fn receiver(&self, id: i32) -> Result<Receiver<SessionUpdate>, StatusCode> {
        if let Some(handle) = self.data.read().await.get(&id) {
            Ok(handle.channel.subscribe())
        } else {
            Err(StatusCode::NOT_FOUND)
        }
    }

    pub async fn sender(&self, id: i32) -> Result<Sender<SessionUpdate>, StatusCode> {
        if let Some(handle) = self.data.read().await.get(&id) {
            Ok(handle.channel.clone())
        } else {
            Err(StatusCode::NOT_FOUND)
        }
    }
}
