use axum::{
    Router,
    extract::{Path, State},
    http::{StatusCode, header::CONTENT_TYPE},
    response::{Html, IntoResponse},
    routing::{get, post},
};
use axum_extra::extract::CookieJar;
use clap::Parser;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use dotenvy::dotenv;
use log::{debug, info};
use tokio::net::TcpListener;
use url::Url;

use crate::{
    app::{App, Session},
    events::events,
    graphs::graphs,
    home::home,
    invite::{invite, qrcode},
    metrics::metrics,
    schnick::{schnick_abort, schnick_select},
    settings::{settings, settings_about, settings_dect, settings_imprint, settings_username},
};

pub mod app;
pub mod events;
pub mod home;
pub mod graphs;
pub mod invite;
pub mod metrics;
pub mod schema;
pub mod schnick;
pub mod session;
pub mod settings;

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

macro_rules! serve_static {
    ( $name:expr, [ $( [ $path:literal, $file:expr, $type:literal ] ),* ]) => {
        match $name {
            $(
                $path => Ok(([(CONTENT_TYPE, $type)], &include_bytes!($file)[..])),
            )*
            _ => Err(StatusCode::NOT_FOUND)
        }
    };
}

#[tokio::main]
async fn main() {
    use crate::schema::users;
    env_logger::init();
    dotenv().ok();
    let config = Config::parse();
    let base = Url::parse(&config.base).expect("could not parse base url");
    info!(target: "main", "creating app");
    let app = App::new(base.clone()).await;
    let root_token = users::table
        .filter(users::id.eq(1))
        .select(users::token)
        .first::<String>(&mut app.connection().await.unwrap())
        .await
        .unwrap();
    app.authenticate_session(&Session {
        id: 1,
        token: root_token,
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
        .route("/abort", post(schnick_abort))
        .route("/home", get(home))
        .route("/graphs", get(graphs))
        .route("/metrics", get(metrics))
        .route("/settings", get(settings))
        .route("/settings/username", post(settings_username))
        .route("/settings/dect", post(settings_dect))
        .route("/settings/about", get(settings_about))
        .route("/settings/imprint", get(settings_imprint))
        .route(
            "/assets/{file}",
            get(async |Path(file): Path<String>| {
                serve_static!(
                    &file[..],
                    [
                        ["abort.svg", "../assets/abort.svg", "image/svg+xml"],
                        ["adult.svg", "../assets/adult.svg", "image/svg+xml"],
                        ["arrow_back.svg", "../assets/arrow_back.svg", "image/svg+xml"],
                        ["arrow_right.svg", "../assets/arrow_right.svg", "image/svg+xml"],
                        ["children.svg", "../assets/children.svg", "image/svg+xml"],
                        ["distance.svg", "../assets/distance.svg", "image/svg+xml"],
                        ["hash_char.svg", "../assets/hash_char.svg", "image/svg+xml"],
                        ["lost.svg", "../assets/lost.svg", "image/svg+xml"],
                        ["paper.svg", "../assets/paper.svg", "image/svg+xml"],
                        ["phone_receiver.svg", "../assets/phone_receiver.svg", "image/svg+xml"],
                        ["rock.svg", "../assets/rock.svg", "image/svg+xml"],
                        ["scissors.svg", "../assets/scissors.svg", "image/svg+xml"],
                        ["score.svg", "../assets/score.svg", "image/svg+xml"],
                        ["spider_web.svg", "../assets/spider_web.svg", "image/svg+xml"],
                        ["streak.svg", "../assets/streak.svg", "image/svg+xml"],
                        ["style.css", "../assets/style.css", "text/css"],
                        ["won.svg", "../assets/won.svg", "image/svg+xml"],
                        ["wrench.svg", "../assets/wrench.svg", "image/svg+xml"]
                    ]
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
