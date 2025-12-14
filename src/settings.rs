use askama::Template;
use axum::{Form, extract::State, http::StatusCode, response::{Html, IntoResponse}};
use axum_extra::extract::CookieJar;
use diesel::{prelude::*, update};
use diesel_async::RunQueryDsl;
use serde::Deserialize;

use crate::app::App;

#[derive(Debug, Clone, HasQuery, Identifiable, Template, QueryableByName)]
#[template(path="settings.html")]
#[diesel(table_name=crate::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    id: i32,
    username: Option<String>,
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
    Ok(Html(user.render().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?))
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
        .returning((users::id, users::username))
        .get_result::<User>(&mut app.connection().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
        .await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Html(user.render().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?))
}