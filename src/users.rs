use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::{CookieJar, cookie::Cookie};
use diesel::{dsl::insert_into, prelude::*};
use diesel_async::{
    AsyncPgConnection, RunQueryDsl,
    pooled_connection::bb8::{Pool, PooledConnection},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::Server;

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub id: i32,
    pub username: Option<String>,
    pub parent: Option<i32>,
    pub token: String,
    pub invite: String,
}

#[derive(Debug, Clone, Insertable, QueryableByName)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct InsertUser {
    pub parent: Option<i32>,
    pub token: String,
    pub invite: String,
}

pub async fn create_user(
    parent: i32,
    conn: &mut PooledConnection<'_, AsyncPgConnection>,
) -> Result<(i32, String), StatusCode> {
    use crate::schema::users;
    let new = InsertUser {
        parent: Some(parent),
        token: Uuid::new_v4().to_string(),
        invite: Uuid::new_v4().to_string(),
    };
    insert_into(users::table)
        .values(&new)
        .returning((users::id, users::token))
        .get_result(conn)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[derive(Debug, Clone, Deserialize, HasQuery, Identifiable)]
#[diesel(table_name = crate::schema::users)]

pub struct Invite {
    pub id: i32,
    pub invite: String,
}

impl Invite {
    pub async fn get(
        id: i32,
        invite: String,
        conn: &mut PooledConnection<'_, AsyncPgConnection>,
    ) -> Result<Self, StatusCode> {
        use crate::schema::users;
        Invite::query()
            .filter(users::id.eq(id))
            .filter(users::invite.eq(invite))
            .first(conn)
            .await
            .map_err(|_| StatusCode::NOT_FOUND)
    }

    pub async fn renew(
        &self,
        conn: &mut PooledConnection<'_, AsyncPgConnection>,
    ) -> Result<(), StatusCode> {
        use crate::schema::users;
        let new = Uuid::new_v4().to_string();
        diesel::update(self)
            .set(users::invite.eq(new))
            .execute(conn)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
            .map(|_| ())
    }
}

pub async fn invite(
    State(Server(pool, matches)): State<Server>,
    cookies: CookieJar,
    Query(invite): Query<Invite>,
) -> Result<(CookieJar, impl IntoResponse), StatusCode> {
    let mut conn = pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let (id, token) = match (cookies.get("id"), cookies.get("token")) {
        (Some(id), Some(token)) => (id.value().parse::<i32>().map_err(|_| StatusCode::BAD_REQUEST)?, token.value()),
        _ => {
            let invite = Invite::get(invite.id, invite.invite, &mut conn).await?;
            let (id, token) = create_user(invite.id, &mut conn).await?;
            return Ok((
                cookies
                    .add(Cookie::new("id", id.to_string()))
                    .add(Cookie::new("token", token)),
                Redirect::temporary(""),
            ));
        }
    };
    Ok((cookies, "hello world"))
}
