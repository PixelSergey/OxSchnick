use axum::{
    Router, extract::State, http::{StatusCode, header::CONTENT_TYPE}, response::{Html, IntoResponse}, routing::{get, post}
};
use axum_extra::{extract::CookieJar, response::Css};
use clap::Parser;
use dotenvy::dotenv;
use log::{debug, info};
use tokio::net::TcpListener;
use url::Url;

use crate::{
    app::{App, Session},
    events::events,
    home::home,
    invite::{invite, qrcode},
    schnick::schnick_select,
};

pub mod app;
pub mod events;
pub mod home;
pub mod invite;
pub mod schema;
pub mod schnick;
pub mod session;

/// A server for tracking schnicks.
#[derive(Debug, Clone, Parser)]
pub struct Config {
    /// base url the app will be served from
    base: String,

    /// address to bind to
    bind: String,
}

/// The `/` route.
pub async fn index(
    State(app): State<App>,
    cookies: CookieJar,
) -> Result<impl IntoResponse, StatusCode> {
    debug!(target: "home::home", "cookies={cookies:?}");
    let _ = app.authenticate(&cookies).await?;
    Ok(Html(include_str!("../templates/index.html")))
}

#[tokio::main]
async fn main() {
    env_logger::init();
    dotenv().ok();
    let config = Config::parse();
    let base = Url::parse(&config.base).expect("could not parse base url");
    info!(target: "main", "creating app");
    let app = App::new(base.clone()).await;
    app.authenticate_session(&Session {
        id: 1,
        token: "00000000-0000-0000-0000-000000000000".to_string(),
    })
    .await
    .expect("could not authenticate root user");
    println!(
        "{:?}",
        app.sessions
            .get_invite(1)
            .await
            .expect("no root user exists")
            .url(&base)
    );
    let router = Router::new()
        .route("/", get(index))
        .route("/events", get(events))
        .route("/qrcode", get(qrcode))
        .route("/invite", get(invite))
        .route("/select", post(schnick_select))
        .route("/home", get(home))
        .route(
            "/assets/style.css",
            get(async || Css(include_str!("../assets/style.css"))),
        )
        .route(
            "/assets/home.svg",
            get(async || {
                (
                    [(CONTENT_TYPE, "image/svg+xml")],
                    include_str!("../assets/home.svg"),
                )
            }),
        )
        .with_state(app);
    let listener = TcpListener::bind(config.bind)
        .await
        .expect("could not bind socket");
    axum::serve(listener, router)
        .await
        .expect("could not serve router");
}
