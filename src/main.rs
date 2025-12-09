use axum::{Router, routing::{get, post}};
use clap::Parser;
use dotenvy::dotenv;
use log::info;
use tokio::net::TcpListener;
use url::Url;

use crate::{
    app::App,
    home::{home, home_events},
    invite::{invite, qrcode},
    schnick::{schnick, schnick_events, schnick_select},
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
    let app = App::new(base.clone()).await;
    println!("{:?}", app.inviter.get(1).await.url(&base));
    let router = Router::new()
        .route("/", get(home))
        .route("/events", get(home_events))
        .route("/qrcode", get(qrcode))
        .route("/invite", get(invite))
        .route("/schnick", get(schnick))
        .route("/schnick/events", get(schnick_events))
        .route("/schnick/select", post(schnick_select))
        .with_state(app);
    let listener = TcpListener::bind(config.bind)
        .await
        .expect("could not bind socket");
    axum::serve(listener, router)
        .await
        .expect("could not serve router");
}
