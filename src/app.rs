use std::{env, sync::Arc};

use axum::http::StatusCode;
use axum_extra::extract::CookieJar;
use chrono::{DateTime, Utc};
use diesel::{dsl::insert_into, prelude::*};
use diesel_async::{
    AsyncPgConnection, RunQueryDsl,
    pooled_connection::{
        AsyncDieselConnectionManager,
        bb8::{Pool, PooledConnection},
    },
};
use log::{debug, info};
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use crate::{
    schnick::Weapon,
    session::{SessionHandle, SessionManager},
};

pub const SESSION_COOKIE_NAME: &'static str = "session";

/// Represents the app state.
///
/// Contains the database connection pool and active schnicks.
#[derive(Debug, Clone)]
pub struct App {
    pub base: Url,
    pool: Arc<Pool<AsyncPgConnection>>,
    pub sessions: SessionManager,
}

/// Represents the login information needed to identify and authenticate a user.
#[derive(Debug, Clone, Serialize, Deserialize, HasQuery)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Session {
    pub id: i32,
    pub token: String,
}

impl App {
    /// Create new app instance, reading database info from environment variables.
    pub async fn new(base: Url) -> Self {
        info!(target: "app::App::new", "connecting to database");
        let pool = {
            let url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
            let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(url);
            Pool::builder()
                .build(config)
                .await
                .expect("could not connect to database")
        };
        Self {
            base,
            pool: Arc::new(pool),
            sessions: Default::default(),
        }
    }

    /// Returns a connection from the connection pool.
    async fn connection(&self) -> Result<PooledConnection<'_, AsyncPgConnection>, StatusCode> {
        self.pool
            .get()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    }

    /// Checks if a given `session` is valid.
    ///
    /// Returns Ok(()) if user with id `session.id` and token `session.token` can be found in the database.
    pub async fn authenticate_session(&self, session: &Session) -> Result<(), StatusCode> {
        debug!(target: "app::authenticating_session", "session={session:?}");
        let guard = self.sessions.data.read().await;
        if let Some(handle) = guard.get(&session.id) {
            if handle.token == session.token {
                Ok(())
            } else {
                Err(StatusCode::FORBIDDEN)
            }
        } else {
            let entry = Session::query()
                .find(session.id)
                .first(&mut self.connection().await?)
                .await
                .map_err(|_| StatusCode::FORBIDDEN)?;
            if entry.token == session.token {
                drop(guard);
                self.sessions
                    .data
                    .write()
                    .await
                    .insert(session.id, SessionHandle::with_token(session.token.clone()));
                Ok(())
            } else {
                println!("{entry:?} vs {session:?}");
                Err(StatusCode::FORBIDDEN)
            }
        }
    }

    /// Checks if a given session is valid given cookies.
    pub async fn authenticate(&self, cookies: &CookieJar) -> Result<i32, StatusCode> {
        let cookie = if let Some(cookie) = cookies.get(SESSION_COOKIE_NAME) {
            cookie.value()
        } else {
            return Err(StatusCode::FORBIDDEN);
        };
        let session = serde_json::from_str::<Session>(cookie).map_err(|_| StatusCode::FORBIDDEN)?;
        self.authenticate_session(&session).await?;
        Ok(session.id)
    }

    /// Registers a new user with given `parent`.
    ///
    /// Returns the session for the newly created user.
    pub async fn register(&self, parent: i32) -> Result<Session, StatusCode> {
        use crate::schema::users;
        let session_token = Uuid::new_v4().to_string();
        let id = insert_into(users::table)
            .values((
                users::parent.eq(parent),
                users::token.eq(&session_token),
                users::created.eq(Utc::now()),
                users::active.eq(true),
            ))
            .returning(users::id)
            .get_result(&mut self.connection().await?)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let session = Session {
            id,
            token: session_token,
        };
        self.authenticate_session(&session).await?;
        Ok(session)
    }

    /// Saves the conclusion of a schnick to the database.
    pub async fn save_schnick(
        &self,
        winner: i32,
        loser: i32,
        weapon: Weapon,
    ) -> Result<(), StatusCode> {
        use crate::schema::schnicks;
        insert_into(schnicks::table)
            .values((
                schnicks::winner.eq(winner),
                schnicks::loser.eq(loser),
                schnicks::weapon.eq(weapon as i32),
                schnicks::played_at.eq(Utc::now()),
            ))
            .execute(&mut self.connection().await?)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
            .map(|_| ())
    }

    pub async fn have_schnicked(&self, id: i32, other: i32) -> Result<bool, StatusCode> {
        use crate::schema::schnicks;
        schnicks::table
            .filter(
                (schnicks::winner.eq(id).and(schnicks::loser.eq(other)))
                    .or(schnicks::loser.eq(id).and(schnicks::winner.eq(other))),
            )
            .first::<(i32, i32, i32, i32, DateTime<Utc>)>(&mut self.connection().await?)
            .await
            .optional()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
            .map(|schnick| schnick.is_some())
    }
}
