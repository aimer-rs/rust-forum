use crate::api::Api;
use crate::app_shell::{MOBILE_TOP, is_mobile};
use crate::components::avatar::round_avatar;
use crate::components::search_header::STACKED_EXTRA;
use crate::components::topic_card::{TopicCard, divider_decor};
use crate::models::search::{
    BlogHit, SearchMode, SearchResults, SearchRoot, UserHit, UserSearchRoot,
};
use crate::router::AppRouter;
use crate::store::AppStore;
use crate::theme::ForumTheme;
use aimer::animation::{AnimatedBuilder, AnimatedSwitcher, AnimationController, curves};
use aimer::input::button::Button;
use aimer::provider::media_query::MediaQuery;
use aimer::router::NavigatorController;
use aimer::share::{ShareRef, Shared};
use aimer::style::{LayoutSpacing, TextAlign, TextOverflow, TextStyle, Theme};
use aimer::{
    AnyWidget, AsyncBuilder, AsyncSnapshot, BoxAlignment, BuildContext, Column, Container,
    Dimension, Expanded, FontWeight, Icons, Opacity, ProviderContext, ProviderHandle, Row,
    Scalable, Scrollable, SizedBox, State, StateUpdater, StatefulWidget, StatelessWidget, Svg,
    SvgStyle, Text, Widget, case,
};
use std::f32::consts::TAU;
use std::time::Duration;

/// How long typing has to pause before the words are searched for, so that a
/// word typed in a burst is one request and not one per letter.
const DEBOUNCE: Duration = Duration::from_millis(300);
/// How long the page takes to cross-fade between one state and the next.
const CONTENT_FADE: Duration = Duration::from_millis(200);
/// The most people asked for at once.
const USER_LIMIT: usize = 30;
const AVATAR_SIZE: f32 = 44.0;
/// The magnifier over the message in the middle of the page, and in the row over
/// results that are being replaced.
const GLYPH_SIZE: u32 = 56;
const ROW_GLYPH_SIZE: u32 = 18;
/// A full swell and shrink of the magnifier while a search is out.
const PULSE_CYCLE: Duration = Duration::from_millis(1200);
/// The room under the bar that is not the page's: the bar's space above, and
/// the space left at the bottom for the navigation.
const BOTTOM_SPACE: f32 = 90.0;

/// The search page: the results of what the bar over it holds.
///
/// The field and the Blog / User toggle are in the shell's bar (see
/// `SearchHeader`) and meet the page in the store, which keeps the term and the
/// mode. The page searches the blog (topics) or the people, whichever the toggle
/// says, once typing pauses.
#[derive(Clone, StatelessWidget)]
pub struct SearchScreen;

impl StatelessWidget for SearchScreen {
    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme = *ForumTheme::of(ctx);
        let store = ProviderHandle::<AppStore>::of(ctx);
        let term = ctx.select::<AppStore, String>(|s| s.search_term.trim().to_string());
        let mode = ctx.select::<AppStore, SearchMode>(|s| s.search_mode);

        // The bar is two rows tall on a narrow window, under the status bar.
        let top = case!(is_mobile(ctx), MOBILE_TOP + STACKED_EXTRA as u32) as f32 + 70.0;
        let window = MediaQuery::select(ctx, |media| media.size.height);
        // What a message has to centre itself in: the window under the bar.
        let room = (window - top - BOTTOM_SPACE).max(240.0);

        let body = if term.is_empty() {
            AnimatedSwitcher::new(
                CONTENT_FADE,
                curves::EASE_IN_OUT,
                message(theme, room, hint(mode)),
            )
            .child_key(format!("{mode:?}-idle"))
            .key("search-switcher-idle")
            .boxed()
        } else {
            let cached = store.read().cached_search(&term, mode);
            let kept = store.read().last_search(mode);
            let key = format!("search-{mode:?}-{term}");
            let searched = term.clone();
            // The switcher belongs to the search it shows, as the search below it
            // is rebuilt for each: one key for all of them would be claimed by the
            // old search and the new one at once.
            let switcher_key = format!("search-switcher-{key}");
            AsyncBuilder::new()
                .request_key(key.clone())
                .key(key)
                .future(move || {
                    let store = store.clone();
                    let term = searched.clone();
                    async move { fetch_search(store, term, mode).await }
                })
                .child(move |snapshot| {
                    // A term already searched shows from the first frame. A new
                    // one keeps the last results up while it loads, so typing
                    // doesn't blink the page empty between every pause.
                    let (phase, content) = match snapshot {
                        AsyncSnapshot::Waiting => match (cached.as_ref(), kept.as_ref()) {
                            (Some(results), _) => ("results", results_list(theme, room, results)),
                            (None, Some(results)) => (
                                "results",
                                Column::new()
                                    .children([
                                        searching_row(theme),
                                        results_list(theme, room, results),
                                    ])
                                    .boxed(),
                            ),
                            (None, None) => (
                                "loading",
                                placeholder(
                                    theme,
                                    room,
                                    PulsingGlyph::new(theme, GLYPH_SIZE).boxed(),
                                    "Searching…",
                                ),
                            ),
                        },
                        AsyncSnapshot::Error(error) => ("error", message(theme, room, error)),
                        AsyncSnapshot::Data(results) => {
                            ("results", results_list(theme, room, results))
                        }
                    };
                    AnimatedSwitcher::new(CONTENT_FADE, curves::EASE_IN_OUT, content)
                        .child_key(phase)
                        .key(switcher_key.clone())
                        .boxed()
                })
                .boxed()
        };

        Container::new().color(theme.background_color()).child(
            Scrollable::new()
                .key("search-scroll")
                .box_child(Column::new().children([
                    SizedBox::new().height(top).boxed(),
                    body,
                    SizedBox::new().height(BOTTOM_SPACE).boxed(),
                ])),
        )
    }
}

/// What the page says before anything is typed.
fn hint(mode: SearchMode) -> &'static str {
    match mode {
        SearchMode::Blog => "Search the forum's topics.",
        SearchMode::User => "Search the forum's members.",
    }
}

/// Searches for `term`, once typing has paused, and keeps what it finds.
async fn fetch_search(
    store: ProviderHandle<AppStore>,
    term: String,
    mode: SearchMode,
) -> Result<SearchResults, String> {
    if let Some(results) = store.read().cached_search(&term, mode) {
        return Ok(results);
    }
    // A newer term replaces this request, and with it this wait, so only a term
    // that was left alone is ever sent.
    tokio::time::sleep(DEBOUNCE).await;

    let path = search_path(&term, mode);
    let results = match mode {
        SearchMode::Blog => SearchResults::Blog(
            Api::get::<SearchRoot>(&path)
                .await
                .map_err(|e| format!("Error: {e}"))?
                .blog_hits(),
        ),
        SearchMode::User => SearchResults::User(
            Api::get::<UserSearchRoot>(&path)
                .await
                .map_err(|e| format!("Error: {e}"))?
                .user_hits(),
        ),
    };

    store.update(|s| s.cache_search(&term, mode, results.clone()));
    Ok(results)
}

/// The request that searches for `term` in `mode`.
///
/// Both paths end in `.json`: without it Discourse answers the same address
/// with the HTML page, which is no use to the parser.
fn search_path(term: &str, mode: SearchMode) -> String {
    let query = encode_term(term);
    match mode {
        SearchMode::Blog => format!("search/query.json?term={query}"),
        SearchMode::User => format!("u/search/users.json?term={query}&limit={USER_LIMIT}"),
    }
}

/// `term` as it can sit in a URL's query: everything but letters, digits and
/// `-._~` as `%XX` bytes of its UTF-8.
fn encode_term(term: &str) -> String {
    let mut encoded = String::with_capacity(term.len());
    for byte in term.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char)
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

/// The magnifier, still.
fn glyph(theme: ForumTheme, size: u32) -> Svg {
    Svg::new(Icons::SEARCH)
        .style(
            "path",
            SvgStyle::new().stroke(theme.on_background_color().with_opacity(140)),
        )
        .width(size)
        .height(size)
}

/// `icon` over `text`, in the middle of `room` pixels of the page's height.
fn placeholder(theme: ForumTheme, room: f32, icon: AnyWidget, text: &str) -> AnyWidget {
    Container::new()
        .width(Dimension::Percent(100.0))
        .height(Dimension::Px(room))
        .child(
            Column::new()
                .horizontal_alignment(BoxAlignment::Center)
                .vertical_alignment(BoxAlignment::Center)
                .gaps(LayoutSpacing::vertical(16))
                .children([
                    icon,
                    Container::new()
                        .padding(LayoutSpacing::horizontal(24.into()))
                        .box_child(
                            Text::new(text.to_string())
                                .text_align(TextAlign::MidCenter)
                                .text_style(
                                    TextStyle::new()
                                        .text_overflow(TextOverflow::Wrap)
                                        .font_size(16)
                                        .color(theme.on_background_color().with_opacity(160)),
                                ),
                        ),
                ]),
        )
        .boxed()
}

/// What the page says in its middle: the magnifier, still, over `text`.
fn message(theme: ForumTheme, room: f32, text: &str) -> AnyWidget {
    placeholder(theme, room, glyph(theme, GLYPH_SIZE).boxed(), text)
}

/// The line over the last results while the next are on their way.
fn searching_row(theme: ForumTheme) -> AnyWidget {
    Container::new()
        .padding(LayoutSpacing::vertical(12).left(24).right(24))
        .child(
            Row::new()
                .vertical_alignment(BoxAlignment::Center)
                .gaps(LayoutSpacing::horizontal(10.into()))
                .children([
                    PulsingGlyph::new(theme, ROW_GLYPH_SIZE).boxed(),
                    Text::new("Searching…")
                        .text_style(
                            TextStyle::new()
                                .font_size(14)
                                .color(theme.on_background_color().with_opacity(160)),
                        )
                        .boxed(),
                ]),
        )
        .boxed()
}

/// The magnifier, swelling and fading in and out for as long as it is on the
/// page: a search is out.
#[derive(Clone, StatefulWidget)]
struct PulsingGlyph {
    theme: ForumTheme,
    size: u32,
}

impl PulsingGlyph {
    fn new(theme: ForumTheme, size: u32) -> Self {
        Self { theme, size }
    }
}

struct PulsingGlyphState {
    theme: ForumTheme,
    size: u32,
    pulse: AnimationController,
}

impl StatefulWidget for PulsingGlyph {
    type State = PulsingGlyphState;

    fn create_state(self) -> Self::State {
        let pulse = AnimationController::new(PULSE_CYCLE, curves::LINEAR);
        pulse.set_repeat(true);
        pulse.forward_from_first_tick();
        PulsingGlyphState {
            theme: self.theme,
            size: self.size,
            pulse,
        }
    }
}

impl State<PulsingGlyph> for PulsingGlyphState {
    fn init_state(&mut self, _updater: StateUpdater<Self>) {}

    fn adopt_config_from(&mut self, new: Self) {
        self.theme = new.theme;
        self.size = new.size;
    }

    fn build(&self, _ctx: &BuildContext) -> impl Widget {
        let (theme, size) = (self.theme, self.size);
        AnimatedBuilder::new(self.pulse.clone(), move |turn| {
            // 0 to 1 and back over one turn, smoothly.
            let swell = 0.5 - 0.5 * (turn * TAU).cos();
            Opacity::new().opacity(0.5 + 0.5 * swell).child(
                Scalable::new()
                    .scale(0.88 + 0.24 * swell)
                    .child(glyph(theme, size)),
            )
        })
    }
}

fn results_list(theme: ForumTheme, room: f32, results: &SearchResults) -> AnyWidget {
    if results.is_empty() {
        return message(theme, room, "Nothing found.");
    }
    match results {
        SearchResults::Blog(hits) => Column::new()
            .children(
                hits.iter()
                    .map(|hit| TopicCard::new(Shared::new(hit.to_topic()).as_share_ref()).boxed())
                    .collect::<Vec<_>>(),
            )
            .boxed(),
        SearchResults::User(hits) => Column::new()
            .children(
                hits.iter()
                    .cloned()
                    .map(|hit| UserRow { hit }.boxed())
                    .collect::<Vec<_>>(),
            )
            .boxed(),
    }
}

/// A member that matched: their avatar, username and name. Pressing it opens
/// their profile.
#[derive(Clone, StatelessWidget)]
struct UserRow {
    hit: UserHit,
}

impl StatelessWidget for UserRow {
    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme = *ForumTheme::of(ctx);
        let navigator = NavigatorController::<AppRouter>::of(ctx);
        let hit = self.hit.clone();
        let username = ShareRef::from_str_slice(&hit.username);

        let mut lines = vec![
            Text::new(hit.username.clone())
                .text_style(
                    TextStyle::new()
                        .font_weight(FontWeight::Bold)
                        .font_size(17)
                        .color(theme.on_background_color()),
                )
                .boxed(),
        ];
        if !hit.name.is_empty() {
            lines.push(
                Text::new(hit.name.clone())
                    .text_style(
                        TextStyle::new()
                            .font_size(14)
                            .color(theme.on_background_color().with_opacity(160)),
                    )
                    .boxed(),
            );
        }

        Button::new()
            .decoration(divider_decor(theme))
            .hover_decoration(
                divider_decor(theme).background_color(theme.primary_color().with_opacity(10)),
            )
            .on_press(move || {
                navigator.push(AppRouter::UserProfile {
                    username: username.clone(),
                })
            })
            .child(
                Container::new()
                    .padding(LayoutSpacing::vertical(12).left(24).right(24))
                    .child(
                        Row::new()
                            .vertical_alignment(BoxAlignment::Center)
                            .gaps(LayoutSpacing::horizontal(14.into()))
                            .children([
                                round_avatar(theme, &hit.avatar_url, &hit.username, AVATAR_SIZE),
                                Expanded::new().box_child(
                                    Column::new().gaps(LayoutSpacing::all(2)).children(lines),
                                ),
                            ]),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_term_is_made_safe_for_a_url() {
        assert_eq!(encode_term("apple"), "apple");
        assert_eq!(encode_term("async fn"), "async%20fn");
        assert_eq!(encode_term("a&b=c#d"), "a%26b%3Dc%23d");
        assert_eq!(encode_term("é"), "%C3%A9");
    }

    #[test]
    fn searches_ask_for_json() {
        assert_eq!(
            search_path("a b", SearchMode::Blog),
            "search/query.json?term=a%20b"
        );
        assert!(search_path("dom", SearchMode::User).starts_with("u/search/users.json?term=dom&"));
    }

    #[test]
    fn each_mode_has_its_own_hint() {
        assert_ne!(hint(SearchMode::Blog), hint(SearchMode::User));
    }

    /// The app on the search page, with `term` in the bar and `seed` run on the
    /// store first.
    fn search_app(
        width: u32,
        term: &str,
        mode: SearchMode,
        seed: impl FnOnce(&mut AppStore),
    ) -> aimer::HeadlessAimerApp<impl Widget + 'static> {
        use aimer::quiver::winit::dpi::PhysicalSize;
        use aimer::router::Navigator;
        use aimer::{AimerApp, HeadlessOptions, Provider};

        let mut store = AppStore::default();
        store.search_term = term.to_string();
        store.search_mode = mode;
        seed(&mut store);
        crate::testing::hold();
        let mut app = AimerApp::start_headless_with(
            Provider::new()
                .create(move || store.clone())
                .child(Navigator::new(AppRouter::Search, |r| r.boxed())),
            HeadlessOptions {
                size: PhysicalSize::new(width, 900),
                scale_factor: 1.0,
            },
        );
        for _ in 0..10 {
            std::thread::sleep(Duration::from_millis(20));
            app.pump_frames(3);
        }
        app
    }

    fn painted(app: &aimer::HeadlessAimerApp<impl Widget + 'static>) -> usize {
        app.paint_source_census()
            .expect("a mounted page has a root")
            .local_v2
    }

    fn blog_hits(count: i64) -> SearchResults {
        SearchResults::Blog(
            (1..=count)
                .map(|id| BlogHit {
                    id,
                    slug: format!("topic-{id}"),
                    title: format!("Topic {id}"),
                    blurb: "an apple a day".into(),
                    username: "ferris".into(),
                    created_at: "2024-01-01T00:00:00Z".into(),
                    posts_count: 3,
                    ..Default::default()
                })
                .collect(),
        )
    }

    #[test]
    fn cached_results_are_painted_from_the_first_frames() {
        for width in [450, 1100] {
            let idle = search_app(width, "", SearchMode::Blog, |_| {});
            let found = search_app(width, "apple", SearchMode::Blog, |s| {
                s.cache_search("apple", SearchMode::Blog, blog_hits(4))
            });
            assert!(
                painted(&found) >= painted(&idle) + 20,
                "width {width}: {} nodes with results, {} without",
                painted(&found),
                painted(&idle)
            );
        }
    }

    #[test]
    fn people_are_listed_too() {
        let users = SearchResults::User(
            (1..=3)
                .map(|n| UserHit {
                    username: format!("user{n}"),
                    name: format!("User {n}"),
                    avatar_url: String::new(),
                })
                .collect(),
        );
        let idle = search_app(450, "", SearchMode::User, |_| {});
        let found = search_app(450, "user", SearchMode::User, |s| {
            s.cache_search("user", SearchMode::User, users)
        });
        assert!(
            painted(&found) >= painted(&idle) + 15,
            "{} vs {}",
            painted(&found),
            painted(&idle)
        );
    }

    #[test]
    fn a_search_that_found_nothing_says_so() {
        let empty = search_app(450, "zzz", SearchMode::Blog, |s| {
            s.cache_search("zzz", SearchMode::Blog, SearchResults::Blog(vec![]))
        });
        let rows = search_app(450, "zzz", SearchMode::Blog, |s| {
            s.cache_search("zzz", SearchMode::Blog, blog_hits(4))
        });
        assert!(painted(&rows) > painted(&empty));
    }

    /// Asks the live forum; run with `cargo test -- --ignored`.
    #[tokio::test]
    #[ignore = "needs the network"]
    async fn the_forum_answers_both_searches() {
        let blog = Api::get::<SearchRoot>(&search_path("apple", SearchMode::Blog))
            .await
            .unwrap();
        assert!(!blog.blog_hits().is_empty());
        let users = Api::get::<UserSearchRoot>(&search_path("dom", SearchMode::User))
            .await
            .unwrap();
        assert!(!users.user_hits().is_empty());
    }

    thread_local! {
        /// The store the test app runs on, to change what is searched while it runs.
        static STORE: std::cell::RefCell<Option<ProviderHandle<AppStore>>> = const { std::cell::RefCell::new(None) };
    }

    /// The app's root for [`STORE`]: it keeps hold of the store and shows the page.
    #[derive(Clone, StatelessWidget)]
    struct Grab;

    impl StatelessWidget for Grab {
        fn build(&self, ctx: &BuildContext) -> impl Widget {
            STORE.with(|store| *store.borrow_mut() = Some(ProviderHandle::<AppStore>::of(ctx)));
            aimer::router::Navigator::new(AppRouter::Search, |r| r.boxed())
        }
    }

    thread_local! {
        /// The navigator of the test app, to move between pages while it runs.
        static NAVIGATOR: std::cell::RefCell<Option<NavigatorController<AppRouter>>> = const { std::cell::RefCell::new(None) };
    }

    /// The app's page: it keeps hold of the navigator and shows the route.
    #[derive(Clone, StatelessWidget)]
    struct Probe(AppRouter);

    impl StatelessWidget for Probe {
        fn build(&self, ctx: &BuildContext) -> impl Widget {
            NAVIGATOR.with(|navigator| {
                *navigator.borrow_mut() = Some(NavigatorController::<AppRouter>::of(ctx))
            });
            self.0.clone().boxed()
        }
    }

    #[test]
    fn coming_back_to_the_search_page_never_claims_a_key_twice() {
        use aimer::quiver::winit::dpi::PhysicalSize;
        use aimer::router::Navigator;
        use aimer::{AimerApp, HeadlessOptions, Provider};

        for width in [450, 1100] {
            let mut store = AppStore::default();
            store.search_term = "search".to_string();
            store.search_mode = SearchMode::Blog;
            store.cache_search("search", SearchMode::Blog, blog_hits(3));
            crate::testing::hold();
            let mut app = AimerApp::start_headless_with(
                Provider::new()
                    .create(move || store.clone())
                    .child(Navigator::new(AppRouter::Search, |r| Probe(r).boxed())),
                HeadlessOptions {
                    size: PhysicalSize::new(width, 900),
                    scale_factor: 1.0,
                },
            );
            let frames = |app: &mut aimer::HeadlessAimerApp<_>| {
                for _ in 0..40 {
                    std::thread::sleep(Duration::from_millis(20));
                    app.pump_frames(2);
                }
            };
            frames(&mut app);
            let navigator = NAVIGATOR
                .with(|n| n.borrow().clone())
                .expect("the page was built");
            let post = AppRouter::PostDetail {
                slug: ShareRef::from_static("topic-1"),
                id: 1,
            };

            let field_has_focus = || {
                crate::components::search_header::LAST_FIELD_NODE
                    .with(|node| node.borrow().as_ref().map(|node| node.has_focus()))
                    .expect("the search field was built")
            };
            // Opened from nowhere in particular, the search takes the keyboard.
            assert!(field_has_focus(), "width {width}: the field should take the keyboard on arrival");

            // Open a post from the search, and come back to the search by the
            // navigation, as the tab bar and the sidebar do.
            navigator.push(post.clone());
            frames(&mut app);
            navigator.set_route(AppRouter::Search);
            frames(&mut app);
            // And the same through the way back.
            navigator.push(post.clone());
            frames(&mut app);
            navigator.pop();
            frames(&mut app);
            navigator.push(AppRouter::Search);
            frames(&mut app);
        }
    }

    #[test]
    fn touching_the_back_button_of_a_post_does_not_put_the_keyboard_in_the_field_under_it() {
        use aimer::quiver::winit::dpi::{PhysicalPosition, PhysicalSize};
        use aimer::quiver::winit::event::{DeviceId, Touch, TouchPhase, WindowEvent};
        use aimer::router::Navigator;
        use aimer::{AimerApp, HeadlessOptions, Provider};

        let mut store = AppStore::default();
        store.search_term = "search".to_string();
        store.cache_search("search", SearchMode::Blog, blog_hits(3));
        crate::testing::hold();
        let mut app = AimerApp::start_headless_with(
            Provider::new()
                .create(move || store.clone())
                .child(Navigator::new(AppRouter::Search, |r| Probe(r).boxed())),
            HeadlessOptions { size: PhysicalSize::new(450, 900), scale_factor: 1.0 },
        );
        let frames = |app: &mut aimer::HeadlessAimerApp<_>| {
            for _ in 0..40 {
                std::thread::sleep(Duration::from_millis(20));
                app.pump_frames(2);
            }
        };
        frames(&mut app);
        let node = || {
            crate::components::search_header::LAST_FIELD_NODE
                .with(|node| node.borrow().clone())
                .expect("the search field was built")
        };

        let navigator = NAVIGATOR.with(|n| n.borrow().clone()).expect("the page was built");
        navigator.push(AppRouter::PostDetail { slug: ShareRef::from_static("topic-1"), id: 1 });
        frames(&mut app);

        // The back button is the round one at the bar's left end, under the status
        // bar's room and the bar's padding. Tapped with a finger and with a mouse:
        // the field that is under it once the page is back must take neither.
        let at = (10.0 + 20.0, MOBILE_TOP as f64 + 10.0 + 20.0);
        let touch = |phase| WindowEvent::Touch(Touch {
            device_id: DeviceId::dummy(),
            phase,
            location: PhysicalPosition::new(at.0, at.1),
            force: None,
            id: 1,
        });
        for phase in [TouchPhase::Started, TouchPhase::Ended] {
            app.send_window_event(touch(phase));
            app.pump_frames(2);
        }
        frames(&mut app);

        assert!(navigator.current_route() == AppRouter::Search, "the tap did not go back");
        assert!(!node().has_focus(), "the tap that went back put the keyboard in the field");
    }

    #[test]
    fn coming_back_from_a_post_to_the_search_does_not_take_the_keyboard() {
        use aimer::quiver::winit::dpi::PhysicalSize;
        use aimer::router::Navigator;
        use aimer::{AimerApp, HeadlessOptions, Provider};

        for width in [450, 1100] {
            let mut store = AppStore::default();
            store.search_term = "search".to_string();
            store.cache_search("search", SearchMode::Blog, blog_hits(3));
            crate::testing::hold();
            let mut app = AimerApp::start_headless_with(
                Provider::new()
                    .create(move || store.clone())
                    .child(Navigator::new(AppRouter::Search, |r| Probe(r).boxed())),
                HeadlessOptions { size: PhysicalSize::new(width, 900), scale_factor: 1.0 },
            );
            let frames = |app: &mut aimer::HeadlessAimerApp<_>| {
                for _ in 0..40 {
                    std::thread::sleep(Duration::from_millis(20));
                    app.pump_frames(2);
                }
            };
            frames(&mut app);
            let node = || {
                crate::components::search_header::LAST_FIELD_NODE
                    .with(|node| node.borrow().clone())
                    .expect("the search field was built")
            };
            assert!(node().has_focus(), "width {width}: the field takes the keyboard on arrival");

            let navigator = NAVIGATOR.with(|n| n.borrow().clone()).expect("the page was built");
            navigator.push(AppRouter::PostDetail { slug: ShareRef::from_static("topic-1"), id: 1 });
            frames(&mut app);
            navigator.pop();
            frames(&mut app);
            assert!(!node().has_focus(), "width {width}: the field took the keyboard coming back from the post");
        }
    }

    #[test]
    fn searching_for_one_thing_after_another_never_claims_a_key_twice() {
        use aimer::quiver::winit::dpi::PhysicalSize;
        use aimer::{AimerApp, HeadlessOptions, Provider};

        let mut seeded = AppStore::default();
        for (term, mode, results) in [
            ("apple", SearchMode::Blog, blog_hits(3)),
            ("pear", SearchMode::Blog, blog_hits(2)),
            (
                "apple",
                SearchMode::User,
                SearchResults::User(vec![UserHit::default()]),
            ),
        ] {
            seeded.cache_search(term, mode, results);
        }
        crate::testing::hold();
        let mut app = AimerApp::start_headless_with(
            Provider::new().create(move || seeded.clone()).child(Grab),
            HeadlessOptions {
                size: PhysicalSize::new(450, 900),
                scale_factor: 1.0,
            },
        );
        let frames = |app: &mut aimer::HeadlessAimerApp<_>| {
            for _ in 0..8 {
                std::thread::sleep(Duration::from_millis(30));
                app.pump_frames(2);
            }
        };
        frames(&mut app);
        let store = STORE
            .with(|store| store.borrow().clone())
            .expect("the page was built");

        // From nothing typed to a search, from one search to another, to the other
        // mode, and back to nothing: each has rebuilt the page below the bar.
        for (term, mode) in [
            ("apple", SearchMode::Blog),
            ("pear", SearchMode::Blog),
            ("apple", SearchMode::Blog),
            ("apple", SearchMode::User),
            ("", SearchMode::User),
            ("pear", SearchMode::Blog),
        ] {
            store.update(|s| {
                s.search_term = term.to_string();
                s.search_mode = mode;
            });
            frames(&mut app);
        }

        // Typed quickly, each pause too short for the search to go out: the
        // page is rebuilt for every letter while the previous one is waiting.
        for term in [
            "r", "ru", "rus", "rust", "rust ", "rust a", "rust as", "rust asy",
        ] {
            store.update(|s| s.search_term = term.to_string());
            for _ in 0..2 {
                std::thread::sleep(Duration::from_millis(20));
                app.pump_frames(2);
            }
        }
    }
}
