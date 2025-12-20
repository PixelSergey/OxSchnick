use axum::{
    extract::Path,
    http::{StatusCode, header::CONTENT_TYPE},
    response::IntoResponse,
};

macro_rules! serve_static {
    ( $name:expr, [ $( [ $path:literal, $file:expr, $type:literal ] ),* ]) => {
        match $name {
            $(
                $path => Ok(([(CONTENT_TYPE, $type)], &include_bytes!($file)[..])),
            )*
            _ => Err(StatusCode::NOT_FOUND)
        }
    };
}

pub async fn assets(Path(file): Path<String>) -> Result<impl IntoResponse, StatusCode> {
    serve_static!(
        &file[..],
        [
            ["style.css", "../../assets/style.css", "text/css"],
            [
                "phone.svg",
                "../../assets/phone.svg",
                "image/svg+xml"
            ],
            ["rock.svg", "../../assets/schnick/rock.svg", "image/svg+xml"],
            ["paper.svg", "../../assets/schnick/paper.svg", "image/svg+xml"],
            ["scissors.svg", "../../assets/schnick/scissors.svg", "image/svg+xml"],
            ["won.svg", "../../assets/schnick/won.svg", "image/svg+xml"],
            ["lost.svg", "../../assets/schnick/lost.svg", "image/svg+xml"],
            ["abort.svg", "../../assets/schnick/abort.svg", "image/svg+xml"],
            ["home.svg", "../../assets/nav_bar/home.svg", "image/svg+xml"],
            [
                "metrics.svg",
                "../../assets/nav_bar/metrics.svg",
                "image/svg+xml"
            ],
            [
                "graphs.svg",
                "../../assets/nav_bar/graphs.svg",
                "image/svg+xml"
            ],
            ["num_invites.svg", "../../assets/metrics/num_invites.svg", "image/svg+xml"],
            ["num_schnicks.svg", "../../assets/metrics/num_schnicks.svg", "image/svg+xml"],
            ["distance.svg", "../../assets/metrics/distance.svg", "image/svg+xml"],
            ["score.svg", "../../assets/metrics/score.svg", "image/svg+xml"],
            ["streak.svg", "../../assets/metrics/streak.svg", "image/svg+xml"],
            ["settings.svg", "../../assets/nav_bar/settings.svg", "image/svg+xml"],
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
