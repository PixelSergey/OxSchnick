mod about;
mod assets;
mod graphs;
mod home;
mod index;
mod invite;
mod metrics;
mod schnick;
mod settings;

pub use about::{about, imprint};
pub use assets::assets;
pub use graphs::{graphs, graphs_graph, graphs_tree, graphs_global, graphs_cache, graphs_sse};
pub use home::{home, home_invite, home_sse};
pub use index::index;
pub use invite::{invite, invite_accept};
pub use metrics::metrics;
pub use schnick::{schnick, schnick_abort, schnick_sse, schnick_submit};
pub use settings::{settings, settings_dect, settings_username};
