use askama::Template;
use axum::{
    Form, extract,
    response::{Html, IntoResponse, Redirect},
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::Deserialize;
use url::Url;

use crate::{
    auth::{self, AuthenticatorEntry}, error::{Error, Result}, graphs::Graphs, state::State, users::Settings
};

#[derive(Template)]
#[template(path = "settings.html")]
pub struct SettingsTemplate<'a> {
    username_value: &'a str,
    college_value: Option<&'a i32>,
    recovery_link: &'a Url
}

#[derive(Debug, Clone, Deserialize)]
pub struct CollegeForm {
    college_value: Option<i32>,
}

pub async fn settings_college(
    extract::State(state): extract::State<State>,
    auth::User(id): auth::User,
    Form(CollegeForm { college_value }): Form<CollegeForm>,
) -> Result<impl IntoResponse> {
    use crate::schema::users;
    use crate::schema::users::college;
    if let Some(ref d) = college_value {
        if !(*d >= 0 && *d <= 43) {
            return Err(Error::InvalidCollege);
        }
    }
    diesel::update(users::table.find(id))
        .set(college.eq(&college_value))
        .execute(
            &mut state
                .pool
                .get()
                .await
                .map_err(|_| Error::InternalServerError)?,
        )
        .await
        .map_err(|_| Error::InvalidCollege)?;
    Graphs::send_update(crate::graphs::GraphUpdate::CollegeSet { id, college: college_value }, &state.graphs).await;
    Ok(Redirect::to("/settings"))
}

#[derive(Debug, Clone, Deserialize)]
pub struct UsernameForm {
    username_value: String,
}

pub async fn settings_username(
    extract::State(state): extract::State<State>,
    auth::User(id): auth::User,
    Form(UsernameForm { username_value }): Form<UsernameForm>,
) -> Result<impl IntoResponse> {
    use crate::schema::users;
    use crate::schema::users::username;
    diesel::update(users::table.find(id))
        .set(username.eq(&username_value))
        .execute(
            &mut state
                .pool
                .get()
                .await
                .map_err(|_| Error::InternalServerError)?,
        )
        .await
        .map_err(|_| Error::DuplicateUsername)?;
    Graphs::send_update(crate::graphs::GraphUpdate::UserRenamed { id, name: username_value }, &state.graphs).await;
    Ok(Redirect::to("/settings"))
}

pub async fn settings(
    extract::State(state): extract::State<State>,
    Settings { username, college, .. }: Settings,
    auth::User(id): auth::User,
    AuthenticatorEntry { token, .. }: AuthenticatorEntry
) -> Result<impl IntoResponse> {
    let mut recovery = state.base_url.join("recovery").map_err(|_| Error::InternalServerError)?;
    recovery.set_query(Some(&format!("id={id}&token={token}")));
    Ok(Html(
        SettingsTemplate {
            username_value: &username,
            college_value: college.as_ref(),
            recovery_link: &recovery
        }
        .render()
        .map_err(|_| Error::InternalServerError)?,
    ))
}
