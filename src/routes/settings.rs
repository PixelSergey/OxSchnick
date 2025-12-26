use askama::Template;
use axum::{
    Form, extract,
    response::{Html, IntoResponse, Redirect},
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::Deserialize;

use crate::{
    auth, error::{Error, Result}, graphs::Graphs, state::State, users::Settings
};

#[derive(Template)]
#[template(path = "settings.html")]
pub struct SettingsTemplate<'a> {
    username_value: &'a str,
    dect_value: Option<&'a str>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DectForm {
    dect_value: Option<String>,
}

pub async fn settings_dect(
    extract::State(state): extract::State<State>,
    auth::User(id): auth::User,
    Form(DectForm { mut dect_value }): Form<DectForm>,
) -> Result<impl IntoResponse> {
    use crate::schema::users;
    use crate::schema::users::dect;
    dect_value.take_if(|inner| inner.is_empty());
    if let Some(ref d) = dect_value {
        if d.len() != 4 || !d.chars().all(|c| c.is_ascii_digit()) {
            return Err(Error::InvalidDect);
        }
    }
    diesel::update(users::table.find(id))
        .set(dect.eq(dect_value))
        .execute(
            &mut state
                .pool
                .get()
                .await
                .map_err(|_| Error::InternalServerError)?,
        )
        .await
        .map_err(|_| Error::InvalidDect)?;
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

pub async fn settings(Settings { username, dect, .. }: Settings) -> Result<impl IntoResponse> {
    Ok(Html(
        SettingsTemplate {
            username_value: &username,
            dect_value: dect.as_deref(),
        }
        .render()
        .map_err(|_| Error::InternalServerError)?,
    ))
}
