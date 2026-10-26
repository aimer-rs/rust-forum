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
use aimer::quiver::window_attr::WindowAttr;
use aimer::router::Navigator;
use aimer::*;

// this is the entry point of the app
#[aimer::main]
fn main() {
    // String::type_id(;)

    AimerApp::new()
        // The settings, the sign-in and the profile open windows of their own:
        // closing the forum's window ends the app, and takes them with it.
        .exit_policy(ExitPolicy::MainWindowClosed)
        .window(
            WindowAttr::new()
                .title("Rust Forum")
                .inner_size(1100, 780)
               .min_inner_size(900, 620),
        )
        .setup(|| {
            // The windows the app opens later look the same: see `chrome`.
            components::windows::chrome().install();
        })
        .child(
            // take this
            Provider::new()
                .create(AppStore::default)
                .child(Navigator::new(AppRouter::Latest, |r| r.boxed())),
        )
        .run()
}
