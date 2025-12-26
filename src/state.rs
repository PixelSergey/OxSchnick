use std::sync::Arc;

use diesel_async::{AsyncPgConnection, pooled_connection::bb8::Pool};
use tokio::sync::{RwLock, mpsc::Sender};
use url::Url;

use crate::{auth::AuthenticationRequest, graphs::GraphRequest, metrics::Metrics, schnicks::SchnickRequest};

#[derive(Clone)]
pub struct State {
    pub base_url: Url,
    pub pool: Pool<AsyncPgConnection>,
    pub authenticator: Sender<AuthenticationRequest>,
    pub schnicker: Sender<SchnickRequest>,
    pub graphs: Sender<GraphRequest>,
    pub metrics: Arc<RwLock<Metrics>>
}
