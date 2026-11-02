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
use aimer::widget::{AnyElement};
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
            AppRouter::Account => AccountScreen::page().boxed(),
            AppRouter::Login => LoginScreen::page().boxed(),
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
