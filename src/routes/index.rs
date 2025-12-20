use axum::response::{IntoResponse, Redirect};
use axum_extra::extract::CookieJar;

use crate::auth::AUTHENTICATOR_COOKIE_NAME;

pub async fn index(cookies: CookieJar) -> impl IntoResponse {
    if cookies.get(AUTHENTICATOR_COOKIE_NAME).is_some() {
        Redirect::to("home")
    } else {
        Redirect::to("about")
    }
}