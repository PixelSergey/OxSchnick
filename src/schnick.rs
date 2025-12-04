use std::{collections::HashMap, sync::Arc};

use askama::Template;
use async_stream::try_stream;
use axum::{
    Form,
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response, Sse, sse::Event},
};
use axum_extra::extract::CookieJar;
use chrono::{DateTime, Utc};
use diesel::{dsl::insert_into, prelude::Insertable};
use diesel_async::{AsyncPgConnection, RunQueryDsl, pooled_connection::bb8::Pool};
use futures::Stream;
use log::{debug, trace};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use tokio::sync::{Mutex, broadcast::Sender};

use crate::{Server, invite::check_token};

#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum Weapon {
    Rock = 0,
    Scissors = 1,
    Paper = 2,
}

impl From<Weapon> for i32 {
    fn from(value: Weapon) -> Self {
        match value {
            Weapon::Rock => 0,
            Weapon::Scissors => 1,
            Weapon::Paper => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Template)]
#[template(path = "form.html")]
pub struct Interaction {
    pub won: bool,
    pub weapon: Weapon,
}

impl Interaction {
    pub fn compatible(&self, other: &Interaction) -> bool {
        let expected = ((self.weapon as i32) + if self.won { 1 } else { -1 }) % 3;
        return (self.won ^ other.won) && ((other.weapon as i32) == expected);
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SchnickEvent {
    Done,
    Retry,
    Cancel,
}

pub async fn authenticate(
    cookies: &CookieJar,
    pool: Arc<Pool<AsyncPgConnection>>,
) -> Result<i32, StatusCode> {
    let (id, token) = match (cookies.get("id"), cookies.get("token")) {
        (Some(id), Some(token)) => (
            id.value()
                .parse::<i32>()
                .map_err(|_| StatusCode::FORBIDDEN)?,
            token.value().to_string(),
        ),
        _ => return Err(StatusCode::FORBIDDEN),
    };
    check_token(
        id,
        &token,
        &mut pool
            .get()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    )
    .await?;
    Ok(id)
}

pub async fn schnick_sse(
    State(Server(pool, schnicks)): State<Server>,
    cookies: CookieJar,
) -> Result<Sse<impl Stream<Item = Result<Event, anyhow::Error>>>, StatusCode> {
    debug!(target: "schnick::schnick_sse", "invoked with cookies={cookies:?}");
    let id = authenticate(&cookies, pool).await?;
    let mut receiver = if let Some(current) = schnicks.read().await.get(&id) {
        current.1.subscribe()
    } else {
        return Err(StatusCode::NOT_FOUND);
    };
    trace!(target: "schnick::schnick_sse", "got receiver");
    Ok(Sse::new(try_stream! {
        trace!(target: "schnick::schnick_sse", "in handler");
        while let Ok(event) = receiver.recv().await {
            debug!(target: "schnick::schnick_sse", "event={event:?}");
            match event {
                SchnickEvent::Done | SchnickEvent::Cancel => {
                    yield Event::default().data(include_str!("../templates/reload.html"));
                    break;
                },
                SchnickEvent::Retry => {
                    yield Event::default().data(include_str!("../templates/form_empty.html"));
                }
            }
        }
        trace!(target: "schnick::schnick_sse", "terminating");
    }))
}

#[derive(Deserialize, Insertable)]
#[diesel(table_name = crate::schema::schnicks)]
pub struct InsertSchnick {
    pub winner: i32,
    pub loser: i32,
    pub weapon: i32,
    pub played_at: DateTime<Utc>,
}

pub async fn save_schnick(
    winner: i32,
    loser: i32,
    weapon: Weapon,
    pool: Arc<Pool<AsyncPgConnection>>,
) -> Result<(), StatusCode> {
    use crate::schema::schnicks;
    let new = InsertSchnick {
        winner,
        loser,
        weapon: weapon as i32,
        played_at: Utc::now(),
    };
    let mut conn = pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    insert_into(schnicks::table)
        .values(&new)
        .execute(&mut conn)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        .map(|_| ())
}

pub fn remove_schnick(
    ida: i32,
    idb: i32,
    schnicks: &mut HashMap<i32, Arc<(Mutex<Option<(i32, Interaction)>>, Sender<SchnickEvent>)>>,
) -> Result<(), StatusCode> {
    schnicks
        .remove(&ida)
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    schnicks
        .remove(&idb)
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(())
}

pub async fn schnick_select(
    State(Server(pool, schnicks)): State<Server>,
    cookies: CookieJar,
    Form(interaction): Form<Interaction>,
) -> Result<Response, StatusCode> {
    debug!(target: "schnick::schnick_select", "invoked with cookies={cookies:?}, interaction={interaction:?}");
    let id = authenticate(&cookies, Arc::clone(&pool)).await?;
    let mut schnicks = schnicks.write().await;
    let entry = schnicks.get(&id).clone().map(|a| Arc::clone(a));
    if let Some(schnick) = entry {
        let mut current = schnick.0.lock().await;
        debug!(target: "schnick::schnick_select", "id={id:?}, current={current:?}, interaction={interaction:?}");
        match (*current, interaction) {
            (Some((other, old)), new) if new.compatible(&old) && other != id => {
                debug!(target: "schnick::schnick_select", "got compatible interactions, saving");
                let (winner, loser) = if new.won { (id, other) } else { (other, id) };
                let weapon = if new.won { new.weapon } else { old.weapon };
                save_schnick(winner, loser, weapon, pool).await?;
                remove_schnick(id, other, &mut schnicks)?;
                schnick
                    .1
                    .send(SchnickEvent::Done)
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                Ok(([("HX-Redirect", "")], StatusCode::OK).into_response())
            }
            (None, new) => {
                debug!(target: "schnick::schnick_select", "got first interaction, replacing");
                current.replace((id, new));
                Ok(Html(
                    new.render()
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
                )
                .into_response())
            }
            (Some((other, _)), new) if id == other => {
                debug!(target: "schnick::schnick_select", "got subsequent interaction from same user, replacing");
                current.replace((id, new));
                Ok(Html(
                    new.render()
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
                )
                .into_response())
            }
            _ => {
                debug!(target: "schnick::schnick_select", "got invalid interactions, resetting");
                current.take();
                schnick
                    .1
                    .send(SchnickEvent::Retry)
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                Ok(Html(include_str!("../templates/form_empty.html")).into_response())
            }
        }
    } else {
        trace!(target: "schnick::schnick_select", "no current schnick");
        Err(StatusCode::NOT_FOUND)
    }
}

#[derive(Debug, Clone, Template)]
#[template(path = "schnick.html")]
pub struct SchnickTemplate {
    pub waiting: bool
}

pub async fn schnick(
    State(Server(pool, schnicks)): State<Server>,
    cookies: CookieJar,
) -> Result<impl IntoResponse, StatusCode> {
    debug!(target: "schnick::schnick", "invoked with cookies={cookies:?}");
    let id = authenticate(&cookies, pool).await?;
    if let Some(handle) = schnicks.read().await.get(&id) {
        Ok(Html::from(
            SchnickTemplate {
                waiting: false
            }
                .render()
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        ))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
