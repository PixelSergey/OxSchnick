use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::{CookieJar, cookie::Cookie};
use diesel::{dsl::insert_into, prelude::*};
use diesel_async::{AsyncPgConnection, RunQueryDsl, pooled_connection::bb8::PooledConnection};
use serde::Deserialize;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::{Server, schnick::OngoingSchnick};

#[derive(Debug, Clone, HasQuery)]
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

pub async fn check_token(
    id: i32,
    token: &str,
    conn: &mut PooledConnection<'_, AsyncPgConnection>,
) -> Result<(), StatusCode> {
    use crate::schema::users;
    User::query()
        .filter(users::id.eq(id))
        .filter(users::token.eq(token))
        .first(conn)
        .await
        .map_err(|_| StatusCode::FORBIDDEN)?;
    Ok(())
}

pub async fn create_schnick(
    inviter: i32,
    invitee: i32,
    matches: Arc<RwLock<HashMap<i32, Arc<Mutex<OngoingSchnick>>>>>,
) -> Result<(), StatusCode> {
    let mut matches = matches.write().await;
    if matches.get(&inviter).is_some() || matches.get(&invitee).is_some() {
        return Err(StatusCode::CONFLICT);
    };
    let new = Arc::new(Mutex::new(OngoingSchnick {
        inviter,
        invitee,
        inviter_selection: None,
        invitee_selection: None,
    }));
    matches.insert(inviter, Arc::clone(&new));
    matches.insert(invitee, new);
    Ok(())
}

pub async fn check_invite(
    id: i32,
    invite: &str,
    conn: &mut PooledConnection<'_, AsyncPgConnection>,
) -> Result<(), StatusCode> {
    use crate::schema::users;
    users::table
        .filter(users::id.eq(id))
        .filter(users::invite.eq(invite))
        .first::<User>(conn)
        .await
        .map_err(|_| StatusCode::FORBIDDEN)
        .map(|_| ())
}

pub async fn renew_invite(
    invite: &Invite,
    conn: &mut PooledConnection<'_, AsyncPgConnection>,
) -> Result<(), StatusCode> {
    use crate::schema::users;
    let new = Uuid::new_v4().to_string();
    diesel::update(invite)
        .set(users::invite.eq(new))
        .execute(conn)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        .map(|_| ())
}

#[derive(Debug, Clone, Deserialize, HasQuery, Identifiable)]
#[diesel(table_name = crate::schema::users)]

pub struct Invite {
    pub id: i32,
    pub invite: String,
}

pub async fn invite(
    State(Server(pool, schnicks)): State<Server>,
    mut cookies: CookieJar,
    Query(invite): Query<Invite>,
) -> Result<(CookieJar, impl IntoResponse), StatusCode> {
    let mut conn = pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    check_invite(invite.id, &invite.invite, &mut conn).await?;
    let id = match (cookies.get("id"), cookies.get("token")) {
        (Some(id), Some(token)) => {
            let (id, token) = (
                id.value()
                    .parse::<i32>()
                    .map_err(|_| StatusCode::FORBIDDEN)?
                    .clone(),
                token.value().to_string(),
            );
            check_token(id, &token, &mut conn).await?;
            id
        }
        _ => {
            let (id, token) = create_user(invite.id, &mut conn).await?;
            cookies = cookies
                .add(Cookie::new("id", id.to_string()))
                .add(Cookie::new("token", token.clone()));
            id
        }
    };
    create_schnick(id, invite.id, schnicks).await?;
    renew_invite(&invite, &mut conn).await?;
    Ok((cookies, Redirect::temporary("match")))
}
