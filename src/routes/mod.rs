mod assets;
mod home;
mod invite;
mod schnick;
mod settings;
mod about;
mod index;
mod graphs;
mod metrics;

pub use assets::assets;
pub use home::{home, home_sse};
pub use invite::invite;
pub use schnick::{schnick, schnick_submit, schnick_abort, schnick_sse};
pub use settings::{settings, settings_submit};
pub use about::{about, imprint};
pub use index::index;
pub use graphs::graphs;
pub use metrics::metrics;