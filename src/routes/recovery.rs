use axum::{extract::Query, response::{IntoResponse, Redirect}};
use axum_extra::extract::{CookieJar, cookie::{Cookie, SameSite}};

use crate::{auth::{self, AUTHENTICATOR_COOKIE_NAME, Authenticated}, error::{Error, Result}};

pub async fn recovery(
    cookies: CookieJar,
    Query(authenticated): Query<Authenticated>
) -> Result<impl IntoResponse> {
    let mut cookie = Cookie::new(
        AUTHENTICATOR_COOKIE_NAME,
        serde_json::to_string(&Authenticated {
            id: authenticated.id,
            token: authenticated.token,
        })
        .map_err(|_| Error::InternalServerError)?,
    );
    cookie.make_permanent();
    cookie.set_same_site(SameSite::Strict);
    cookie.set_path("/");
    #[cfg(not(debug_assertions))]
    cookie.set_secure(Some(true));
    Ok((cookies.add(cookie), Redirect::to("/")))
}