use std::{collections::HashMap, env, sync::Arc};

use axum::http::StatusCode;
use axum_extra::extract::CookieJar;
use chrono::{DateTime, Utc};
use diesel::{dsl::insert_into, prelude::*, update};
use diesel_async::{
    AsyncPgConnection, RunQueryDsl,
    pooled_connection::{
        AsyncDieselConnectionManager,
        bb8::{Pool, PooledConnection},
    },
};
use log::{debug, info, trace};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, broadcast::Sender};
use url::Url;
use uuid::Uuid;

use crate::{
    invite::Invite,
    schnick::{Schnick, Weapon},
};

pub const SESSION_COOKIE_NAME: &'static str = "session";

/// Represents the app state.
///
/// Contains the database connection pool and active schnicks.
#[derive(Debug, Clone)]
pub struct App {
    pub base: Url,
    pool: Arc<Pool<AsyncPgConnection>>,
    schnicks: Arc<Mutex<HashMap<i32, Arc<Schnick>>>>,
    pub redirects: Arc<Mutex<HashMap<i32, tokio::sync::watch::Sender<()>>>>,
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
            schnicks: Arc::new(Mutex::new(HashMap::new())),
            pool: Arc::new(pool),
            redirects: Arc::new(Mutex::new(HashMap::new())),
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
        let real = Session::query()
            .find(session.id)
            .first(&mut self.connection().await?)
            .await
            .map_err(|_| StatusCode::FORBIDDEN)?;
        if real.token == session.token {
            debug!(target: "app::authenticating_session", "token match");
            Ok(())
        } else {
            debug!(target: "app::authenticating_session", "token mismatch");
            Err(StatusCode::FORBIDDEN)
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
        let invite_token = Uuid::new_v4().to_string();
        let id = insert_into(users::table)
            .values((
                users::parent.eq(parent),
                users::token.eq(&session_token),
                users::invite.eq(&invite_token),
                users::created.eq(Utc::now()),
                users::active.eq(true),
            ))
            .returning(users::id)
            .get_result(&mut self.connection().await?)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(Session {
            id,
            token: session_token,
        })
    }

    /// Authenticates a given `invite`.
    ///
    /// Checks whether a user with the id `invite.id` and invite token `invite.token` exists and is active.
    pub async fn authenticate_invite(&self, invite: &Invite) -> Result<(), StatusCode> {
        use crate::schema::users;
        Invite::query()
            .filter(users::id.eq(invite.id))
            .filter(users::invite.eq(&invite.token))
            //.filter(users::active.eq(true)) // UNCOMMENT TO ENFORCE ACTIVE FLAG
            .first(&mut self.connection().await?)
            .await
            .map_err(|_| StatusCode::FORBIDDEN)
            .map(|_| ())
    }

    /// Renews the invite token of the given user.
    pub async fn renew_invite(&self, id: i32) -> Result<(), StatusCode> {
        use crate::schema::users;
        let invite_token = Uuid::new_v4().to_string();
        update(users::table)
            .filter(users::id.eq(id))
            .set(users::invite.eq(&invite_token))
            .execute(&mut self.connection().await?)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
            .map(|_| ())
    }

    /// Gets the current `Invite` of the user with the given `id` fron the database.
    pub async fn get_invite(&self, id: i32) -> Result<Invite, StatusCode> {
        Invite::query()
            .find(id)
            .first(&mut self.connection().await?)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    }

    /// Gets the active schnick of the user with id `id`, if any.
    pub async fn active_schnick(&self, id: i32) -> Result<Arc<Schnick>, StatusCode> {
        self.schnicks
            .lock()
            .await
            .get(&id)
            .cloned()
            .ok_or(StatusCode::NOT_FOUND)
    }

    /// Starts a new active schnick between users with ids `id` and `other` and set it as their active schnick
    pub async fn start_schnick(&self, id: i32, other: i32) {
        let schnick = Arc::new(Schnick {
            ids: (id, other),
            partial: Mutex::new(None),
            sender: Sender::new(4),
        });
        let mut schnicks = self.schnicks.lock().await;
        schnicks.insert(id, Arc::clone(&schnick));
        schnicks.insert(other, schnick);
    }

    /// Ends the active schnick.
    pub async fn end_schnick(&self, schnick: Arc<Schnick>) {
        let (id, other) = schnick.ids;
        let mut schnicks = self.schnicks.lock().await;
        schnicks.remove(&id);
        schnicks.remove(&other);
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

    // TODO: fix this hack
    pub async fn redirect(id: i32, redirects: Arc<Mutex<HashMap<i32, tokio::sync::watch::Sender<()>>>>) {
        let mut receiver = {
            let mut redirects = redirects.lock().await;
            if let Some(sender) = redirects.get(&id) {
                sender.subscribe()
            } else {
                let new = tokio::sync::watch::Sender::new(());
                let receiver = new.subscribe();
                trace!(target: "app::redirect", "sanity check (before)");
                redirects.insert(id, new);
                trace!(target: "app::redirect", "sanity check (after)");
                receiver
            }
        };
        trace!(target: "app::redirect", "got receiver, waiting for update");
        let _ = receiver.changed().await;
        trace!(target: "app::redirect", "got update, removing");
        redirects.lock().await.remove(&id);
    }
}
