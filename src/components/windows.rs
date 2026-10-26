//! Settings, signing in and the member's profile on a computer: windows of their
//! own, beside the app's, rather than pages of it.
//!
//! The windows are declared by [`WindowsLayer`] and shown while the store says they
//! are open (see [`Windows`](crate::store::Windows)), so any part of the app can ask
//! for one with [`open_settings`], [`open_login`] or [`open_profile`]. A window the
//! member closes themselves tells the store, so it can be asked for again. Each has
//! the app's theme and store.
//!
//! The settings and the sign-in are pages of one window, the dialog, with a
//! navigator of its own. It has no title bar: its pages carry a button to go back
//! and one to close it.

use crate::components::general_button::GeneralButton;
use crate::router::AppRouter;
use crate::screen::account::AccountScreen;
use crate::screen::login::{EmailCodeScreen, LoginScreen, PasswordScreen};
use crate::screen::settings::{AppearanceScreen, SettingsScreen};
use crate::store::{AppStore, DialogPage, Windows};
use crate::theme::ForumTheme;
use aimer::native::macos_windowing::MacosWindowing;
use aimer::router::{Navigator, NavigatorController};
use aimer::style::{LayoutSpacing, TextAlign, TextStyle, Theme};
use aimer::{
    Align, Alignment, AnyWidget, BuildContext, Container, Dimension, ProviderContext,
    ProviderHandle, Stack, StatelessWidget, Text, Widget, Window, WindowAttr, ZeroSizedBox,
};
use std::cell::{Cell, RefCell};

/// Where a screen is shown, which says how much room it leaves at its top: a page
/// leaves room for the bar over it, a window of its own for the buttons in its
/// title bar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Placement {
    #[default]
    Page,
    Window,
}

impl Placement {
    /// The key a screen's scroll view has here: windows share what is remembered
    /// by key, so two views of one screen would otherwise share a scroll position.
    pub fn key(self, key: &'static str) -> String {
        match self {
            Self::Page => key.to_owned(),
            Self::Window => format!("window-{key}"),
        }
    }
}

/// The windows are as wide as a window gets to be one that lays a page out for a
/// computer rather than a phone: the narrowest is the width the sidebar layout has.
const DIALOG_SIZE: (u32, u32) = (640, 760);
const PROFILE_SIZE: (u32, u32) = (640, 800);
const MIN_SIZE: (u32, u32) = (600, 520);
/// Where the buttons over a page of the dialog sit, from its edges.
const BAR_INSET: u32 = 16;
const BAR_TOP: u32 = 12;
const BAR_BUTTON_WIDTH: f32 = 84.0;

/// How the profile window looks: the same as the app's own, with the title bar
/// drawn over the page and no title in it. Set before a window is created, as
/// that is when AppKit takes it.
pub fn chrome() -> MacosWindowing {
    MacosWindowing::new()
        .titlebar_transparent(true)
        .title_hidden(true)
        .fullsize_content_view(true)
        .traffic_light_position(16.0, 14.0)
        .has_shadow(true)
        .accepts_first_mouse(true)
}

/// How the dialog looks: no title bar to take hold of it by, so it moves by
/// being dragged from its background.
fn dialog_chrome() -> MacosWindowing {
    MacosWindowing::new()
        .movable_by_window_background(true)
        .has_shadow(true)
        .accepts_first_mouse(true)
}

thread_local! {
    /// The navigator of the dialog while it is open, for sending an open dialog to
    /// another page.
    static DIALOG_NAV: RefCell<Option<NavigatorController<AppRouter>>> = const { RefCell::new(None) };
    /// Which windows the last build of the layer had open.
    static OPEN: Cell<(bool, bool)> = const { Cell::new((false, false)) };
}

/// Opens the dialog on `page`; a dialog already open goes to it instead.
fn open_dialog(store: &ProviderHandle<AppStore>, page: DialogPage, route: AppRouter) {
    if store.read().windows.dialog.is_some() {
        if let Some(navigator) = DIALOG_NAV.with(|nav| nav.borrow().clone()) {
            navigator.set_route(route);
            return;
        }
    }
    store.update(|store| store.windows.dialog = Some(page));
}

/// Opens the window of the settings.
pub fn open_settings(store: &ProviderHandle<AppStore>) {
    open_dialog(store, DialogPage::Settings, AppRouter::Settings);
}

/// Opens the window to sign in from.
pub fn open_login(store: &ProviderHandle<AppStore>) {
    open_dialog(store, DialogPage::Login, AppRouter::Login);
}

/// Opens the window with the member's own profile.
pub fn open_profile(store: &ProviderHandle<AppStore>) {
    store.update(|store| store.windows.profile = true);
}

/// A window the store opens and closes: `open` says whether it is on screen, and
/// `closed` is what the store does when the member closes it themselves.
fn declared<W: Widget + 'static>(
    store: &ProviderHandle<AppStore>,
    attr: WindowAttr,
    open: bool,
    closed: fn(&mut Windows),
    content: impl Fn(&BuildContext) -> W + 'static,
) -> AnyWidget {
    let store = store.clone();
    Window::new()
        .attr(attr.min_inner_size(MIN_SIZE.0, MIN_SIZE.1))
        .open(open)
        .on_close(move || store.update(|store| closed(&mut store.windows)))
        .builder(content)
        .boxed()
}

fn dialog_attr() -> WindowAttr {
    WindowAttr::new()
        .title("Settings")
        .inner_size(DIALOG_SIZE.0, DIALOG_SIZE.1)
        .decorations(false)
}

fn profile_attr() -> WindowAttr {
    WindowAttr::new()
        .title("Profile")
        .inner_size(PROFILE_SIZE.0, PROFILE_SIZE.1)
}

/// Declares the windows of the app. It paints nothing and takes no room.
#[derive(Clone, StatelessWidget)]
pub struct WindowsLayer;

impl StatelessWidget for WindowsLayer {
    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let store = ProviderHandle::<AppStore>::of(ctx);
        let open = ctx.select::<AppStore, Windows>(|store| store.windows);
        let signed_in = ctx.select::<AppStore, bool>(|store| store.current_user.is_some());

        let dialog = open.dialog;
        let profile = open.profile && signed_in;
        // The platform takes the look of the next window it creates from here, so
        // it is set for the one asked for below.
        let (was_dialog, was_profile) = OPEN.with(|open| open.replace((dialog.is_some(), profile)));
        if profile && !was_profile {
            chrome().install();
        }
        if dialog.is_some() && !was_dialog {
            dialog_chrome().install();
        }

        // Both are always in the tree, so each keeps its place: a window is closed
        // by being declared shut, not by being left out.
        Stack::new().children([
            declared(
                &store,
                dialog_attr(),
                dialog.is_some(),
                |w| w.dialog = None,
                move |_ctx| {
                    let first = match dialog {
                        Some(DialogPage::Login) => AppRouter::Login,
                        _ => AppRouter::Settings,
                    };
                    Navigator::new(first, |route| DialogRoute(route).boxed())
                },
            ),
            declared(
                &store,
                profile_attr(),
                profile,
                |w| w.profile = false,
                |_ctx| AccountScreen::new(Placement::Window),
            ),
        ])
    }
}

/// A page of the dialog: the page of its route, with the buttons that go back
/// over the ones pushed on the first and that close the window.
///
/// Signing in sends a navigator to the account page; here that is the settings
/// again, where the member started from.
#[derive(Clone, StatelessWidget)]
struct DialogRoute(AppRouter);

impl StatelessWidget for DialogRoute {
    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let navigator = NavigatorController::<AppRouter>::of(ctx);
        DIALOG_NAV.with(|nav| *nav.borrow_mut() = Some(navigator.clone()));
        let page: AnyWidget = match &self.0 {
            AppRouter::Settings | AppRouter::Account => SettingsScreen.boxed(),
            AppRouter::SettingsAppearance => AppearanceScreen.boxed(),
            AppRouter::Login => LoginScreen::window().boxed(),
            AppRouter::LoginEmail => EmailCodeScreen.boxed(),
            AppRouter::LoginPassword => PasswordScreen.boxed(),
            _ => return blank(ctx),
        };
        with_buttons(ctx, page, navigator)
    }
}

fn blank(ctx: &BuildContext) -> AnyWidget {
    let theme = *ForumTheme::of(ctx);
    Container::new()
        .color(theme.background_color())
        .child(ZeroSizedBox)
        .boxed()
}

/// `page` with the button that goes back a page, when there is one, over its top
/// left, and the button that closes the window over its top right.
fn with_buttons(
    ctx: &BuildContext,
    page: AnyWidget,
    navigator: NavigatorController<AppRouter>,
) -> AnyWidget {
    let theme = *ForumTheme::of(ctx);
    let store = ProviderHandle::<AppStore>::of(ctx);
    let mut layers = vec![page];
    if navigator.can_pop() {
        layers.push(
            Align::new().alignment(Alignment::TopLeft).box_child(
                Container::new()
                    .padding(LayoutSpacing::new().top(BAR_TOP).left(BAR_INSET))
                    .child(button(theme, "Back", move || {
                        if navigator.can_pop() {
                            navigator.pop();
                        }
                    })),
            ),
        );
    }
    layers.push(
        Align::new().alignment(Alignment::TopRight).box_child(
            Container::new()
                .height(50)
                .height(50)
                .padding(LayoutSpacing::new().top(BAR_TOP).right(BAR_INSET))
                .child(button(theme, "Close", move || {
                    store.update(|store| store.windows.dialog = None)
                })),
        ),
    );
    Stack::new().children(layers).boxed()
}

/// A button of the app's own kind, labelled `label`.
fn button(theme: ForumTheme, label: &'static str, on_press: impl Fn() + 'static) -> AnyWidget {
    GeneralButton::with_child(
        Container::new()
            .padding(LayoutSpacing::new().top(8).bottom(8))
            .child(
                Text::new(label)
                    .text_align(TextAlign::MidCenter)
                    .text_style(
                        TextStyle::new()
                            .font_size(16)
                            .color(theme.on_background_color()),
                    ),
            ),
        on_press,
    )
    .width(BAR_BUTTON_WIDTH)
    .boxed()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screen::settings::widgets::testing;
    use aimer::quiver::windows::host::{Request, WindowCommand, drain};
    use aimer::quiver::winit::dpi::PhysicalSize;
    use aimer::style::AnimatedTheme;
    use aimer::{AimerApp, HeadlessAimerApp, HeadlessOptions, Provider};

    thread_local! {
        static SEEN: std::cell::RefCell<Option<(ProviderHandle<AppStore>, NavigatorController<AppRouter>)>> =
            const { std::cell::RefCell::new(None) };
    }

    /// The layer, under the theme and navigator the app gives it, keeping hold of
    /// the store and the navigator.
    #[derive(Clone, StatelessWidget)]
    struct Host;

    impl StatelessWidget for Host {
        fn build(&self, ctx: &BuildContext) -> impl Widget {
            SEEN.with(|seen| {
                *seen.borrow_mut() = Some((
                    ProviderHandle::<AppStore>::of(ctx),
                    NavigatorController::<AppRouter>::of(ctx),
                ))
            });
            AnimatedTheme::new()
                .data(ForumTheme::light())
                .child(WindowsLayer)
        }
    }

    fn start(prepare: impl FnOnce(&mut AppStore)) -> HeadlessAimerApp<impl Widget + 'static> {
        let mut store = AppStore::default();
        prepare(&mut store);
        crate::testing::hold();
        drain();
        let mut app = AimerApp::start_headless_with(
            Provider::new()
                .create(move || store.clone())
                .child(Navigator::new(AppRouter::Latest, |_| Host.boxed())),
            HeadlessOptions {
                size: PhysicalSize::new(1100, 800),
                scale_factor: 1.0,
            },
        );
        testing::settle(&mut app);
        app
    }

    fn store() -> ProviderHandle<AppStore> {
        SEEN.with(|seen| seen.borrow().as_ref().unwrap().0.clone())
    }

    /// The attributes of every window asked for since the last look.
    fn opened() -> Vec<WindowAttr> {
        drain()
            .into_iter()
            .filter_map(|request| match request {
                Request::Open(open) => Some(open.attr),
                Request::Command(..) => None,
            })
            .collect()
    }

    fn expected_dialog() -> WindowAttr {
        dialog_attr().min_inner_size(MIN_SIZE.0, MIN_SIZE.1)
    }

    fn expected_profile() -> WindowAttr {
        profile_attr().min_inner_size(MIN_SIZE.0, MIN_SIZE.1)
    }

    #[test]
    fn no_window_is_asked_for_until_the_store_says_so() {
        let _app = start(|_| {});
        assert!(opened().is_empty());
    }

    #[test]
    fn the_dialog_has_no_title_bar() {
        assert!(!expected_dialog().to_winit().decorations);
        assert!(expected_profile().to_winit().decorations);
    }

    #[test]
    fn the_dialog_opens_for_a_guest_and_a_member_alike() {
        for page in [DialogPage::Settings, DialogPage::Login] {
            let _app = start(|store| store.windows.dialog = Some(page));
            assert_eq!(opened(), [expected_dialog()]);
        }
        let _app = start(|store| {
            store.sign_in("domma".to_owned());
            store.windows.dialog = Some(DialogPage::Settings);
        });
        assert_eq!(opened(), [expected_dialog()]);
    }

    #[test]
    fn the_settings_and_the_sign_in_are_one_window() {
        // Asking for the sign-in with the settings open asks for no second window.
        let mut app = start(|store| store.windows.dialog = Some(DialogPage::Settings));
        assert_eq!(opened(), [expected_dialog()]);
        store().update(|store| store.windows.dialog = Some(DialogPage::Login));
        testing::settle(&mut app);
        assert!(opened().is_empty());
    }

    #[test]
    fn each_window_is_asked_for_once_however_often_the_layer_is_built() {
        let mut app = start(|store| {
            store.sign_in("domma".to_owned());
            store.windows.dialog = Some(DialogPage::Settings);
            store.windows.profile = true;
        });
        assert_eq!(opened(), [expected_dialog(), expected_profile()]);
        // Something else in the store changes, and the layer builds again.
        store().update(|store| store.theme_mode = aimer::style::ThemeMode::Dark);
        testing::settle(&mut app);
        assert!(opened().is_empty());
    }

    #[test]
    fn the_profile_window_opens_for_a_member() {
        let _app = start(|store| {
            store.sign_in("domma".to_owned());
            store.windows.profile = true;
        });
        assert_eq!(opened(), [expected_profile()]);
    }

    #[test]
    fn the_profile_is_not_opened_for_nobody() {
        let _app = start(|store| store.windows.profile = true);
        assert!(opened().is_empty());
    }

    #[test]
    fn a_window_the_member_closes_tells_the_store() {
        let _app = start(|store| {
            store.windows.dialog = Some(DialogPage::Login);
            store.current_user = Some("domma".to_owned());
            store.windows.profile = true;
        });
        let mut closers = Vec::new();
        for request in drain() {
            if let Request::Open(open) = request {
                closers.push(open.on_closed.expect("the store is told when it closes"));
            }
        }
        assert_eq!(closers.len(), 2);
        assert!(store().read().windows.dialog.is_some() && store().read().windows.profile);
        // The dialog was declared first: closing it leaves the other open.
        closers.remove(0)();
        assert!(
            store().read().windows.dialog.is_none(),
            "it would not open again"
        );
        assert!(store().read().windows.profile);
        closers.remove(0)();
        assert!(!store().read().windows.profile);
    }

    #[test]
    fn signing_in_closes_a_dialog_opened_to_sign_in() {
        let mut app = start(|store| store.windows.dialog = Some(DialogPage::Login));
        drain();
        store().update(|store| store.sign_in("domma".to_owned()));
        testing::settle(&mut app);
        assert!(
            drain()
                .iter()
                .any(|request| matches!(request, Request::Command(_, WindowCommand::Close))),
            "the window stayed open"
        );
        assert_eq!(
            store().read().windows.dialog,
            None,
            "it would open again on signing out"
        );
    }

    #[test]
    fn signing_in_leaves_the_settings_open() {
        let mut app = start(|store| store.windows.dialog = Some(DialogPage::Settings));
        drain();
        store().update(|store| store.sign_in("domma".to_owned()));
        testing::settle(&mut app);
        assert!(
            !drain()
                .iter()
                .any(|request| matches!(request, Request::Command(_, WindowCommand::Close)))
        );
        assert_eq!(store().read().windows.dialog, Some(DialogPage::Settings));
    }

    #[test]
    fn signing_out_closes_the_profile_window() {
        let mut app = start(|store| {
            store.sign_in("domma".to_owned());
            store.windows.profile = true;
        });
        drain();
        store().update(|store| store.sign_out());
        testing::settle(&mut app);
        assert!(
            drain()
                .iter()
                .any(|request| matches!(request, Request::Command(_, WindowCommand::Close)))
        );
        assert!(!store().read().windows.profile);
    }

    /// A page of the dialog, keeping hold of the dialog's navigator.
    #[derive(Clone, StatelessWidget)]
    struct Pages(AppRouter);

    impl StatelessWidget for Pages {
        fn build(&self, ctx: &BuildContext) -> impl Widget {
            SEEN.with(|seen| {
                *seen.borrow_mut() = Some((
                    ProviderHandle::<AppStore>::of(ctx),
                    NavigatorController::<AppRouter>::of(ctx),
                ))
            });
            DialogRoute(self.0.clone())
        }
    }

    /// The dialog's own pages, as the window builds them, starting at `first`.
    fn dialog(first: AppRouter, page: DialogPage) -> HeadlessAimerApp<impl Widget + 'static> {
        let mut store = AppStore::default();
        store.windows.dialog = Some(page);
        crate::testing::hold();
        let mut app = AimerApp::start_headless_with(
            Provider::new().create(move || store.clone()).child(
                AnimatedTheme::new()
                    .data(ForumTheme::light())
                    .child(Navigator::new(first, |route| Pages(route).boxed())),
            ),
            HeadlessOptions {
                size: PhysicalSize::new(640, 760),
                scale_factor: 1.0,
            },
        );
        testing::settle(&mut app);
        app
    }

    fn route() -> AppRouter {
        SEEN.with(|seen| seen.borrow().as_ref().unwrap().1.current_route())
    }

    fn push(route: AppRouter) {
        SEEN.with(|seen| seen.borrow().as_ref().unwrap().1.push(route));
    }

    /// Where the middle of the back button is, and of the close button.
    const BACK: (f64, f64) = (BAR_INSET as f64 + 30.0, BAR_TOP as f64 + 18.0);
    const CLOSE: (f64, f64) = (640.0 - BAR_INSET as f64 - 30.0, BAR_TOP as f64 + 18.0);

    #[test]
    fn a_page_pushed_in_the_dialog_has_a_back_button() {
        let mut app = dialog(AppRouter::Login, DialogPage::Login);
        push(AppRouter::LoginEmail);
        testing::settle(&mut app);
        assert!(route() == AppRouter::LoginEmail);

        testing::tap(&mut app, 1, BACK);
        testing::settle(&mut app);
        assert!(route() == AppRouter::Login);
    }

    #[test]
    fn the_first_page_of_the_dialog_has_no_back_button() {
        let mut app = dialog(AppRouter::Login, DialogPage::Login);
        testing::tap(&mut app, 1, BACK);
        testing::settle(&mut app);
        assert!(route() == AppRouter::Login);
    }

    #[test]
    fn the_close_button_closes_the_dialog() {
        for first in [AppRouter::Settings, AppRouter::Login] {
            let mut app = dialog(first, DialogPage::Settings);
            assert!(store().read().windows.dialog.is_some());
            testing::tap(&mut app, 1, CLOSE);
            testing::settle(&mut app);
            assert_eq!(store().read().windows.dialog, None);
        }
    }

    #[test]
    fn the_close_button_is_on_the_pages_pushed_too() {
        let mut app = dialog(AppRouter::Settings, DialogPage::Settings);
        push(AppRouter::SettingsAppearance);
        testing::settle(&mut app);
        testing::tap(&mut app, 1, CLOSE);
        testing::settle(&mut app);
        assert_eq!(store().read().windows.dialog, None);
    }

    #[test]
    fn the_appearance_page_goes_back_to_the_settings() {
        let mut app = dialog(AppRouter::Settings, DialogPage::Settings);
        push(AppRouter::SettingsAppearance);
        testing::settle(&mut app);
        assert!(route() == AppRouter::SettingsAppearance);

        testing::tap(&mut app, 1, BACK);
        testing::settle(&mut app);
        assert!(route() == AppRouter::Settings);
    }

    #[test]
    fn the_theme_row_opens_the_appearance_page_in_the_dialog() {
        let mut app = dialog(AppRouter::Settings, DialogPage::Settings);
        // The first row: it sits under the title and the "Appearance" heading.
        testing::tap(&mut app, 1, (300.0, 210.0));
        testing::settle(&mut app);
        assert!(route() == AppRouter::SettingsAppearance);
    }

    #[test]
    fn the_sign_in_row_of_the_settings_opens_the_sign_in_in_the_dialog() {
        let mut app = dialog(AppRouter::Settings, DialogPage::Settings);
        // The row of the account group, under "Theme" and the "Account" heading.
        testing::tap(&mut app, 1, (300.0, 330.0));
        testing::settle(&mut app);
        assert!(route() == AppRouter::Login);
        // And back to the settings.
        testing::tap(&mut app, 2, BACK);
        testing::settle(&mut app);
        assert!(route() == AppRouter::Settings);
    }

    #[test]
    fn asking_for_the_sign_in_moves_an_open_dialog_to_it() {
        let mut app = dialog(AppRouter::Settings, DialogPage::Settings);
        open_login(&store());
        testing::settle(&mut app);
        assert!(route() == AppRouter::Login);
        open_settings(&store());
        testing::settle(&mut app);
        assert!(route() == AppRouter::Settings);
    }

    #[test]
    fn signing_in_from_the_settings_returns_to_the_settings() {
        let mut app = dialog(AppRouter::Settings, DialogPage::Settings);
        push(AppRouter::Login);
        testing::settle(&mut app);
        // What the sign-in pages do once the forum has said yes.
        SEEN.with(|seen| {
            seen.borrow()
                .as_ref()
                .unwrap()
                .1
                .set_route(AppRouter::Account)
        });
        store().update(|store| store.sign_in("domma".to_owned()));
        testing::settle(&mut app);
        assert_eq!(store().read().windows.dialog, Some(DialogPage::Settings));
        assert!(route() == AppRouter::Account);
    }
}
