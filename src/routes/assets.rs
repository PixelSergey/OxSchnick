use axum::{
    extract::Path,
    http::header::CONTENT_TYPE,
    response::IntoResponse,
};
use crate::error::{Error, Result};

macro_rules! serve_static {
    ( $name:expr, [ $( [ $path:literal, $file:expr, $type:literal ] ),* ]) => {
        match $name {
            $(
                $path => Ok(([(CONTENT_TYPE, $type)], &include_bytes!($file)[..])),
            )*
            _ => Err(Error::NotFound)
        }
    };
}

pub async fn assets(Path(file): Path<String>) -> Result<impl IntoResponse> {
    serve_static!(
        &file[..],
        [
            ["style.css", "../../assets/style.css", "text/css"],
            ["phone.svg", "../../assets/phone.svg", "image/svg+xml"],
            ["rock.svg", "../../assets/rock.svg", "image/svg+xml"],
            ["paper.svg", "../../assets/paper.svg", "image/svg+xml"],
            ["scissors.svg", "../../assets/scissors.svg", "image/svg+xml"],
            ["won.svg", "../../assets/won.svg", "image/svg+xml"],
            ["lost.svg", "../../assets/lost.svg", "image/svg+xml"],
            ["abort.svg", "../../assets/abort.svg", "image/svg+xml"],
            ["home.svg", "../../assets/home.svg", "image/svg+xml"],
            ["metrics.svg", "../../assets/metrics.svg", "image/svg+xml"],
            ["graphs.svg", "../../assets/graphs.svg", "image/svg+xml"],
            [
                "num_invites.svg",
                "../../assets/num_invites.svg",
                "image/svg+xml"
            ],
            [
                "num_schnicks.svg",
                "../../assets/num_schnicks.svg",
                "image/svg+xml"
            ],
            ["distance.svg", "../../assets/distance.svg", "image/svg+xml"],
            ["score.svg", "../../assets/score.svg", "image/svg+xml"],
            ["streak.svg", "../../assets/streak.svg", "image/svg+xml"],
            ["settings.svg", "../../assets/settings.svg", "image/svg+xml"],
            [
                "arrow_back.svg",
                "../../assets/arrow_back.svg",
                "image/svg+xml"
            ],
            [
                "arrow_right.svg",
                "../../assets/arrow_right.svg",
                "image/svg+xml"
            ]
        ]
    )
}
