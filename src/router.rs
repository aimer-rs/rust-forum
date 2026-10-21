use std::ops::Range;
use std::time::Duration;

use crate::app_shell::AppShell;
use crate::components::topic_card::topic_tag;
use crate::screen::post_detail::PostDetailScreen;
use crate::screen::account::AccountScreen;
use crate::screen::login::{EmailCodeScreen, LoginScreen, PasswordScreen};
use crate::screen::search::SearchScreen;
use crate::screen::settings::{AppearanceScreen, SettingsScreen};
use crate::screen::topic_list::TopicListScreen;
use crate::screen::user_profile::UserProfileScreen;
use aimer::animation::curves;
use aimer::router::{
    CrossFadeTransition, ExpandTransition, NavigationContext, NavigationTransition, ResolvedNavigationTransition,
    Route, Router, Shell, SlideTransition, TransitionChoice, ViewLevelTransitions,
};
use aimer::share::{ShareRef, Shared};
use aimer::widget::{AnyElement, PortableWidget};
use aimer::{AnyWidget, BuildContext, Key, Widget};

const ROUTE_TRANSITION_DURATION: Duration = Duration::from_millis(200);
/// How long a post takes to open out of its card and to close into it again.
const POST_OPEN_DURATION: Duration = Duration::from_millis(500);
/// The most blur, in logical pixels, that builds on everything while the card
/// dissolves into the post: sharp before the dissolve, this much at its peak
/// (held through the middle of it, see `POST_BLUR_HELD`), and sharp again after.
/// `0.0` turns the blur off. The renderer clamps it to 96.
const POST_OPEN_BLUR: f32 = 30.0;
/// How dark the feed has gone, from 0 to 1, by the time the post fills the screen.
const POST_OPEN_DIM: f32 = 0.5;
/// The fraction of the dissolve through which the blur stays at its fullest. The
/// blur rises over what is left of the dissolve before it and falls over what is
/// left after, which is about 27% each way.
const POST_BLUR_HELD: f32 = 0.45;

/// Where on the open's timeline the card dissolves into the post.
///
/// The window grows over the whole open, so this is while it is growing: the
/// card starts to go a fifth of the way in, and is gone, with the post all
/// that is left in the window, at seven tenths, before the window is full.
fn post_dissolve() -> Range<f32> {
    0.2..0.7
}

/// Corner radius, in logical pixels, of the window while it sits on the card.
const POST_CARD_RADIUS: f32 = 12.0;
/// Every route shares this boundary, so the [`AppShell`] frame (sidebar) stays
/// mounted and only the content outlet animates between routes.
const APP_SHELL_KEY: &str = "rustforum::app-shell";

#[derive(Clone, PartialEq)]
pub enum AppRouter {
    Latest,
    New,
    Hot,
    CategoryTopics { slug: ShareRef<str>, id: i64 },
    PostDetail { slug: ShareRef<str>, id: i64 },
    UserProfile { username: ShareRef<str> },
    Search,
    Account,
    Login,
    LoginEmail,
    LoginPassword,
    Settings,
    SettingsAppearance,
}

impl Route for AppRouter {
    fn parse(path: &str) -> Option<Self> {
        let path = path.split(['?', '#']).next().unwrap_or(path);
        let mut segments = path.trim_matches('/').split('/');
        let route = match segments.next()? {
            "" => Self::Latest,
            "new" => Self::New,
            "hot" => Self::Hot,
            "settings" => match segments.next() {
                None => Self::Settings,
                Some("appearance") => Self::SettingsAppearance,
                Some(_) => return None,
            },
            "search" => Self::Search,
            "account" => Self::Account,
            "login" => match segments.next() {
                None => Self::Login,
                Some("email") => Self::LoginEmail,
                Some("password") => Self::LoginPassword,
                Some(_) => return None,
            },
            "u" => Self::UserProfile {
                username: ShareRef::from_str_slice(segments.next()?),
            },
            kind @ ("c" | "t") => {
                let slug = ShareRef::from_str_slice(segments.next()?);
                let id = segments.next()?.parse().ok()?;
                if kind == "c" {
                    Self::CategoryTopics { slug, id }
                } else {
                    Self::PostDetail { slug, id }
                }
            }
            _ => return None,
        };
        segments.next().is_none().then_some(route)
    }

    fn format(&self) -> String {
        match self {
            Self::Latest => "/".to_string(),
            Self::New => "/new".to_string(),
            Self::Hot => "/hot".to_string(),
            Self::CategoryTopics { slug, id } => format!("/c/{slug}/{id}"),
            Self::PostDetail { slug, id } => format!("/t/{slug}/{id}"),
            Self::UserProfile { username } => format!("/u/{username}"),
            Self::Search => "/search".to_string(),
            Self::Account => "/account".to_string(),
            Self::Login => "/login".to_string(),
            Self::LoginEmail => "/login/email".to_string(),
            Self::LoginPassword => "/login/password".to_string(),
            Self::Settings => "/settings".to_string(),
            Self::SettingsAppearance => "/settings/appearance".to_string(),
        }
    }

    fn shell_key(&self) -> Option<&'static str> {
        Some(APP_SHELL_KEY)
    }
}

// The `Router` derive emitted these; the hand-written `Route` above replaces
// it so the routes can declare a persistent shell boundary.
impl Widget for AppRouter {
    fn to_element(self, ctx: &BuildContext) -> AnyElement {
        Router::build(&self, ctx).to_element(ctx)
    }
}

impl PortableWidget for AppRouter {}

/// A route as seen by the shell's content outlet.
///
/// The outlet skips its animation between routes that share a `shell_key`, so
/// it needs a route type of its own that has none: [`AppRouter`] carries the
/// shell boundary, while this wrapper carries the page transitions.
#[derive(Clone, PartialEq)]
struct AppPage(AppRouter);

impl Route for AppPage {
    fn parse(path: &str) -> Option<Self> {
        AppRouter::parse(path).map(Self)
    }

    fn format(&self) -> String {
        self.0.format()
    }

    /// How deep a page is, for [`ViewLevelTransitions`]: a profile is a level
    /// below the feeds, and a post a level below a profile, so a post opened from
    /// a profile enters from the right and leaves to it.
    fn view_level(&self) -> u32 {
        match self.0 {
            AppRouter::UserProfile { .. } => 1,
            AppRouter::PostDetail { .. } => 2,
            _ => 0,
        }
    }

    /// Opening a post grows the page out of the topic card that opened it, and
    /// popping shrinks it back into the card. Without a card on screen (a deep
    /// link, say) it zooms in instead. A post opened from a user profile slides
    /// in by view level instead, and slides back out the same way; see
    /// [`PostTransition`].
    fn navigation_transition(&self) -> TransitionChoice<Self> {
        match self.0 {
            AppRouter::PostDetail { .. } => TransitionChoice::custom(PostTransition::new()),
            // The sign-in and settings pages slide in over the page before, as a page of a stack
            // does, and slide away again on a pop or an edge swipe.
            AppRouter::Login
            | AppRouter::LoginEmail
            | AppRouter::LoginPassword
            | AppRouter::Settings
            | AppRouter::SettingsAppearance => {
                TransitionChoice::custom(SlideTransition::new())
            }
            _ => TransitionChoice::Inherit,
        }
    }
}

/// How the page of a post is reached, which depends on where it is reached from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PostMotion {
    /// The page slides in by view level, as one page of a stack does over another.
    SlideByLevel,
    /// The page grows out of the topic card that opened it.
    Expand,
}

/// Picks the motion for the navigation between `from` and `to`, one of which is
/// a post.
///
/// A profile lists posts too, but its rows are not the cards the page grows out
/// of: the card of that topic may be somewhere in a feed that is no longer on
/// screen, and the page would grow from where it was. The search page is a
/// place to look something up rather than a feed to open cards from, and it
/// keeps no card to shrink back into. So a post reached from either, and the way
/// back to it, slide by view level instead.
fn post_motion(from: &AppPage, to: &AppPage) -> PostMotion {
    let slides = |page: &AppPage| matches!(page.0, AppRouter::UserProfile { .. } | AppRouter::Search);
    if slides(from) || slides(to) { PostMotion::SlideByLevel } else { PostMotion::Expand }
}

/// The transition of a post page: it grows out of its card from a feed, and
/// slides in by view level from a profile.
///
/// A route's policy is chosen by the destination on a push and by the departing
/// page on a pop, and a custom effect is handed the real source and destination
/// either way. That is what lets one policy tell the two apart.
struct PostTransition {
    expand: ExpandTransition<AppPage>,
    levels: ViewLevelTransitions,
}

impl PostTransition {
    fn new() -> Self {
        Self { expand: post_expand_transition(), levels: ViewLevelTransitions::new() }
    }
}

impl NavigationTransition<AppPage> for PostTransition {
    fn resolve(&self, navigation: &NavigationContext<'_, '_, AppPage>) -> Option<ResolvedNavigationTransition> {
        match post_motion(navigation.from, navigation.to) {
            PostMotion::SlideByLevel => self.levels.resolve(navigation),
            PostMotion::Expand => self.expand.resolve(navigation),
        }
    }
}

/// How a post opens out of its card: the card grows to fill the screen, and
/// while it grows it turns into the post.
///
/// The card's window starts opening on the first frame and grows over the whole
/// open, on `FAST_OUT_SLOW_IN`, quickly at first and settling slowly; its corners
/// round off with it. The card is stretched to the window, so it is the card
/// itself that expands.
///
/// Inside it the card crossfades into the post, from a fifth of the way in to
/// seven tenths (`post_dissolve`), on an S-curve (slow, quick, slow) with the
/// post rising exactly as the card falls. The two are happening at once: the card
/// is still growing as it becomes the post, and the post is still growing once it
/// has. Everything blurs through the dissolve, up to `POST_OPEN_BLUR` and held at
/// that through the middle of it (`POST_BLUR_HELD`), which also hides the little
/// of the feed that shows through at the crossing. The feed darkens to
/// `POST_OPEN_DIM` along the window, as the window covers it, so the post reads
/// as shading what it leaves behind.
///
/// The timeline is linear so that the window's curve is its only easing. Going
/// back plays the same motion in reverse.
fn post_expand_transition() -> ExpandTransition<AppPage> {
    ExpandTransition::new()
        .tag(post_card_tag)
        .duration(POST_OPEN_DURATION)
        .grow_curve(curves::EASE_IN_OUT)
        .dissolve(post_dissolve())
        .dissolve_curve(curves::EASE_IN_OUT)
        .crossfade(true)
        .scrim(POST_OPEN_DIM)
        .blur(POST_OPEN_BLUR)
        .blur_held(POST_BLUR_HELD)
        .source_radius(POST_CARD_RADIUS)
        .page_radius(16.0)
}

/// The topic card a post page grows out of, or `None` for any other route.
fn post_card_tag(route: &AppPage) -> Option<Key> {
    match route.0 {
        AppRouter::PostDetail { id, .. } => Some(topic_tag(id)),
        _ => None,
    }
}

impl Router for AppPage {
    fn build(&self, _ctx: &BuildContext) -> AnyWidget {
        match &self.0 {
            AppRouter::Latest => topic_list_content(ShareRef::from_static("latest.json"), "Latest"),
            AppRouter::New => topic_list_content("new.json", "New"),
            AppRouter::Hot => topic_list_content("hot.json", "Hot"),
            AppRouter::CategoryTopics { slug, id } => {
                let path = format!("c/{slug}/{}.json", *id);
                topic_list_content(Shared::new(path).project(|item| item.as_str()), slug.clone())
            }
            AppRouter::PostDetail { slug, id } => PostDetailScreen::new(slug.clone(), *id).boxed(),
            AppRouter::UserProfile { username } => UserProfileScreen::new(username.clone()).boxed(),
            AppRouter::Search => SearchScreen.boxed(),
            AppRouter::Account => AccountScreen.boxed(),
            AppRouter::Login => LoginScreen.boxed(),
            AppRouter::LoginEmail => EmailCodeScreen.boxed(),
            AppRouter::LoginPassword => PasswordScreen.boxed(),
            AppRouter::Settings => SettingsScreen.boxed(),
            AppRouter::SettingsAppearance => AppearanceScreen.boxed(),
        }
    }
}

/// The page shown in the shell's content outlet for a fixed Discourse
/// list/category path.
fn topic_list_content(
    path: impl Into<ShareRef<str>>,
    title: impl Into<ShareRef<str>>,
) -> AnyWidget {
    TopicListScreen::new(path.into(), title.into()).boxed()
}

impl Router for AppRouter {
    fn build(&self, _ctx: &BuildContext) -> AnyWidget {
        let page = AppPage(self.clone());
        // The persistent frame (sidebar) never moves; only the outlet animates.
        // Routes without their own transition cross-fade.
        let content = page.clone();
        Shell::new(AppShell::new(self.clone()), move |ctx| content.build(ctx))
            .route(page)
            .transition(
                CrossFadeTransition::new()
                    .duration(ROUTE_TRANSITION_DURATION)
                    .curve(curves::FAST_OUT_SLOW_IN),
            )
            .boxed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_round_trip_through_their_paths() {
        for path in ["/", "/new", "/hot", "/settings", "/settings/appearance", "/search", "/account", "/login", "/login/email", "/login/password", "/c/rust/12", "/t/hello-world/345", "/u/domma"] {
            let route = AppRouter::parse(path).expect(path);
            assert_eq!(route.format(), path);
        }
        assert!(AppRouter::parse("/t/hello/not-a-number").is_none());
        assert!(AppRouter::parse("/new/extra").is_none());
        assert!(AppRouter::parse("/login/other").is_none());
        assert!(AppRouter::parse("/nope").is_none());
    }

    thread_local! {
        /// The path of the route the test app most recently built.
        static LAST_ROUTE: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
    }

    /// The app on a feed of `count` topics, ids from 1, already streamed.
    fn feed_app(count: i64) -> aimer::HeadlessAimerApp<impl aimer::Widget + 'static> {
        feed_app_with(count, |_| {})
    }

    /// [`feed_app`], with `seed` run on the store before the app starts.
    fn feed_app_with(
        count: i64,
        seed: impl FnOnce(&mut crate::store::AppStore),
    ) -> aimer::HeadlessAimerApp<impl aimer::Widget + 'static> {
        use crate::models::Topic;
        use crate::store::AppStore;
        use aimer::quiver::winit::dpi::PhysicalSize;
        use aimer::router::Navigator;
        use aimer::{AimerApp, HeadlessOptions, Provider};

        let mut store = AppStore::default();
        seed(&mut store);
        // The same topics on every list, as the real lists overlap.
        for path in ["latest.json", "new.json", "hot.json"] {
            let path = ShareRef::from_static(path);
        for id in 1..=count {
            store.push_stream_topic(
                path.clone(),
                Topic {
                    id,
                    title: format!("Topic {id}"),
                    slug: format!("topic-{id}"),
                    posts_count: 3,
                    last_poster_username: format!("user{id}"),
                    ..Default::default()
                },
            );
        }
        store.finish_stream(path);
        }

        crate::testing::hold();
        let mut app = AimerApp::start_headless_with(
            Provider::new()
                .create(move || store.clone())
                .child(Navigator::new(AppRouter::Latest, |r| {
                    LAST_ROUTE.with(|last| *last.borrow_mut() = r.format());
                    r.boxed()
                })),
            HeadlessOptions { size: PhysicalSize::new(450, 780), scale_factor: 1.0 },
        );
        app.pump_frames(10);
        app
    }

    fn click(app: &mut aimer::HeadlessAimerApp<impl aimer::Widget + 'static>, at: (f32, f32)) {
        use aimer::quiver::winit::dpi::PhysicalPosition;
        use aimer::quiver::winit::event::{DeviceId, ElementState, MouseButton, WindowEvent};

        app.send_window_event(WindowEvent::CursorMoved {
            device_id: DeviceId::dummy(),
            position: PhysicalPosition::new(at.0 as f64, at.1 as f64),
        });
        for state in [ElementState::Pressed, ElementState::Released] {
            app.send_window_event(WindowEvent::MouseInput {
                device_id: DeviceId::dummy(),
                state,
                button: MouseButton::Left,
            });
        }
    }

    /// Pumps frames until the app stops asking for more, returning how many.
    fn settle(app: &mut aimer::HeadlessAimerApp<impl aimer::Widget + 'static>) -> usize {
        let mut frames = 0;
        for _ in 0..300 {
            std::thread::sleep(Duration::from_millis(10));
            if app.pump_frames(1) == 0 {
                break;
            }
            frames += 1;
        }
        frames
    }

    /// Taps a card and watches every frame of the open for a skeleton.
    fn skeletons_while_opening(cached_post: bool) -> (usize, usize) {
        use crate::components::skeleton_topic_card::BUILT as LIST;
        use crate::screen::post_detail::BUILT as POST;
        use aimer::animation::shared_element_bounds;
        use std::sync::atomic::Ordering::SeqCst;

        let mut app = feed_app_with(3, |store| {
            if cached_post {
                store.cache_post(2, Default::default());
            }
        });
        let card = shared_element_bounds(&topic_tag(2)).expect("the card is a shared element");
        let (list_before, post_before) = (LIST.load(SeqCst), POST.load(SeqCst));

        click(&mut app, (card.x + card.width / 2.0, card.y + card.height / 2.0));
        for _ in 0..60 {
            std::thread::sleep(Duration::from_millis(10));
            app.pump_frames(1);
        }
        (LIST.load(SeqCst) - list_before, POST.load(SeqCst) - post_before)
    }

    #[test]
    fn opening_a_cached_post_builds_no_skeleton_anywhere() {
        let (list, post) = skeletons_while_opening(true);
        assert_eq!((list, post), (0, 0), "skeletons built: list {list}, post {post}");
    }

    #[test]
    fn opening_a_post_builds_the_shells_action_bar() {
        use crate::app_shell::BAR_BUILDS;
        use aimer::animation::shared_element_bounds;
        use std::sync::atomic::Ordering::SeqCst;

        let mut app = feed_app(3);
        let before = BAR_BUILDS.load(SeqCst);
        let card = shared_element_bounds(&topic_tag(2)).unwrap();
        click(&mut app, (card.x + card.width / 2.0, card.y + card.height / 2.0));
        settle(&mut app);
        assert!(BAR_BUILDS.load(SeqCst) > before, "the action bar was never built");
    }

    #[test]
    fn opening_an_uncached_post_does_show_its_skeleton_while_it_loads() {
        // Keeps the counting honest: the same watch must see a skeleton here.
        let (list, post) = skeletons_while_opening(false);
        assert!(post > 0, "no post skeleton was built");
        assert_eq!(list, 0, "the feed must never fall back to its skeleton");
    }

    #[test]
    fn tapping_the_author_leaves_the_feed_for_their_profile() {
        use aimer::animation::shared_element_bounds;

        let mut app = feed_app(3);
        let card = shared_element_bounds(&topic_tag(2)).expect("the card is a shared element");
        assert_eq!(LAST_ROUTE.with(|last| last.borrow().clone()), "/");

        // The avatar is the first thing in the card's header, inside its padding.
        click(&mut app, (card.x + 20.0 + 12.0, card.y + 20.0 + 12.0));
        settle(&mut app);

        assert_eq!(LAST_ROUTE.with(|last| last.borrow().clone()), "/u/user2");
    }

    #[test]
    fn opening_a_post_grows_it_out_of_its_card() {
        use aimer::animation::{last_shared_element, shared_element_bounds};

        let mut app = feed_app(3);
        // The tab bar's icons are asset files, which a headless run cannot load.
        // Whatever else declines to paint is the transition's doing.
        let declined_at_rest = app
            .paint_source_census()
            .expect("a mounted page has a root")
            .declined_paint
            .len();

        let card = shared_element_bounds(&topic_tag(2)).expect("the card is a shared element");
        assert!(last_shared_element(&topic_tag(2)).unwrap().content.is_some());

        click(&mut app, (card.x + card.width / 2.0, card.y + card.height / 2.0));
        // Mid-motion the list, the blur over it, the post and the card's copy are
        // all on screen, and every one of them paints through the retained path.
        app.pump_frames(3);
        let census = app.paint_source_census().expect("a mounted page has a root");
        assert_eq!(census.declined_paint.len(), declined_at_rest, "declined {:?}", census.declined_paint);
        assert!(census.drawn_unmapped.is_empty(), "unmapped {:?}", census.drawn_unmapped);
        assert!(census.invalid_bounds.is_empty(), "invalid bounds {:?}", census.invalid_bounds);
        let frames = settle(&mut app);
        assert!(frames > 5, "the push should animate, got {frames} frames");
    }

    #[test]
    fn going_back_still_knows_the_card_in_a_long_feed() {
        use aimer::animation::{last_shared_element, shared_element_bounds};

        // A feed longer than the registry remembers dropped items: the cards
        // dropped after the one that was opened must not push it out.
        let mut app = feed_app(60);
        let card = shared_element_bounds(&topic_tag(2)).expect("the card is a shared element");

        click(&mut app, (card.x + card.width / 2.0, card.y + card.height / 2.0));
        assert!(settle(&mut app) > 5, "the push animates");

        let remembered = last_shared_element(&topic_tag(2));
        assert!(remembered.is_some(), "Back would fall back to the plain zoom");
        assert!(remembered.unwrap().content.is_some());
    }

    #[test]
    fn the_blur_builds_to_thirty_pixels_at_its_peak() {
        // Configurable in `POST_OPEN_BLUR`: the feed goes from sharp to this many
        // pixels of blur through the dissolve, and back to sharp.
        assert_eq!(POST_OPEN_BLUR, 30.0);
    }

    #[test]
    fn the_card_dissolves_into_the_post_while_the_window_is_still_growing() {
        // The window grows over the whole open, so the card is dissolving into
        // the post only when the dissolve starts after the start and ends before
        // the end: while the card is growing, not before or after.
        let dissolve = post_dissolve();

        assert!(dissolve.start > 0.0 && dissolve.end < 1.0, "{dissolve:?}");
        // It is slow: a good part of the open, not a flash.
        assert!(dissolve.end - dissolve.start >= 0.4, "{dissolve:?}");
    }

    fn page(route: AppRouter) -> AppPage {
        AppPage(route)
    }

    fn post() -> AppPage {
        page(AppRouter::PostDetail { slug: ShareRef::from_static("a"), id: 1 })
    }

    fn profile() -> AppPage {
        page(AppRouter::UserProfile { username: ShareRef::from_static("alice") })
    }

    #[test]
    fn a_post_opened_from_a_profile_slides_by_view_level_and_pops_back_the_same_way() {
        // Opening: the profile is the page being left. Going back: it is the one
        // being returned to. Either way the profile is on the other side.
        assert_eq!(post_motion(&profile(), &post()), PostMotion::SlideByLevel);
        assert_eq!(post_motion(&post(), &profile()), PostMotion::SlideByLevel);
    }

    #[test]
    fn a_post_opened_from_the_search_slides_by_view_level_and_pops_back_the_same_way() {
        let search = page(AppRouter::Search);
        assert_eq!(post_motion(&search, &post()), PostMotion::SlideByLevel);
        assert_eq!(post_motion(&post(), &search), PostMotion::SlideByLevel);
        assert!(post().view_level() > search.view_level());
    }

    #[test]
    fn a_post_opened_from_anywhere_else_still_grows_out_of_its_card() {
        for other in [
            page(AppRouter::Latest),
            page(AppRouter::New),
            page(AppRouter::Hot),
            page(AppRouter::CategoryTopics { slug: ShareRef::from_static("c"), id: 2 }),
            page(AppRouter::Settings),
        ] {
            assert_eq!(post_motion(&other, &post()), PostMotion::Expand);
            assert_eq!(post_motion(&post(), &other), PostMotion::Expand);
        }
        // Another post, as from a link in one, has no card to grow from either way.
        assert_eq!(post_motion(&post(), &post()), PostMotion::Expand);
    }

    #[test]
    fn a_post_is_a_level_deeper_than_the_profile_so_it_enters_from_the_right() {
        // `ViewLevelTransitions` slides in from the right for a deeper page and
        // out to the right when it is left, which is what a back gesture expects.
        assert!(post().view_level() > profile().view_level());
        assert!(profile().view_level() > page(AppRouter::Latest).view_level());
    }

    /// A finger on the window: goes down at `from`, travels to `to` in `steps`
    /// moves with a frame after each, and lifts there.
    fn swipe(app: &mut aimer::HeadlessAimerApp<impl aimer::Widget + 'static>, from: (f32, f32), to: (f32, f32), steps: u32) {
        use aimer::quiver::winit::dpi::PhysicalPosition;
        use aimer::quiver::winit::event::{DeviceId, Touch, TouchPhase, WindowEvent};

        let touch = |phase, at: (f32, f32)| WindowEvent::Touch(Touch {
            device_id: DeviceId::dummy(),
            phase,
            location: PhysicalPosition::new(at.0 as f64, at.1 as f64),
            force: None,
            id: 1,
        });
        app.send_window_event(touch(TouchPhase::Started, from));
        app.pump_frames(1);
        for step in 1..=steps {
            let t = step as f32 / steps as f32;
            app.send_window_event(touch(TouchPhase::Moved, (from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t)));
            app.pump_frames(1);
        }
        app.send_window_event(touch(TouchPhase::Ended, to));
    }

    /// Opens the post of topic 2 from the feed, then drags a finger rightward
    /// from `from` to well past halfway across the page, and says whether the feed
    /// is back.
    fn feed_is_back_after_swiping_the_post_from(from: (f32, f32)) -> bool {
        use aimer::animation::shared_element_bounds;

        let mut app = feed_app(3);
        let card = shared_element_bounds(&topic_tag(2)).expect("the card is on screen");
        click(&mut app, (card.x + card.width / 2.0, card.y + card.height / 2.0));
        assert!(settle(&mut app) > 5, "the post opens");
        assert!(shared_element_bounds(&topic_tag(2)).is_none(), "the feed is gone while the post is open");

        swipe(&mut app, from, (400.0, from.1), 12);
        settle(&mut app);
        app.pump_frames(3);

        shared_element_bounds(&topic_tag(2)).is_some()
    }

    /// A two-finger drag on a trackpad: the fingers go down, travel `dx` pixels
    /// sideways in `steps` events with a frame after each, rest for `rest` if they
    /// are to lift without any speed, and lift.
    fn trackpad_drag(
        app: &mut aimer::HeadlessAimerApp<impl aimer::Widget + 'static>,
        dx: f64,
        steps: u32,
        rest: Duration,
    ) {
        use aimer::quiver::winit::dpi::PhysicalPosition;
        use aimer::quiver::winit::event::{DeviceId, MouseScrollDelta, TouchPhase, WindowEvent};

        let wheel = |phase, dx: f64| WindowEvent::MouseWheel {
            device_id: DeviceId::dummy(),
            delta: MouseScrollDelta::PixelDelta(PhysicalPosition::new(dx, 0.0)),
            phase,
        };
        let step = |app: &mut aimer::HeadlessAimerApp<_>| {
            std::thread::sleep(Duration::from_millis(8));
            app.pump_frames(1);
        };
        // A trackpad announces a new touch twice: may-begin, then begin.
        for _ in 0..2 {
            app.send_window_event(wheel(TouchPhase::Started, 2.0));
            step(app);
        }
        for _ in 0..steps {
            app.send_window_event(wheel(TouchPhase::Moved, dx / steps as f64));
            step(app);
        }
        // Real time is what a flick's speed is measured in, so a drag that must not
        // be one rests until none of its travel is recent, whatever the machine is
        // doing meanwhile.
        // Frames go on while they rest: the smoother hands the distance it was given
        // to the page over frames, and a display does not stop for a pause.
        let started = std::time::Instant::now();
        while started.elapsed() < rest {
            std::thread::sleep(Duration::from_millis(8));
            app.pump_frames(1);
        }
        app.send_window_event(wheel(TouchPhase::Ended, 0.0));
        step(app);
    }

    #[test]
    fn a_two_finger_trackpad_swipe_right_on_a_post_brings_the_feed_back() {
        use aimer::animation::shared_element_bounds;

        let mut app = feed_app(3);
        let card = shared_element_bounds(&topic_tag(2)).expect("the card is on screen");
        click(&mut app, (card.x + card.width / 2.0, card.y + card.height / 2.0));
        assert!(settle(&mut app) > 5, "the post opens");
        assert!(shared_element_bounds(&topic_tag(2)).is_none(), "the feed is gone while the post is open");

        // Well past the travel a full pop takes on a 450 px page.
        trackpad_drag(&mut app, 400.0, 16, Duration::ZERO);
        settle(&mut app);
        app.pump_frames(3);

        assert!(shared_element_bounds(&topic_tag(2)).is_some(), "the swipe did not take the post back to the feed");
    }

    #[test]
    fn a_two_finger_trackpad_swipe_that_stops_short_leaves_the_post_where_it_is() {
        use aimer::animation::shared_element_bounds;

        let mut app = feed_app(3);
        let card = shared_element_bounds(&topic_tag(2)).expect("the card is on screen");
        click(&mut app, (card.x + card.width / 2.0, card.y + card.height / 2.0));
        assert!(settle(&mut app) > 5, "the post opens");

        // A little way, and the fingers rest before they lift.
        trackpad_drag(&mut app, 40.0, 16, Duration::from_millis(300));
        settle(&mut app);
        app.pump_frames(3);

        assert!(shared_element_bounds(&topic_tag(2)).is_none(), "the post is still open");
    }

    #[test]
    fn swiping_right_from_the_left_edge_of_a_post_brings_the_feed_back() {
        assert!(feed_is_back_after_swiping_the_post_from((8.0, 400.0)));
    }

    #[test]
    fn swiping_right_from_the_middle_of_a_post_brings_the_feed_back() {
        assert!(feed_is_back_after_swiping_the_post_from((150.0, 400.0)));
    }

    #[test]
    fn every_route_shares_the_shell_boundary() {
        assert_eq!(AppRouter::Latest.shell_key(), AppRouter::Settings.shell_key());
        assert!(AppRouter::Latest.shell_key().is_some());
    }

    #[test]
    fn only_post_detail_zooms_and_the_outlet_route_has_no_shell_key() {
        let post = AppPage(AppRouter::PostDetail { slug: ShareRef::from_static("a"), id: 1 });
        assert!(matches!(post.navigation_transition(), TransitionChoice::Custom(_)));
        assert!(matches!(AppPage(AppRouter::Hot).navigation_transition(), TransitionChoice::Inherit));
        // A shell key here would make the outlet skip its animation.
        assert!(post.shell_key().is_none());
    }

    #[test]
    fn the_sign_in_pages_slide_in_and_the_account_page_before_them_does_not() {
        for route in [AppRouter::Login, AppRouter::LoginEmail, AppRouter::LoginPassword] {
            assert!(matches!(AppPage(route).navigation_transition(), TransitionChoice::Custom(_)));
        }
        assert!(matches!(AppPage(AppRouter::Account).navigation_transition(), TransitionChoice::Inherit));
    }
}
