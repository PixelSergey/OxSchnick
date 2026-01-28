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
    recovery_link: &'a Url,
    colleges: &'a [(i32, String)]
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
    use crate::schema::{users, colleges};
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
    
    // Look up the college name from the database
    let college_id = college_value.unwrap_or(0);
    let college_name: String = colleges::table
        .select(colleges::college)
        .find(college_id)
        .first::<String>(
            &mut state
                .pool
                .get()
                .await
                .map_err(|_| Error::InternalServerError)?,
        )
        .await
        .map_err(|_| Error::InternalServerError)?;
    
    Graphs::send_update(crate::graphs::GraphUpdate::CollegeSet { id, college: college_name }, &state.graphs).await;
    let mut conn = state
        .pool
        .get()
        .await
        .map_err(|_| Error::InternalServerError)?;
    state.metrics.write().await.update(&mut conn).await?;
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
    let mut conn = state
        .pool
        .get()
        .await
        .map_err(|_| Error::InternalServerError)?;
    state.metrics.write().await.update(&mut conn).await?;
    Ok(Redirect::to("/settings"))
}

pub async fn settings(
    extract::State(state): extract::State<State>,
    Settings { username, college, .. }: Settings,
    auth::User(id): auth::User,
    AuthenticatorEntry { token, .. }: AuthenticatorEntry
) -> Result<impl IntoResponse> {
    use crate::schema::colleges;
    let mut recovery = state.base_url.join("recovery").map_err(|_| Error::InternalServerError)?;
    recovery.set_query(Some(&format!("id={id}&token={token}")));
    let colleges_list: Vec<(i32, String)> = colleges::table
        .select((colleges::id, colleges::college))
        .load::<(i32, String)>(
            &mut state
                .pool
                .get()
                .await
                .map_err(|_| Error::InternalServerError)?,
        )
        .await
        .map_err(|_| Error::InternalServerError)?;
    Ok(Html(
        SettingsTemplate {
            username_value: &username,
            college_value: college.as_ref(),
            recovery_link: &recovery,
            colleges: &colleges_list
        }
        .render()
        .map_err(|_| Error::InternalServerError)?,
    ))
}
