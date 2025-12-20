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
                "phone_receiver.svg",
                "../../assets/phone_receiver.svg",
                "image/svg+xml"
            ],
            ["rock.svg", "../../assets/rock.svg", "image/svg+xml"],
            ["paper.svg", "../../assets/paper.svg", "image/svg+xml"],
            ["scissors.svg", "../../assets/scissors.svg", "image/svg+xml"],
            ["won.svg", "../../assets/won.svg", "image/svg+xml"],
            ["lost.svg", "../../assets/lost.svg", "image/svg+xml"],
            ["abort.svg", "../../assets/abort.svg", "image/svg+xml"],
            ["adult.svg", "../../assets/adult.svg", "image/svg+xml"],
            [
                "hash_char.svg",
                "../../assets/hash_char.svg",
                "image/svg+xml"
            ],
            [
                "spider_web.svg",
                "../../assets/spider_web.svg",
                "image/svg+xml"
            ],
            ["children.svg", "../../assets/children.svg", "image/svg+xml"],
            ["distance.svg", "../../assets/distance.svg", "image/svg+xml"],
            ["score.svg", "../../assets/score.svg", "image/svg+xml"],
            ["streak.svg", "../../assets/streak.svg", "image/svg+xml"],
            ["wrench.svg", "../../assets/wrench.svg", "image/svg+xml"],
            [
                "arrow_right.svg",
                "../../assets/arrow_right.svg",
                "image/svg+xml"
            ]
        ]
    )
}
