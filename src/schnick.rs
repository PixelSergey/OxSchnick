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

use crate::app::App;

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
pub struct Schnick {
    pub ids: (i32, i32),
    pub partial: Mutex<Option<(i32, Interaction)>>,
    /// The event channel for this schnick.
    ///
    /// This is subscribed to by event sources. A value of None means the schnick has been cancelled.
    /// A value of Some(false) means the schnick has been reset and a value of Some(true) means it has successfully concluded.
    pub sender: Sender<Option<bool>>,
}

/// Event source for schnick updates
pub async fn schnick_events(
    State(app): State<App>,
    cookies: CookieJar,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    debug!(target: "schnick::schnick_events", "cookies={cookies:?}");
    let id = app.authenticate(&cookies).await?;
    let schnick = app.active_schnick(id).await?;
    let mut receiver = schnick.sender.subscribe();
    let stream = try_stream! {
        yield Event::default().data(include_str!("../templates/form.html"));
        while let Ok(Some(event)) = receiver.recv().await {
            if event {
                yield Event::default().event("redirect").data("location.href=\"..\";");
            } else {
                yield Event::default().data(include_str!("../templates/form.html"))
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
    let schnick = app.active_schnick(id).await?;
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
            app.end_schnick(Arc::clone(&schnick)).await;
            let (winner, loser, weapon) = if interaction.won {
                (id, other, interaction.weapon)
            } else {
                (other, id, old.weapon)
            };
            app.save_schnick(winner, loser, weapon).await?;
            let _ = schnick.sender.send(Some(true));
            Ok(Html(include_str!("../templates/redirect.html")))
        },
        Some((other, _)) if id == other => {
            trace!(target: "schnick::schnick_select", "new interaction received from same user, ignoring");
            Ok(Html(include_str!("../templates/waiting.html")))
        },
        _ => {
            trace!(target: "schnick::schnick_select", "invalid interaction received, resetting");
            partial.take();
            let _ = schnick.sender.send(Some(false));
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
    let _schnick = app.active_schnick(id).await?;
    Ok(Html(include_str!("../templates/schnick.html")))
}
