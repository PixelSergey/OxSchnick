use axum::{
    Router,
    routing::{get, post},
};
use axum_extra::response::Css;
use diesel_async::{
    AsyncPgConnection,
    pooled_connection::{AsyncDieselConnectionManager, bb8::Pool},
};
use dotenvy::dotenv;
use std::{collections::HashMap, env, sync::Arc};
use tokio::{net::TcpListener, sync::RwLock};

use crate::{
    invite::invite,
    schnick::{OngoingSchnick, schnick, schnick_select, schnick_sse},
};

pub mod invite;
pub mod schema;
pub mod schnick;

#[derive(Debug, Clone)]
pub struct Server(
    Arc<Pool<AsyncPgConnection>>,
    Arc<RwLock<HashMap<i32, OngoingSchnick>>>,
);

#[tokio::main]
pub async fn main() {
    env_logger::init();
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
        .route(
            "/style.css",
            get(async || Css(include_str!("../templates/style.css"))),
        )
        .route("/invite", get(invite))
        .route("/schnick", get(schnick))
        .route("/schnick/sse", get(schnick_sse))
        .route("/schnick/select", post(schnick_select))
        .with_state(Server(
            Arc::new(pool),
            Arc::new(RwLock::new(HashMap::new())),
        ));
    let listener = TcpListener::bind("127.0.0.1:8080")
        .await
        .expect("Could not bind socket");
    axum::serve(listener, app).await.unwrap();
}
