use std::env;

use anyhow::anyhow;
use clap::Parser;
use diesel_async::{
    AsyncPgConnection,
    pooled_connection::{AsyncDieselConnectionManager, bb8::Pool},
};
use dotenvy::dotenv;
use tokio::{net::TcpListener, task::LocalSet};
use url::Url;

use crate::router::router;

pub mod auth;
pub mod router;
pub mod routes;
pub mod schema;
pub mod schnicks;
pub mod state;
pub mod users;

/// A server for tracking schnicks.
#[derive(Debug, Clone, Parser)]
pub struct Config {
    /// base url the app will be served from
    base: String,

    /// address to bind to
    bind: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    dotenv().ok();
    let config = Config::parse();
    let base_url = Url::parse(&config.base)?;
    let pool = {
        let url = env::var("DATABASE_URL")?;
        let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(url);
        Pool::builder().build(config).await?
    };
    let listener = TcpListener::bind(config.bind).await?;
    let (router, mut authenticator, schnicker) = router(base_url, pool).await?;
    let invite = authenticator
        .root_invite()
        .await
        .ok_or(anyhow!("no root user"))?;
    println!("{invite:?}");
    let local_set = LocalSet::new();
    let schnicker_handle = local_set.spawn_local(schnicker.worker());
    let authenticator_handle = tokio::spawn(authenticator.worker());
    let _ = tokio::join!(
        local_set,
        schnicker_handle,
        authenticator_handle,
        axum::serve(listener, router)
    );
    Ok(())
}
