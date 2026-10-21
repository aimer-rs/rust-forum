use std::rc::Rc;
use std::time::Duration;

use crate::api::session;
use crate::preferences;
use crate::components::post_action_bar::{ActionBar, SearchControls};
use crate::components::side_bar::{AppSideBar, MOBILE_BREAKPOINT, MobileNavbar};
use crate::router::AppRouter;
use crate::store::AppStore;
use crate::theme::ForumTheme;
use aimer::Dimension::Percent;
use aimer::animation::{AnimationController, AnimationStatus, StatusListener, curves};
use aimer::share::ShareRef;
use aimer::provider::media_query::MediaQuery;
use aimer::router::{Navigator, Outlet};
use aimer::style::{AnimatedTheme, Theme, ThemeMode};
use aimer::*;

#[derive(Clone, StatefulWidget)]
pub struct AppShell {
    current_route: AppRouter,
}

impl AppShell {
    pub fn new(current_route: AppRouter) -> Self {
        Self { current_route }
    }
}

pub struct AppShellState {
    current_route: AppRouter,
}

impl StatefulWidget for AppShell {
    type State = AppShellState;

    fn create_state(self) -> Self::State {
        AppShellState {
            current_route: self.current_route,
        }
    }
}

impl State<AppShell> for AppShellState {
    fn init_state(&mut self, _updater: StateUpdater<Self>) {}

    fn adopt_config_from(&mut self, new: Self) {
        self.current_route = new.current_route;
    }

    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme_mode = ctx.select::<AppStore, ThemeMode>(|s| s.theme_mode);
        // The system appearance is only watched while the mode follows it, so an
        // explicit Light or Dark is not rebuilt by a switch it ignores.
        let brightness = theme_mode.resolve(if theme_mode.follows_system() {
            ctx.watch_platform_brightness()
        } else {
            Brightness::Light
        });
        AnimatedTheme::new()
            .data(match brightness {
                Brightness::Light => ForumTheme::light(),
                Brightness::Dark => ForumTheme::dark(),
            })
            .duration(Duration::from_millis(300))
            .curve(curves::EASE_IN_OUT)
            .child(ShellFrame)
    }
}

#[derive(Clone, StatelessWidget)]
struct ShellFrame;

impl StatelessWidget for ShellFrame {
    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme = *ForumTheme::of(ctx);
        // Only the answer is watched, so a window drag rebuilds this at the breakpoint
        // and nowhere else.

        let mobile = is_mobile(ctx);
        // The shell stays mounted across routes, so it is told nothing about
        // them; the navigator is where the current one is read.
        let route = Navigator::<AppRouter>::of(ctx).current_route();
        // The page, with the action bar floating over its top edge.
        let mut content = vec![
            restore_session(ProviderHandle::<AppStore>::of(ctx)),
            restore_preferences(ProviderHandle::<AppStore>::of(ctx)),
            SizedBox::new()
                .width(Percent(100.0))
                .height(Percent(100.0))
                .box_child(Outlet),
            Positioned::new()
                .layer(3)
                .top(case!(mobile, MOBILE_TOP))
                .box_child(ActionBarSlot::new(route.clone())),
        ];

        Container::new()
            .color(theme.background_color())
            .child(if mobile {
                content.push(NavbarSlot::new(route).boxed());
                Stack::new().children(content).boxed()
            } else {
                Row::new()
                    .justify_content(JustifyContent::SpaceBetween)
                    .children([
                        AppSideBar::new().boxed(),
                        Expanded::new()
                            .flex(0.95)
                            .box_child(Stack::new().children(content)),
                    ])
                    .boxed()
            })
    }
}

/// Signs the member the app was left signed in as back in, once, as it opens.
///
/// It paints nothing. Its request key is the same on every rebuild of the shell,
/// so the request is made the once and is kept; a member who signs in by hand
/// before it is answered is not replaced by it.
fn restore_session(store: ProviderHandle<AppStore>) -> AnyWidget {
    AsyncBuilder::new()
        .request_key("restore-session")
        .future(move || {
            let store = store.clone();
            async move {
                if let Some(username) = session::restore().await? {
                    store.update(|store| {
                        if store.current_user.is_none() {
                            store.sign_in(username);
                        }
                    });
                }
                Ok::<(), String>(())
            }
        })
        .child(|_| Widget::boxed(ZeroSizedBox))
        .boxed()
}

/// Puts back what the member chose for the app's theme and tab bar, once, as it
/// opens. Like [`restore_session`] it paints nothing.
///
/// The tests leave it out: they would read the choices of whoever runs them.
fn restore_preferences(store: ProviderHandle<AppStore>) -> AnyWidget {
    if cfg!(test) {
        return Widget::boxed(ZeroSizedBox);
    }
    AsyncBuilder::new()
        .request_key("restore-preferences")
        .future(move || {
            let store = store.clone();
            async move {
                let saved = preferences::load().await;
                store.update(|store| {
                    if let Some(mode) = saved.theme_mode {
                        store.set_theme_mode(mode);
                    }
                    if let Some(tabs) = saved.navbar {
                        store.navbar = tabs;
                    }
                });
                Ok::<(), String>(())
            }
        })
        .child(|_| Widget::boxed(ZeroSizedBox))
        .boxed()
}

/// Whether a route is a page the tab bar gives way to: the pages opened from
/// another (a post, somebody's profile) and the ones to sign in on, which are
/// about one thing and want the whole window for it.
fn hides_navbar(route: &AppRouter) -> bool {
    matches!(
        route,
        AppRouter::PostDetail { .. }
            | AppRouter::UserProfile { .. }
            | AppRouter::Login
            | AppRouter::LoginEmail
            | AppRouter::LoginPassword
            | AppRouter::SettingsAppearance
    )
}

/// How long a button takes to come in or go out.
const BUTTON_DURATION: Duration = Duration::from_millis(400);

/// The post a route shows, if it shows one.
fn post_of(route: &AppRouter) -> Option<i64> {
    match route {
        AppRouter::PostDetail { id, .. } => Some(*id),
        _ => None,
    }
}

/// The user a route shows the profile of, if it shows one.
fn profile_of(route: &AppRouter) -> Option<ShareRef<str>> {
    match route {
        AppRouter::UserProfile { username } => Some(username.clone()),
        _ => None,
    }
}

/// Whether a route is the search page.
fn is_search(route: &AppRouter) -> bool {
    matches!(route, AppRouter::Search)
}

/// Whether a route is the account page.
fn is_account(route: &AppRouter) -> bool {
    matches!(route, AppRouter::Account)
}

/// Whether a route has a back button. The search and account pages have none:
/// they are places the navigation leads to, not pages opened from another.
fn has_back(route: &AppRouter) -> bool {
    post_of(route).is_some()
        || profile_of(route).is_some()
        || matches!(route, AppRouter::Settings
                | AppRouter::SettingsAppearance
                | AppRouter::Login
                | AppRouter::LoginEmail
                | AppRouter::LoginPassword)
}

/// Where the bar over a page is: its parts come in with the page and go with it.
///
/// Each part has a controller of its own: the back button, the title (a user's
/// name), and the like and comment buttons. A page that brings new parts plays only
/// theirs, and the parts already there stay as they are.
#[derive(Clone, StatefulWidget)]
struct ActionBarSlot {
    route: AppRouter,
}

impl ActionBarSlot {
    fn new(route: AppRouter) -> Self {
        Self { route }
    }
}

/// One part of the bar: its motion, whether it is in the tree, and what it shows.
struct Part<T> {
    /// Drives the part: `0.0` out, `1.0` in.
    controller: AnimationController,
    /// Whether the part is in the tree at all. It leaves once it has gone out, so
    /// that, hidden, it cannot catch taps meant for the page under it.
    mounted: bool,
    /// What the part shows. Kept while it goes out, so it doesn't change as it
    /// leaves.
    shown: Option<T>,
}

impl<T> Part<T> {
    fn new(shown: Option<T>) -> Self {
        let controller = AnimationController::new(BUTTON_DURATION, curves::FAST_OUT_SLOW_IN);
        if shown.is_some() {
            controller.set_value(1.0);
        }
        Self {
            controller,
            mounted: shown.is_some(),
            shown,
        }
    }

    /// Sends the part where the route wants it: in with `wanted`, or out.
    fn follow(&mut self, wanted: Option<T>) {
        match wanted {
            Some(wanted) => {
                self.shown = Some(wanted);
                self.mounted = true;
                // The value is checked as well as the status: a controller set to
                // `1.0` by hand still reports itself dismissed.
                if self.controller.status() != AnimationStatus::Forward
                    && self.controller.value() < 1.0
                {
                    self.controller.forward();
                }
            }
            None if self.mounted => {
                // A part sent out before its first frame is still at `0.0` while
                // going in, and has to be turned round all the same: left alone it
                // would come in and stay.
                if self.controller.status() != AnimationStatus::Reverse
                    && (self.controller.value() > 0.0
                        || self.controller.status() == AnimationStatus::Forward)
                {
                    self.controller.reverse();
                }
            }
            None => {}
        }
    }

    /// What the part shows, while it is in the tree: its controller and content.
    fn visible(&self) -> Option<(AnimationController, &T)> {
        self.shown
            .as_ref()
            .filter(|_| self.mounted)
            .map(|shown| (self.controller.clone(), shown))
    }
}

struct ActionBarSlotState {
    route: AppRouter,
    /// Whether the page before this one was opened from another, so that this
    /// one is where the user came back to.
    came_back: bool,
    back: Part<()>,
    title: Part<ShareRef<str>>,
    counts: Part<i64>,
    search: Part<()>,
    /// The Blog / User toggle, at the end of the bar over the search page.
    toggle: Part<()>,
    /// The button over the account page that opens the settings.
    settings: Part<()>,
}

impl ActionBarSlotState {
    /// Sends each part of the bar where the route wants it.
    fn follow_route(&mut self) {
        self.back.follow(has_back(&self.route).then_some(()));
        self.title.follow(profile_of(&self.route));
        self.search.follow(is_search(&self.route).then_some(()));
        self.follow_end();
    }

    /// Sends the parts at the end of the bar where the route wants them: the like
    /// and comment buttons, the toggle and the settings button.
    ///
    /// They share one place, so only one of them is ever there. A page that wants
    /// its own waits until the one before has gone out entirely, and is brought in
    /// by [`follow_end`](Self::follow_end) again when that happens. Moving the
    /// two at once would put one over the other.
    fn follow_end(&mut self) {
        let free = |others: [&bool; 2]| !others.into_iter().any(|mounted| *mounted);
        let counts = post_of(&self.route).filter(|_| free([&self.toggle.mounted, &self.settings.mounted]));
        let toggle = is_search(&self.route).then_some(()).filter(|_| free([&self.counts.mounted, &self.settings.mounted]));
        let settings = is_account(&self.route).then_some(()).filter(|_| free([&self.counts.mounted, &self.toggle.mounted]));
        self.counts.follow(counts);
        // Sending one out may be what lets the next in, but it is still going out
        // until its motion ends, which is when this is asked again.
        self.toggle.follow(toggle);
        self.settings.follow(settings);
    }
}

impl StatefulWidget for ActionBarSlot {
    type State = ActionBarSlotState;

    fn create_state(self) -> Self::State {
        // A shell that starts on a page with a bar (a deep link) has it in place.
        ActionBarSlotState {
            came_back: false,
            back: Part::new(has_back(&self.route).then_some(())),
            title: Part::new(profile_of(&self.route)),
            counts: Part::new(post_of(&self.route)),
            search: Part::new(is_search(&self.route).then_some(())),
            toggle: Part::new(is_search(&self.route).then_some(())),
            settings: Part::new(is_account(&self.route).then_some(())),
            route: self.route,
        }
    }
}

/// Which part of the bar a controller drives.
#[derive(Clone, Copy)]
enum Which {
    Back,
    Title,
    Counts,
    Search,
    Toggle,
    Settings,
}

/// Takes a part of the bar out of the tree once it has finished going out.
struct PartListener {
    updater: StateUpdater<ActionBarSlotState>,
    which: Which,
}

// Safety: the shell, its state and the controllers all live on the rendering
// thread. The bounds only satisfy `StatusListener`, which is shared with
// controllers that may be handed to other threads; this one never leaves its
// creator's.
unsafe impl Send for PartListener {}
unsafe impl Sync for PartListener {}

impl StatusListener for PartListener {
    fn on_status_changed(&self, status: AnimationStatus) {
        if status != AnimationStatus::Dismissed {
            return;
        }
        let which = self.which;
        self.updater.set_state(move |state| match which {
            Which::Back if !has_back(&state.route) => state.back.mounted = false,
            Which::Title if profile_of(&state.route).is_none() => state.title.mounted = false,
            Which::Counts if post_of(&state.route).is_none() => state.counts.mounted = false,
            Which::Search if !is_search(&state.route) => state.search.mounted = false,
            Which::Toggle if !is_search(&state.route) => state.toggle.mounted = false,
            Which::Settings if !is_account(&state.route) => state.settings.mounted = false,
            _ => {}
        });
        // The end of the bar may be free now for the page that was waiting.
        self.updater.set_state(|state| state.follow_end());
    }
}

impl State<ActionBarSlot> for ActionBarSlotState {
    fn init_state(&mut self, updater: StateUpdater<Self>) {
        for (controller, which) in [
            (&self.back.controller, Which::Back),
            (&self.title.controller, Which::Title),
            (&self.counts.controller, Which::Counts),
            (&self.search.controller, Which::Search),
            (&self.toggle.controller, Which::Toggle),
            (&self.settings.controller, Which::Settings),
        ] {
            controller.add_status_listener(Rc::new(PartListener { updater, which }));
        }
    }

    fn adopt_config_from(&mut self, new: Self) {
        // The shell is rebuilt for more than a new page, so the page left is only
        // taken note of when there is one.
        if self.route != new.route {
            self.came_back = has_back(&self.route);
            self.route = new.route;
        }
        self.follow_route();
    }

    fn build(&self, _ctx: &BuildContext) -> impl Widget {
        if !self.back.mounted && !self.title.mounted && !self.counts.mounted && !self.search.mounted && !self.settings.mounted {
            return Widget::boxed(ZeroSizedBox);
        }
        #[cfg(test)]
        BAR_BUILDS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        ActionBar::new(
            self.back.visible().map(|(controller, _)| controller),
            self.title
                .visible()
                .map(|(controller, name)| (controller, name.to_string())),
            self.counts
                .visible()
                .map(|(controller, id)| (controller, *id)),
            // The toggle is part of the search field's layer, and its controller is
            // there even while it waits: it keeps its place and is not yet shown.
            self.search.visible().map(|(controller, _)| SearchControls {
                field: controller,
                toggle: self.toggle.controller.clone(),
                // Coming back from a post the user reads the results; they were
                // not opening the search to type.
                auto_focus: !self.came_back,
            }),
            self.settings.visible().map(|(controller, _)| controller),
        )
        .boxed()
    }
}

/// Where the tab bar of a narrow window is: it comes in and goes out with the
/// page, the way the buttons of the bar over a page do, and is out of the tree
/// once it has gone, so that it cannot catch taps meant for the page under it.
#[derive(Clone, StatefulWidget)]
struct NavbarSlot {
    route: AppRouter,
}

impl NavbarSlot {
    fn new(route: AppRouter) -> Self {
        Self { route }
    }
}

struct NavbarSlotState {
    route: AppRouter,
    bar: Part<()>,
}

impl NavbarSlotState {
    /// Sends the bar where the route wants it.
    fn follow_route(&mut self) {
        self.bar.follow((!hides_navbar(&self.route)).then_some(()));
    }
}

impl StatefulWidget for NavbarSlot {
    type State = NavbarSlotState;

    fn create_state(self) -> Self::State {
        // A shell that starts on a page with the bar has it in place.
        NavbarSlotState {
            bar: Part::new((!hides_navbar(&self.route)).then_some(())),
            route: self.route,
        }
    }
}

/// Takes the tab bar out of the tree once it has finished going out.
struct NavbarListener {
    updater: StateUpdater<NavbarSlotState>,
}

// Safety: as for `PartListener`, the shell, its state and the controllers all
// live on the rendering thread; the bounds only satisfy `StatusListener`.
unsafe impl Send for NavbarListener {}
unsafe impl Sync for NavbarListener {}

impl StatusListener for NavbarListener {
    fn on_status_changed(&self, status: AnimationStatus) {
        if status == AnimationStatus::Dismissed {
            self.updater.set_state(|state| {
                if hides_navbar(&state.route) {
                    state.bar.mounted = false;
                }
            });
        }
    }
}

impl State<NavbarSlot> for NavbarSlotState {
    fn init_state(&mut self, updater: StateUpdater<Self>) {
        self.bar.controller.add_status_listener(Rc::new(NavbarListener { updater }));
    }

    fn adopt_config_from(&mut self, new: Self) {
        self.route = new.route;
        self.follow_route();
    }

    fn build(&self, _ctx: &BuildContext) -> impl Widget {
        match self.bar.visible() {
            Some((controller, _)) => MobileNavbar::new(controller).boxed(),
            None => Widget::boxed(ZeroSizedBox),
        }
    }
}

#[cfg(test)]
pub(crate) static BAR_BUILDS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// How far the bar over a page, and the page's own top spacing, sit below the top
/// of a narrow window: what the status bar takes, and a little room under it, as
/// the navigation bars of Apple's apps leave. Everything that is spaced from the
/// bar is spaced from this.
pub const MOBILE_TOP: u32 = 52;

pub fn is_mobile(ctx: &BuildContext) -> bool {
    MediaQuery::select(ctx, |media| media.size.width < MOBILE_BREAKPOINT)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn post(id: i64) -> AppRouter {
        AppRouter::PostDetail {
            slug: ShareRef::from_static("a-post"),
            id,
        }
    }

    fn profile(name: &'static str) -> AppRouter {
        AppRouter::UserProfile {
            username: ShareRef::from_static(name),
        }
    }

    fn state(route: AppRouter) -> ActionBarSlotState {
        ActionBarSlot::new(route).create_state()
    }

    fn go(shell: &mut ActionBarSlotState, route: AppRouter) {
        shell.adopt_config_from(state(route));
    }

    #[test]
    fn nothing_is_there_until_a_page_with_a_bar_is_opened() {
        let shell = state(AppRouter::Latest);
        assert!(!shell.back.mounted && !shell.title.mounted && !shell.counts.mounted);
        assert_eq!(shell.back.controller.value(), 0.0);
    }

    #[test]
    fn a_shell_that_starts_on_a_post_has_its_parts_in_place() {
        let shell = state(post(7));
        assert!(shell.back.mounted && shell.counts.mounted && !shell.title.mounted);
        assert_eq!(shell.counts.shown, Some(7));
        assert_eq!(
            (shell.back.controller.value(), shell.counts.controller.value()),
            (1.0, 1.0)
        );
    }

    #[test]
    fn opening_a_post_mounts_its_parts_and_leaving_keeps_them_while_they_go_out() {
        let mut shell = state(AppRouter::Latest);

        go(&mut shell, post(7));
        assert!(shell.back.mounted && shell.counts.mounted);
        assert_eq!(shell.counts.shown, Some(7));

        go(&mut shell, AppRouter::Latest);
        // Still there, showing the same post, until the motion ends and the
        // listener takes them out.
        assert!(shell.back.mounted && shell.counts.mounted);
        assert_eq!(shell.counts.shown, Some(7));
    }

    #[test]
    fn a_profile_has_the_back_button_and_the_users_name_but_no_counts() {
        let mut shell = state(AppRouter::Latest);
        go(&mut shell, profile("domma"));
        assert!(shell.back.mounted && shell.title.mounted);
        assert!(!shell.counts.mounted);
        assert_eq!(shell.title.shown.as_deref(), Some("domma"));
    }

    #[test]
    fn going_from_a_profile_to_a_post_swaps_the_title_for_the_counts_and_keeps_back() {
        let mut shell = state(profile("domma"));
        // The back button is in, as the profile page leaves it.
        shell.back.controller.set_value(1.0);
        shell.back.controller.forward();

        go(&mut shell, post(7));

        assert!(shell.counts.mounted);
        assert_eq!(shell.counts.shown, Some(7));
        // The title is on its way out, and the back button was not played again.
        assert_eq!(shell.title.controller.status(), AnimationStatus::Reverse);
        assert_eq!(shell.back.controller.value(), 1.0);
    }

    #[test]
    fn a_bar_set_in_place_by_a_deep_link_still_goes_out() {
        let mut shell = state(post(7));
        go(&mut shell, AppRouter::Latest);
        assert_eq!(shell.back.controller.status(), AnimationStatus::Reverse);
        assert_eq!(shell.counts.controller.status(), AnimationStatus::Reverse);
    }

    #[test]
    fn moving_between_posts_points_the_counts_at_the_new_one() {
        let mut shell = state(post(7));
        go(&mut shell, post(8));
        assert_eq!(shell.counts.shown, Some(8));
        assert!(shell.counts.mounted);
    }

    #[test]
    fn the_search_page_has_the_search_field_and_nothing_else() {
        let mut shell = state(AppRouter::Latest);
        go(&mut shell, AppRouter::Search);
        assert!(shell.search.mounted);
        assert!(!shell.back.mounted && !shell.title.mounted && !shell.counts.mounted);
    }

    #[test]
    fn leaving_the_search_page_sends_its_field_out_and_keeps_it_while_it_goes() {
        let mut shell = state(AppRouter::Search);
        assert_eq!(shell.search.controller.value(), 1.0);
        go(&mut shell, AppRouter::Latest);
        assert!(shell.search.mounted);
        assert_eq!(shell.search.controller.status(), AnimationStatus::Reverse);
    }

    #[test]
    fn going_from_search_to_a_profile_brings_in_the_back_button_and_the_name() {
        let mut shell = state(AppRouter::Search);
        go(&mut shell, profile("domma"));
        assert!(shell.back.mounted && shell.title.mounted);
        assert_eq!(shell.search.controller.status(), AnimationStatus::Reverse);
    }

    #[test]
    fn a_part_sent_out_before_its_first_frame_still_goes_out() {
        // The search page opened and left at once: the field was told to come in
        // but has not moved yet, and must not be left mounted over the back button.
        let mut shell = state(AppRouter::Latest);
        go(&mut shell, AppRouter::Search);
        assert_eq!(shell.search.controller.value(), 0.0);
        go(&mut shell, profile("domma"));
        assert_eq!(shell.search.controller.status(), AnimationStatus::Reverse);
        assert!(shell.back.mounted);
    }

    #[test]
    fn the_search_field_takes_the_keyboard_unless_the_user_came_back_to_the_results() {
        let mut shell = state(AppRouter::Latest);
        go(&mut shell, AppRouter::Search);
        assert!(!shell.came_back);

        go(&mut shell, post(7));
        go(&mut shell, AppRouter::Search);
        assert!(shell.came_back);
        // A rebuild of the shell for another reason is not leaving the page.
        go(&mut shell, AppRouter::Search);
        assert!(shell.came_back);

        // Opening the search from a feed again is not coming back.
        go(&mut shell, AppRouter::Latest);
        go(&mut shell, AppRouter::Search);
        assert!(!shell.came_back);
    }

    #[test]
    fn the_toggle_waits_for_the_like_and_comment_buttons_to_go_out() {
        let mut shell = state(post(7));
        go(&mut shell, AppRouter::Search);
        // The counts are going out, and the toggle is not yet on its way in.
        assert_eq!(shell.counts.controller.status(), AnimationStatus::Reverse);
        assert!(!shell.toggle.mounted);
        assert_eq!(shell.toggle.controller.status(), AnimationStatus::Dismissed);

        // The counts finish, and the toggle is let in.
        shell.counts.mounted = false;
        shell.follow_end();
        assert!(shell.toggle.mounted);
        assert_eq!(shell.toggle.controller.status(), AnimationStatus::Forward);
    }

    #[test]
    fn the_counts_wait_for_the_toggle_to_go_out() {
        let mut shell = state(AppRouter::Search);
        go(&mut shell, post(7));
        assert_eq!(shell.toggle.controller.status(), AnimationStatus::Reverse);
        assert!(!shell.counts.mounted);

        shell.toggle.mounted = false;
        shell.follow_end();
        assert!(shell.counts.mounted);
        assert_eq!(shell.counts.controller.status(), AnimationStatus::Forward);
    }

    #[test]
    fn a_page_with_nothing_before_it_at_the_end_of_the_bar_does_not_wait() {
        let mut shell = state(AppRouter::Latest);
        go(&mut shell, AppRouter::Search);
        assert!(shell.toggle.mounted);
        assert_eq!(shell.toggle.controller.status(), AnimationStatus::Forward);
    }

    #[test]
    fn the_account_page_has_the_settings_button_and_no_back_button() {
        let mut shell = state(AppRouter::Latest);
        go(&mut shell, AppRouter::Account);
        assert!(shell.settings.mounted);
        assert!(!shell.back.mounted && !shell.title.mounted && !shell.search.mounted);
    }

    #[test]
    fn the_settings_page_has_a_back_button_to_the_account_page() {
        let mut shell = state(AppRouter::Account);
        go(&mut shell, AppRouter::Settings);
        assert!(shell.back.mounted);
        // The settings button goes out as the page opens, and is not played again.
        assert_eq!(shell.settings.controller.status(), AnimationStatus::Reverse);
    }

    #[test]
    fn the_appearance_page_has_a_back_button_to_the_settings() {
        let mut shell = state(AppRouter::Settings);
        go(&mut shell, AppRouter::SettingsAppearance);
        assert!(shell.back.mounted && !shell.title.mounted && !shell.search.mounted);
    }

    #[test]
    fn the_login_page_has_a_back_button_to_the_account_page() {
        let mut shell = state(AppRouter::Account);
        go(&mut shell, AppRouter::Login);
        assert!(shell.back.mounted && !shell.title.mounted && !shell.search.mounted);
    }

    #[test]
    fn the_tab_bar_gives_way_to_posts_other_profiles_the_sign_in_pages_and_the_appearance_page_only() {
        for route in [
            AppRouter::PostDetail { slug: ShareRef::from_static("a"), id: 1 },
            AppRouter::UserProfile { username: ShareRef::from_static("domma") },
            AppRouter::Login,
            AppRouter::LoginEmail,
            AppRouter::LoginPassword,
            AppRouter::SettingsAppearance,
        ] {
            assert!(hides_navbar(&route));
        }
        for route in [
            AppRouter::Latest,
            AppRouter::New,
            AppRouter::Hot,
            AppRouter::Search,
            AppRouter::Account,
            AppRouter::Settings,
            AppRouter::CategoryTopics { slug: ShareRef::from_static("rust"), id: 12 },
        ] {
            assert!(!hides_navbar(&route));
        }
    }

    fn navbar(route: AppRouter) -> NavbarSlotState {
        NavbarSlot::new(route).create_state()
    }

    fn navbar_go(shell: &mut NavbarSlotState, route: AppRouter) {
        shell.adopt_config_from(navbar(route));
    }

    #[test]
    fn the_tab_bar_starts_in_place_on_a_page_that_has_it_and_out_on_one_that_has_not() {
        let shown = navbar(AppRouter::Latest);
        assert!(shown.bar.mounted);
        assert_eq!(shown.bar.controller.value(), 1.0);

        let hidden = navbar(AppRouter::Login);
        assert!(!hidden.bar.mounted);
        assert_eq!(hidden.bar.controller.value(), 0.0);
    }

    #[test]
    fn opening_a_post_sends_the_tab_bar_out_and_going_back_brings_it_in_again() {
        let mut shell = navbar(AppRouter::Latest);

        navbar_go(&mut shell, post(1));
        // It goes out on its controller and stays in the tree until it has gone.
        assert_eq!(shell.bar.controller.status(), AnimationStatus::Reverse);
        assert!(shell.bar.mounted);

        // Back before it has gone, it turns round from where it is.
        shell.bar.controller.set_value(0.5);
        navbar_go(&mut shell, AppRouter::Latest);
        assert_eq!(shell.bar.controller.status(), AnimationStatus::Forward);
        assert!(shell.bar.mounted);
    }

    #[test]
    fn the_tab_bar_stays_put_between_pages_that_have_it() {
        let mut shell = navbar(AppRouter::Latest);
        navbar_go(&mut shell, AppRouter::Search);
        navbar_go(&mut shell, AppRouter::Account);
        assert_eq!(shell.bar.controller.value(), 1.0);
        assert_ne!(shell.bar.controller.status(), AnimationStatus::Reverse);
    }
}
