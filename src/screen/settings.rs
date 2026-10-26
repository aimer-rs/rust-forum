mod appearance;
pub(crate) mod widgets;

pub use appearance::AppearanceScreen;

use crate::api::session;
use crate::app_shell::{has_windows, is_mobile};
use crate::components::avatar::round_avatar;
use crate::components::windows;
use crate::router::AppRouter;
use crate::screen::settings::widgets::{
    SettingsRow, Tile, Trailing, footnote, group, heading, page,
};
use crate::store::AppStore;
use crate::theme::ForumTheme;
use aimer::router::NavigatorController;
use aimer::style::{Theme, ThemeMode};
use aimer::{
    AnyWidget, BuildContext, Icons, ProviderContext, ProviderHandle, State, StateUpdater,
    StatefulWidget, Widget,
};

/// The settings: what can be changed, in groups. Each group is a heading over a
/// card of rows; a row either says something or opens a page of its own.
///
/// - **Appearance**: the theme, and the tabs of the bar of a narrow window; see
///   [`AppearanceScreen`].
/// - **Account**: who is signed in, or the way to sign in.
/// - **Security**: signing out of this device, and where the sign-in is kept.
#[derive(Clone, StatefulWidget)]
pub struct SettingsScreen;

pub struct SettingsState {
    /// Whether the member has pressed "Sign out" and been asked to press it again.
    /// Signing in again needs a code from their inbox, so it is not done on a
    /// single press.
    confirming_sign_out: bool,
    updater: StateUpdater<Self>,
}

impl StatefulWidget for SettingsScreen {
    type State = SettingsState;

    fn create_state(self) -> Self::State {
        SettingsState {
            confirming_sign_out: false,
            updater: StateUpdater::new(),
        }
    }
}

impl SettingsState {
    /// Asks to be pressed again; the second press signs out.
    fn press_sign_out(&mut self, store: &ProviderHandle<AppStore>) {
        if !self.confirming_sign_out {
            self.confirming_sign_out = true;
            return;
        }
        self.confirming_sign_out = false;
        session::end(store);
    }
}

fn theme_mode_name(mode: ThemeMode) -> &'static str {
    match mode {
        ThemeMode::System => "System",
        ThemeMode::Light => "Light",
        ThemeMode::Dark => "Dark",
    }
}

impl State<SettingsScreen> for SettingsState {
    fn init_state(&mut self, updater: StateUpdater<Self>) {
        self.updater = updater;
    }

    fn adopt_config_from(&mut self, _new: Self) {}

    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme = *ForumTheme::of(ctx);
        let navigator = NavigatorController::<AppRouter>::of(ctx);
        let store = ProviderHandle::<AppStore>::of(ctx);
        let user = ctx.select::<AppStore, Option<String>>(|s| s.current_user.clone());
        let mode = ctx.select::<AppStore, ThemeMode>(|s| s.theme_mode);
        let tab_count = ctx.select::<AppStore, usize>(|s| s.navbar.len());

        // On a computer the account opens in a window of its own; the sign-in is
        // the next page of the window the settings are in.
        let windowed = has_windows(ctx);
        let open_appearance = {
            let navigator = navigator.clone();
            move || navigator.push(AppRouter::SettingsAppearance)
        };

        let mut appearance = vec![
            SettingsRow::new(Tile::Lucide(Icons::PALETTE), "Theme")
                .value(theme_mode_name(mode))
                .trailing(Trailing::Chevron)
                .on_press(open_appearance.clone())
                .build(theme),
        ];
        if is_mobile(ctx) {
            appearance.push(
                SettingsRow::new(Tile::Lucide(Icons::PANEL_BOTTOM), "Navigation bar")
                    .value(format!("{tab_count} tabs"))
                    .trailing(Trailing::Chevron)
                    .on_press(open_appearance)
                    .build(theme),
            );
        }

        let account = match &user {
            Some(name) => {
                let navigator = navigator.clone();
                let store = store.clone();
                let avatar = name.clone();
                SettingsRow::new(
                    Tile::Custom(Box::new(move |_| round_avatar(theme, "", &avatar, 24.0))),
                    name.clone(),
                )
                    .trailing(Trailing::Chevron)
                    .on_press(move || {
                        if windowed {
                            windows::open_profile(&store);
                        } else {
                            navigator.set_route(AppRouter::Account);
                        }
                    })
                    .build(theme)
            }
            None => {
                let navigator = navigator.clone();
                SettingsRow::new(Tile::Lucide(Icons::USER), "Sign in")
                    .detail("Join in on the forum")
                    .trailing(Trailing::Chevron)
                    .on_press(move || navigator.push(AppRouter::Login))
                    .build(theme)
            }
        };

        let mut security: Vec<AnyWidget> = Vec::new();
        if user.is_some() {
            let updater = self.updater;
            let store = store.clone();
            let (title, detail) = if self.confirming_sign_out {
                ("Press again to sign out", "You will need a new code to sign back in.")
            } else {
                ("Sign out", "Remove your sign-in from this device.")
            };
            security.push(
                SettingsRow::new(Tile::Lucide(Icons::LOG_OUT), title)
                    .detail(detail)
                    .accent()
                    .on_press(move || {
                        let store = store.clone();
                        updater.set_state(move |state| state.press_sign_out(&store));
                    })
                    .build(theme),
            );
        }
        security.push(
            SettingsRow::new(Tile::Lucide(Icons::SHIELD_CHECK), "Sign-in storage")
                .detail("Your sign-in is kept in this device's secure storage.")
                .build(theme),
        );

        page(
            ctx,
            theme,
            "scrollable-settings",
            "Settings",
            vec![
                heading(theme, "Appearance"),
                group(appearance),
                heading(theme, "Account"),
                group(vec![account]),
                heading(theme, "Security"),
                group(security),
                footnote(theme, "More settings will arrive here as the app grows."),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::widgets::testing;
    use aimer::quiver::winit::dpi::PhysicalSize;
    use aimer::router::Navigator;
    use aimer::{AimerApp, BuildContext, HeadlessOptions, Provider, StatelessWidget};

    thread_local! {
        static STORE: std::cell::RefCell<Option<ProviderHandle<AppStore>>> = const { std::cell::RefCell::new(None) };
    }

    /// The app's page: it keeps hold of the store and shows the route.
    #[derive(Clone, StatelessWidget)]
    struct Probe(AppRouter);

    impl StatelessWidget for Probe {
        fn build(&self, ctx: &BuildContext) -> impl Widget {
            STORE.with(|store| *store.borrow_mut() = Some(ProviderHandle::<AppStore>::of(ctx)));
            self.0.clone().boxed()
        }
    }

    fn signed_in_as() -> Option<String> {
        STORE.with(|store| store.borrow().as_ref().expect("the page was built").read().current_user.clone())
    }

    #[test]
    fn signing_out_takes_a_second_press() {
        let mut store = AppStore::default();
        store.sign_in("domma".to_owned());
        crate::testing::hold();
        let mut app = AimerApp::start_headless_with(
            Provider::new().create(move || store.clone()).child(Navigator::new(AppRouter::Settings, |r| Probe(r).boxed())),
            HeadlessOptions { size: PhysicalSize::new(390, 900), scale_factor: 1.0 },
        );

        testing::settle(&mut app);
        assert_eq!(signed_in_as().as_deref(), Some("domma"));

        // The "Sign out" row, the first of the security group.
        testing::tap(&mut app, 1, (200.0, 495.0));
        testing::settle(&mut app);
        assert_eq!(signed_in_as().as_deref(), Some("domma"), "one press signed out");

        testing::tap(&mut app, 2, (200.0, 495.0));
        testing::settle(&mut app);
        assert_eq!(signed_in_as(), None, "the second press did not sign out");
    }
}
