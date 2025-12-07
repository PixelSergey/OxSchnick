use axum::{extract::State, http::{Response, StatusCode}, response::IntoResponse};
use axum_extra::extract::CookieJar;
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
    pub sender: Sender<()>,
}

/// The `/schnick` route.
pub async fn schnick(
    State(app): State<App>,
    cookies: CookieJar
) -> Result<impl IntoResponse, StatusCode> {
    let id = app.authenticate(&cookies).await?;
    let schnick = app.active_schnick(id).await?;
    todo!("not yet implemented");
    Ok(())
}