use std::sync::Arc;

use axum::response::sse::Event;
use diesel_async::{AsyncPgConnection, pooled_connection::bb8::Pool};
use tokio::sync::{RwLock, broadcast, mpsc::Sender};
use url::Url;

use crate::{auth::AuthenticationRequest, schnicks::SchnickRequest};

#[derive(Debug, Clone)]
pub struct State {
    pub base_url: Url,
    pub pool: Pool<AsyncPgConnection>,
    pub authenticator: Sender<AuthenticationRequest>,
    pub schnicker: Sender<SchnickRequest>,
    pub graph_cache: Arc<RwLock<String>>,
    pub graph_updates: Arc<broadcast::Receiver<Arc<Event>>>,
}
