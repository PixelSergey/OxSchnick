use std::f64;

use crate::{auth::AuthenticatorEntry, schnicks::Weapon, state::State};
use axum::{extract::FromRequestParts, http::StatusCode};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use libm::erf;
use log::error;

#[derive(Debug, Clone, Identifiable, HasQuery, QueryableByName, AsChangeset)]
#[diesel(table_name=crate::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Settings {
    pub id: i32,
    pub username: String,
    pub college: Option<i32>,
}

#[derive(Debug, Clone, Identifiable, HasQuery, QueryableByName)]
#[diesel(table_name=crate::schema::metrics)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Stats {
    pub id: i32,
    pub num_schnicks: i32,
    pub num_won: i32,
    pub longest_winning_streak: i32,
    pub current_winning_streak: i32,
    pub longest_losing_streak: i32,
    pub current_losing_streak: i32,
    pub num_children: i32,
    pub num_rock: i32,
    pub num_paper: i32,
    pub num_scissors: i32,
}

impl Stats {
    pub fn favorites(&self) -> &[Weapon] {
        if self.num_rock < self.num_paper {
            if self.num_paper < self.num_scissors {
                &[Weapon::Scissors]
            } else if self.num_paper == self.num_scissors {
                &[Weapon::Scissors, Weapon::Paper]
            } else {
                &[Weapon::Paper]
            }
        } else if self.num_rock == self.num_paper {
            if self.num_paper < self.num_scissors {
                &[Weapon::Scissors]
            } else if self.num_paper == self.num_scissors {
                &[Weapon::Rock, Weapon::Paper, Weapon::Scissors]
            } else {
                &[Weapon::Rock, Weapon::Paper]
            }
        } else {
            if self.num_rock < self.num_scissors {
                &[Weapon::Scissors]
            } else if self.num_rock == self.num_scissors {
                &[Weapon::Rock, Weapon::Scissors]
            } else {
                &[Weapon::Rock]
            }
        }
    }
}

impl FromRequestParts<State> for Settings {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &State,
    ) -> Result<Self, Self::Rejection> {
        use crate::schema::users;
        let (id, _) = parts
            .extensions
            .get::<(i32, AuthenticatorEntry)>()
            .ok_or(StatusCode::FORBIDDEN)?;
        users::table
            .filter(users::id.eq(id))
            .select(Settings::as_select())
            .first::<Settings>(&mut state.pool.get().await.map_err(|e| {
                error!(target: "users::from_request_parts", "{:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?)
            .await
            .map_err(|e| {
                error!(target: "users::from_request_parts", "{:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })
    }
}

impl FromRequestParts<State> for (Settings, Stats, i32) {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &State,
    ) -> Result<Self, Self::Rejection> {
        use crate::schema::metrics;
        use crate::schema::users;
        let (id, _) = parts
            .extensions
            .get::<(i32, AuthenticatorEntry)>()
            .ok_or(StatusCode::FORBIDDEN)?;
        let (settings, stats) = users::table
            .filter(users::id.eq(id))
            .inner_join(metrics::table)
            .select((Settings::as_select(), Stats::as_select()))
            .first::<(Settings, Stats)>(&mut state.pool.get().await.map_err(|e| {
                error!(target: "users::from_request_parts", "{:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?)
            .await
            .map_err(|e| {
                error!(target: "users::from_request_parts", "{:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        let score = if stats.num_schnicks == 0 {
            0
        } else {
            /* "CAST(
                (erf(((num_won - num_schnicks * 0.5) / sqrt(num_schnicks * 0.25)) / sqrt(2)) * 10) ^ 3 AS INTEGER)" */
            let num_won = stats.num_won as f64;
            let num_schnicks = stats.num_schnicks as f64;
            (erf(((num_won - num_schnicks * 0.5) / (num_schnicks * 0.25).sqrt()) / f64::consts::SQRT_2) * 10.0).powi(3) as i32
        };
        Ok((settings, stats, score))
    }
}
