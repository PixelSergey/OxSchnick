use std::{convert::Infallible, sync::Arc, time::Duration};

use async_stream::try_stream;
use axum::{
    Form, extract::State, http::StatusCode, response::{
        Html, IntoResponse, Sse,
        sse::{Event, KeepAlive},
    }
};
use axum_extra::extract::CookieJar;
use futures::Stream;
use log::{debug, trace};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use tokio::sync::{Mutex, broadcast::Sender};

use crate::{app::App, session::{SessionManager, SessionUpdate}};

/// A weapon type in a schnick.
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq)]
#[repr(u8)]
pub enum Weapon {
    Rock = 0,
    Scissors = 1,
    Paper = 2,
}

/// The outcome of a schnick from the point of view of one of the players.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Interaction {
    pub won: bool,
    pub weapon: Weapon,
}

impl Interaction {
    fn complementary(&self) -> Self {
        Self {
            won: !self.won,
            weapon: match (self.weapon, self.won) {
                (Weapon::Rock, true) => Weapon::Scissors,
                (Weapon::Paper, true) => Weapon::Rock,
                (Weapon::Scissors, true) => Weapon::Paper,
                (Weapon::Rock, false) => Weapon::Paper,
                (Weapon::Paper, false) => Weapon::Scissors,
                (Weapon::Scissors, false) => Weapon::Rock,
            },
        }
    }

    pub fn compatible(&self, other: &Self) -> bool {
        &self.complementary() == other
    }
}

/// The state of a schnick match from the point of view of one of the players.
#[derive(Debug)]
pub struct SchnickHandle {
    pub ids: (i32, i32),
    pub partial: Mutex<Option<(i32, Interaction)>>,
    /// The event channel for this schnick.
    ///
    /// This is subscribed to by event sources. A value of None means the schnick has been cancelled.
    /// A value of Some(false) means the schnick has been reset and a value of Some(true) means it has successfully concluded.
    pub sender: Sender<Option<bool>>,
}

impl SessionManager {
    /// Gets the active schnick of the user with id `id`, if any.
    pub async fn active_schnick(&self, id: i32) -> Result<Arc<SchnickHandle>, StatusCode> {
        self.data
            .read()
            .await
            .get(&id)
            .ok_or(StatusCode::NOT_FOUND)?
            .schnick
            .clone()
            .ok_or(StatusCode::NOT_FOUND)
    }

    /// Starts a new active schnick between users with ids `id` and `other` and set it as their active schnick
    pub async fn start_schnick(&self, id: i32, other: i32) -> Result<(), StatusCode> {
        let schnick = Arc::new(SchnickHandle {
            ids: (id, other),
            partial: Mutex::new(None),
            sender: Sender::new(4),
        });
        let mut data = self.data.write().await;
        data.get_mut(&id).ok_or(StatusCode::INTERNAL_SERVER_ERROR)?.schnick.replace(Arc::clone(&schnick));
        data.get_mut(&other).ok_or(StatusCode::INTERNAL_SERVER_ERROR)?.schnick.replace(schnick);
        Ok(())
    }

    /// Ends the active schnick.
    pub async fn end_schnick(&self, schnick: Arc<SchnickHandle>) -> Result<(), StatusCode> {
        let (id, other) = schnick.ids;
        let mut data = self.data.write().await;
        data.get_mut(&id).ok_or(StatusCode::INTERNAL_SERVER_ERROR)?.schnick.take();
        data.get_mut(&other).ok_or(StatusCode::INTERNAL_SERVER_ERROR)?.schnick.take();
        Ok(())
    }
}

/// Event source for schnick updates
pub async fn schnick_events(
    State(app): State<App>,
    cookies: CookieJar,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    debug!(target: "schnick::schnick_events", "cookies={cookies:?}");
    trace!(target: "schnick::schnick_events", "authenticating");
    let id = app.authenticate(&cookies).await?;
    trace!(target: "schnick::schnick_events", "checking for active schnick");
    let _ = app.sessions.active_schnick(id).await?;
    trace!(target: "schnick::schnick_events", "getting receiver");
    let mut receiver = app.sessions.receiver(id).await?;
    let stream = try_stream! {
        yield Event::default().data(include_str!("../templates/form.html"));
        while let Ok(update) = receiver.recv().await {
            match update {
                SessionUpdate::SchnickEnded => {yield Event::default().event("redirect").data("location.href=\"..\";"); break;}
                SessionUpdate::SchnickRetried => yield Event::default().data(include_str!("../templates/form.html")),
                _ => {break},
            }
        }
    };
    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(1))))
}

/// Event source for schnick updates
pub async fn schnick_select(
    State(app): State<App>,
    cookies: CookieJar,
    Form(interaction): Form<Interaction>
) -> Result<impl IntoResponse, StatusCode> {
    debug!(target: "schnick::schnick_select", "cookies={cookies:?} interaction={interaction:?}");
    let id = app.authenticate(&cookies).await?;
    let schnick = app.sessions.active_schnick(id).await?;
    let sender = app.sessions.sender(if schnick.ids.0 == id {schnick.ids.1} else {id}).await?;
    // TODO: think about contention and timing attacks with inner mutability
    let mut partial = schnick.partial.lock().await;
    match *partial {
        None => {
            trace!(target: "schnick::schnick_select", "no partial interaction found, replacing");
            partial.replace((id, interaction));
            Ok(Html(include_str!("../templates/waiting.html")))
        },
        Some((other, old)) if old.compatible(&interaction) && id != other => {
            trace!(target: "schnick::schnick_select", "compatible interaction received, concluding");
            app.sessions.end_schnick(Arc::clone(&schnick)).await?;
            let (winner, loser, weapon) = if interaction.won {
                (id, other, interaction.weapon)
            } else {
                (other, id, old.weapon)
            };
            app.save_schnick(winner, loser, weapon).await?;
            let _ = sender.send(SessionUpdate::SchnickEnded);
            Ok(Html(include_str!("../templates/redirect.html")))
        },
        Some((other, _)) if id == other => {
            trace!(target: "schnick::schnick_select", "new interaction received from same user, ignoring");
            Ok(Html(include_str!("../templates/waiting.html")))
        },
        _ => {
            trace!(target: "schnick::schnick_select", "invalid interaction received, resetting");
            partial.take();
            let _ = sender.send(SessionUpdate::SchnickRetried);
            Ok(Html(include_str!("../templates/form.html")))
        }
    }
}

/// The `/schnick` route.
pub async fn schnick(
    State(app): State<App>,
    cookies: CookieJar,
) -> Result<impl IntoResponse, StatusCode> {
    let id = app.authenticate(&cookies).await?;
    let _schnick = app.sessions.active_schnick(id).await?;
    Ok(Html(include_str!("../templates/schnick.html")))
}
