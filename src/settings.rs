use askama::Template;
use axum::{
    Form,
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
};
use axum_extra::extract::CookieJar;
use diesel::{prelude::*, update};
use diesel_async::RunQueryDsl;
use serde::Deserialize;

use crate::app::App;

#[derive(Debug, Clone, HasQuery, Identifiable, Template, QueryableByName)]
#[template(path = "settings.html")]
#[diesel(table_name=crate::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub id: i32,
    pub username: String,
    pub dect: Option<String>,
}

pub async fn settings(
    State(app): State<App>,
    cookies: CookieJar,
) -> Result<impl IntoResponse, StatusCode> {
    use crate::schema::users;
    let id = app.authenticate(&cookies).await?;
    let user = User::query()
        .filter(users::id.eq(id))
        .first(&mut app.connection().await?)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Html(
        user.render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

#[derive(Debug, Clone, Deserialize)]
pub struct UsernameForm {
    username: String,
}

pub async fn settings_username(
    State(app): State<App>,
    cookies: CookieJar,
    Form(data): Form<UsernameForm>,
) -> Result<impl IntoResponse, StatusCode> {
    use crate::schema::users;
    let id = app.authenticate(&cookies).await?;
    let user = update(users::table)
        .filter(users::id.eq(id))
        .set(users::username.eq(data.username))
        .returning((users::id, users::username, users::dect))
        .get_result::<User>(
            &mut app
                .connection()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Html(
        user.render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

#[derive(Debug, Clone, Deserialize)]
pub struct DectForm {
    dect: String,
}

pub async fn settings_dect(
    State(app): State<App>,
    cookies: CookieJar,
    Form(data): Form<DectForm>,
) -> Result<impl IntoResponse, StatusCode> {
    use crate::schema::users;
    let id = app.authenticate(&cookies).await?;
    let dect_value = if data.dect.is_empty() {
        None
    } else {
        Some(data.dect)
    };
    let user = update(users::table)
        .filter(users::id.eq(id))
        .set(users::dect.eq(dect_value))
        .returning((users::id, users::username, users::dect))
        .get_result::<User>(
            &mut app
                .connection()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Html(
        user.render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

pub async fn settings_about() -> Result<impl IntoResponse, StatusCode> {
    Ok(Html(include_str!("../templates/about_us.html")))
}

pub async fn settings_imprint() -> Result<impl IntoResponse, StatusCode> {
    Ok(Html(include_str!("../templates/imprint.html")))
}
