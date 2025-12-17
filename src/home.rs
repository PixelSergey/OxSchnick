use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
};
use axum_extra::extract::CookieJar;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use log::{debug, error};

use crate::{app::App, schnick::Weapon, settings::User};

#[derive(Template)]
#[template(path = "home.html")]
struct Home {
    pub username: String,
    pub invite: String,
    pub num_schnicks: i32,
    pub num_won: i32,
    pub score: i32,
    pub win_streak: (i32, i32),
    pub lose_streak: (i32, i32),
    pub children: i32,
    pub favorites: Vec<Weapon>,
}

#[derive(Debug, Clone, Identifiable, HasQuery, QueryableByName)]
#[diesel(table_name=crate::schema::metrics)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Stats {
    id: i32,
    num_schnicks: i32,
    num_won: i32,
    longest_winning_streak: i32,
    current_winning_streak: i32,
    longest_losing_streak: i32,
    current_losing_streak: i32,
    num_children: i32,
    num_rock: i32,
    num_paper: i32,
    num_scissors: i32,
}

impl Stats {
    pub fn favorites(&self) -> Vec<Weapon> {
        if self.num_rock < self.num_paper {
            if self.num_paper < self.num_scissors {
                vec![Weapon::Scissors]
            } else if self.num_paper == self.num_scissors {
                vec![Weapon::Scissors, Weapon::Paper]
            } else {
                vec![Weapon::Paper]
            }
        } else if self.num_rock == self.num_paper {
            if self.num_paper < self.num_scissors {
                vec![Weapon::Scissors]
            } else if self.num_paper == self.num_scissors {
                vec![Weapon::Rock, Weapon::Paper, Weapon::Scissors]
            } else {
                vec![Weapon::Rock, Weapon::Paper]
            }
        } else {
            if self.num_rock < self.num_scissors {
                vec![Weapon::Scissors]
            } else if self.num_rock == self.num_scissors {
                vec![Weapon::Rock, Weapon::Scissors]
            } else {
                vec![Weapon::Rock]
            }
        }
    }
}

impl Home {
    pub async fn user_stats(app: &App, id: i32) -> Result<(User, Stats), StatusCode> {
        use crate::schema::metrics;
        use crate::schema::users;
        users::table
            .filter(users::id.eq(id))
            .inner_join(metrics::table)
            .select((User::as_select(), Stats::as_select()))
            .first::<(User, Stats)>(&mut app.connection().await?)
            .await
            .map_err(|e| {
                error!(target: "home::user_stats", "{:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })
    }

    pub async fn for_id(app: App, id: i32) -> Result<Home, StatusCode> {
        let (user, stats) = Home::user_stats(&app, id).await?;
        let invite = app.sessions.get_invite(id).await?;
        let invite_url = invite.url(&app.base)?;
        Ok(Home {
            username: user.username,
            invite: invite_url,
            num_schnicks: stats.num_schnicks,
            num_won: stats.num_won,
            score: 0,
            win_streak: (stats.current_winning_streak, stats.longest_winning_streak),
            lose_streak: (stats.current_losing_streak, stats.longest_losing_streak),
            children: stats.num_children,
            favorites: stats.favorites(),
        })
    }
}

/// The `/home` route.
pub async fn home(
    State(app): State<App>,
    cookies: CookieJar,
) -> Result<impl IntoResponse, StatusCode> {
    debug!(target: "home::home", "cookies={cookies:?}");
    let id = app.authenticate(&cookies).await?;
    Ok(Html(
        Home::for_id(app, id)
            .await?
            .render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}
