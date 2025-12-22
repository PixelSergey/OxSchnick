use askama::Template;
use axum::{
    Form, extract,
    response::{Html, IntoResponse, Redirect},
};
use diesel::QueryDsl;
use diesel_async::RunQueryDsl;
use serde::Deserialize;

use crate::{auth, error::{Error, Result}, state::State, users::Settings};

#[derive(Template)]
#[template(path = "settings.html")]
pub struct SettingsTemplate<'a> {
    username: &'a str,
    dect: Option<&'a str>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SettingsForm {
    username: String,
    dect: Option<String>,
}

pub async fn settings_submit(
    extract::State(state): extract::State<State>,
    auth::User(id): auth::User,
    Form(SettingsForm { username, mut dect }): Form<SettingsForm>,
) -> Result<impl IntoResponse> {
    use crate::schema::users;
    dect.take_if(|inner| inner.is_empty());
    let new = Settings { id, username, dect };
    diesel::update(users::table.find(id))
        .set(&new)
        .execute(
            &mut state
                .pool
                .get()
                .await
                .map_err(|_| Error::InternalServerError)?,
        )
        .await
        .map_err(|_| Error::InvalidSettings)?;
    Ok(Redirect::to("settings"))
}

pub async fn settings(
    Settings { username, dect, .. }: Settings,
) -> Result<impl IntoResponse> {
    Ok(Html(
        SettingsTemplate {
            username: &username,
            dect: dect.as_deref(),
        }
        .render()
        .map_err(|_| Error::InternalServerError)?,
    ))
}
