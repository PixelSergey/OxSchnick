use axum::{Router, routing::get};
use diesel::prelude::*;
use diesel_async::{
    AsyncPgConnection,
    pooled_connection::{AsyncDieselConnectionManager, bb8::Pool},
};
use dotenvy::dotenv;
use uuid::Uuid;
use std::{collections::HashMap, env, sync::Arc};
use tokio::{net::TcpListener, sync::{Mutex, RwLock}};

use crate::{matches::IncompleteMatch, users::invite};

pub mod schema;
pub mod users;
pub mod matches;

pub struct Server (Pool<AsyncPgConnection>, RwLock<HashMap<Uuid, Arc<Mutex<IncompleteMatch>>>>);

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
        .with_state(pool);
    let listener = TcpListener::bind("127.0.0.1:8080")
        .await
        .expect("Could not bind socket");
    axum::serve(listener, app).await.unwrap();
}
