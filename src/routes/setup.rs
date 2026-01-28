use askama::Template;
use axum::{
    Form, extract,
    response::{Html, IntoResponse, Redirect},
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::Deserialize;

use crate::{
    auth::self, error::{Error, Result}, graphs::Graphs, state::State
};

#[derive(Template)]
#[template(path = "setup.html")]
pub struct SetupTemplate<'a> {
    colleges: &'a [(i32, String)]
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetupForm {
    college_value: Option<i32>,
    username_value: String,
}

pub async fn setup_set(
    extract::State(state): extract::State<State>,
    auth::User(id): auth::User,
    Form(SetupForm { college_value, username_value }): Form<SetupForm>,
) -> Result<impl IntoResponse> {
    use crate::schema::{users, colleges};
    use crate::schema::users::{college, username};
    
    // Validate college value
    if let Some(ref d) = college_value {
        if !(*d >= 0 && *d <= 43) {
            return Err(Error::InvalidCollege);
        }
    }
    
    // Update both college and username simultaneously
    diesel::update(users::table.find(id))
        .set((college.eq(&college_value), username.eq(&username_value)))
        .execute(
            &mut state
                .pool
                .get()
                .await
                .map_err(|_| Error::InternalServerError)?,
        )
        .await
        .map_err(|_| Error::InvalidSetup)?;
    
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
    
    // Send both graph updates
    Graphs::send_update(crate::graphs::GraphUpdate::CollegeSet { id, college: college_name }, &state.graphs).await;
    Graphs::send_update(crate::graphs::GraphUpdate::UserRenamed { id, name: username_value }, &state.graphs).await;
    
    Ok(Redirect::to("/"))
}

pub async fn setup(
    extract::State(state): extract::State<State>,
) -> Result<impl IntoResponse> {
    use crate::schema::colleges;
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
        SetupTemplate {
            colleges: &colleges_list
        }
        .render()
        .map_err(|_| Error::InternalServerError)?,
    ))
}

