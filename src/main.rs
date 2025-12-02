use axum::{Router, routing::get};
use diesel_async::{
    AsyncPgConnection,
    pooled_connection::{AsyncDieselConnectionManager, bb8::Pool},
};
use dotenvy::dotenv;
use std::{collections::HashMap, env, sync::Arc};
use tokio::{
    net::TcpListener,
    sync::{Mutex, RwLock},
};

use crate::{schnick::OngoingSchnick, invite::invite};

pub mod schnick;
pub mod schema;
pub mod invite;

#[derive(Debug, Clone)]
pub struct Server(
    Pool<AsyncPgConnection>,
    Arc<RwLock<HashMap<i32, Arc<Mutex<OngoingSchnick>>>>>,
);

#[tokio::main]
pub async fn main() {
    dotenv().ok();
    let pool = {
        let url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(url);
        Pool::builder()
            .build(config)
            .await
            .expect("Could not connect to database")
    };
    let app = Router::new()
        .route("/", get(async || "hi"))
        .route("/invite", get(invite))
        .with_state(Server(pool, Arc::new(RwLock::new(HashMap::new()))));
    let listener = TcpListener::bind("127.0.0.1:8080")
        .await
        .expect("Could not bind socket");
    axum::serve(listener, app).await.unwrap();
}
