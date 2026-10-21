mod api;
mod app_shell;
mod components;
mod html_markdown;
mod models;
mod preferences;
mod router;
mod screen;
mod store;
mod theme;
#[cfg(test)]
mod testing;
mod topic_stream;

use crate::router::AppRouter;
use crate::store::AppStore;
use aimer::native::macos_windowing::MacosWindowing;
use aimer::quiver::window_attr::WindowAttr;
use aimer::router::Navigator;
use aimer::*;

// this is the entry point of the app
#[aimer::main]
fn main() {
    // String::type_id(;)

    AimerApp::new()
        .window(
            WindowAttr::new()
                .title("Rust Forum")
                .inner_size(1100, 780)
               .min_inner_size(900, 620),
        )
        .setup(|| {
            MacosWindowing::new()
                .titlebar_transparent(true)
                .title_hidden(true)
                .fullsize_content_view(true)
                .traffic_light_position(16.0, 14.0)
                .has_shadow(true)
                .accepts_first_mouse(true)
                .install();
        })
        .child(
            // take this
            Provider::new()
                .create(AppStore::default)
                .child(Navigator::new(AppRouter::Latest, |r| r.boxed())),
        )
        .run()
}
