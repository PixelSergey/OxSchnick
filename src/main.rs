use axum::{Router, routing::get};
use clap::Parser;
use dotenvy::dotenv;
use log::info;
use tokio::net::TcpListener;
use url::Url;

use crate::{
    app::App,
    home::home,
    invite::{invite, qrcode},
};

pub mod app;
pub mod home;
pub mod invite;
pub mod schema;
pub mod schnick;

/// A server for tracking schnicks.
#[derive(Debug, Clone, Parser)]
pub struct Config {
    /// base url the app will be served from
    base: String,

    /// address to bind to
    bind: String,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    dotenv().ok();
    let config = Config::parse();
    let base = Url::parse(&config.base).expect("could not parse base url");
    info!(target: "main", "creating app");
    let app = App::new(base).await;
    let router = Router::new()
        .route("/", get(home))
        .route("/qrcode", get(qrcode))
        .route("/invite", get(invite))
        .with_state(app);
    let listener = TcpListener::bind(config.bind)
        .await
        .expect("could not bind socket");
    axum::serve(listener, router)
        .await
        .expect("could not serve router");
}
