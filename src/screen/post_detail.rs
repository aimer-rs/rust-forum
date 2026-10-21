mod comments;

use crate::app_shell::{MOBILE_TOP, is_mobile};
use crate::api::Api;
use crate::html_markdown::{forum_markdown, html_to_markdown};
use crate::models::posts::PostRoot;
use crate::screen::post_detail::comments::CommentsSection;
use crate::store::AppStore;
use crate::theme::ForumTheme;
use aimer::animation::{AnimatedBuilder, AnimatedSwitcher, AnimationController, FadeTransition, curves};
use aimer::console::error;
use aimer::share::ShareRef;
use aimer::style::{BoxDecoration, LayoutSpacing, TextOverflow, TextStyle, Theme};
use aimer::{
    AnyWidget, AsyncBuilder, AsyncSnapshot, BuildContext, Color, Column, Container, Dimension, FontWeight, ProviderHandle, ScrollController,
    Scrollable, SizedBox, Stack, State, StateUpdater, StatefulWidget, StatelessWidget, Text,
    Widget, ZeroSizedBox, case,
};
use std::rc::Rc;
use std::time::Duration;

/// How many post skeletons have been built, for tests that look for one flashing.
#[cfg(test)]
pub(crate) static BUILT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// How long the page takes to come up from transparent when it is first built.
///
/// The transition that opens a post paints the page for one frame before it
/// clips it to the card, so for that frame everything below the card shows. The
/// transition's own dissolve does not begin for a hundred milliseconds, so
/// holding the page back until then costs nothing that can be seen.
const REVEAL: Duration = Duration::from_millis(70);
/// How long the post takes to cross-fade in over its skeleton once it arrives.
const CONTENT_FADE: Duration = Duration::from_millis(250);
/// How long the post skeleton stays invisible before it starts to fade in: the
/// page it belongs to is still being laid into a transition, and a post that
/// loads within this never shows a skeleton at all.
const SKELETON_DELAY: Duration = Duration::from_millis(80);
/// How long the post skeleton then takes to fade in.
const SKELETON_FADE_IN: Duration = Duration::from_millis(150);

#[derive(Clone, StatefulWidget)]
pub struct PostDetailScreen {
    slug: ShareRef<str>,
    id: i64,
}

impl PostDetailScreen {
    pub fn new(slug: ShareRef<str>, id: i64) -> Self {
        Self { slug, id }
    }
}

pub struct PostDetailScreenState {
    parent: PostDetailScreen,
    scroll_controller: ScrollController,
    /// Holds the whole page transparent for its first frames; see [`REVEAL`].
    reveal: AnimationController,
    is_scrolling: bool,
}

impl StatefulWidget for PostDetailScreen {
    type State = PostDetailScreenState;

    fn create_state(self) -> Self::State {
        PostDetailScreenState {
            parent: self,
            scroll_controller: ScrollController::new(),
            reveal: {
                let reveal = AnimationController::new(REVEAL, curves::EASE_IN);
                reveal.forward_from_first_tick();
                reveal
            },
            is_scrolling: false,
        }
    }
}

impl State<PostDetailScreen> for PostDetailScreenState {
    fn init_state(&mut self, _updater: StateUpdater<Self>) {
        // self.scroll_controller.on_scroll(move |offset: Vec2d| {
        //     let current_scroll = updater.read_state().is_scrolling;
        //     let is_scrolled = offset.y > 50.0;
        //     if is_scrolled != current_scroll {
        //         updater.set_state(move |s| s.is_scrolling = is_scrolled)
        //     }
        // });
    }

    fn adopt_config_from(&mut self, new: Self) {
        let id_changed = self.parent.id != new.parent.id;
        self.parent = new.parent;

        if id_changed {
            self.is_scrolling = false;
        }
    }

    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme = *ForumTheme::of(ctx);
        let store = ProviderHandle::<AppStore>::of(ctx);
        // let navigator = NavigatorController::<AppRouter>::of(ctx);
        let slug = self.parent.slug.clone();
        let id = self.parent.id;
        // The card this page opened from already has the title: show it from the
        // first frame instead of bars, so the card visibly becomes the post.
        let known_title = store.read().topic_title(id);
        // A post seen before is already in the store. `AsyncBuilder` always
        // starts in `Waiting`, so without this a cached post would flash its
        // skeleton for a frame before the (instant) fetch delivers it.
        let cached = store.read().cached_post(id);
        let scroll_ctrler = self.scroll_controller.clone();

        let data = AsyncBuilder::new()
            .request_key(id)
            .key(format!("post-{id}"))
            .future(move || {
                let store = store.clone();
                let slug = slug.clone();
                async move { fetch_cached_post(store, slug, id).await }
            })
            .child(move |snapshot| {
                // The skeleton and the post cross-fade when the post arrives. A
                // post that was already cached never shows the skeleton, so both
                // of its states share a key and nothing fades.
                let (phase, content) = match snapshot {
                    AsyncSnapshot::Waiting => match &cached {
                        Some(post) => ("content", post_body(post, id)),
                        None => (
                            "skeleton",
                            SkeletonPostContent::new(known_title.clone()).boxed(),
                        ),
                    },
                    AsyncSnapshot::Error(error) => {
                        error!("Error: {}", error);
                        (
                            "error",
                            Text::new(error.clone())
                                .text_style(TextStyle::new().color(theme.on_background_color()))
                                .boxed(),
                        )
                    }
                    AsyncSnapshot::Data(post) => {
                        ("content", post_body(post, id))
                    }
                };
                AnimatedSwitcher::new(CONTENT_FADE, curves::EASE_IN_OUT, content)
                    .child_key(phase)
                    .key(format!("post-switcher-{id}"))
                    .boxed()
            });

        FadeTransition::new(self.reveal.clone(), Container::new().color(theme.background_color()).child(
            Stack::new().children([
                Scrollable::new()
                    .controller(scroll_ctrler)
                    .box_child(Column::new().children([
                        SizedBox::new().height(case!(is_mobile(ctx), MOBILE_TOP) + 86).boxed(),
                        data.boxed(),
                        SizedBox::new().height(90).boxed(),
                    ])),
            ]),
        ))
    }
}

/// The post's title and text with its comments under it.
fn post_body(post: &PostRoot, id: i64) -> AnyWidget {
    let markdown = post
        .post_stream
        .posts
        .first()
        .map(|p| html_to_markdown(&p.cooked))
        .unwrap_or_default();

    Column::new()
        .children([
            PostContent {
                title: post.title.clone(),
                markdown,
            }
            .boxed(),
            CommentsSection::new(id, post.post_stream.clone()).boxed(),
        ])
        .boxed()
}

async fn fetch_cached_post(
    store: ProviderHandle<AppStore>,
    slug: ShareRef<str>,
    id: i64,
) -> Result<PostRoot, String> {
    if let Some(post) = store.read().cached_post(id) {
        return Ok(post);
    }

    let post = Api::get::<PostRoot>(&format!("t/{slug}/{id}.json"))
        .await
        .map_err(|e| format!("Error: {}", e))?;

    store.update(|s| s.cache_post(id, post.clone()));
    Ok(post)
}

/// Renders the post title and markdown. Reads the theme in its own `build` so
/// it re-renders when the theme changes, independent of the `AsyncBuilder`'s
/// snapshot (whose child closure only re-runs when the data changes).
#[derive(Clone, StatelessWidget)]
struct PostContent {
    title: String,
    markdown: Rc<str>,
}

impl StatelessWidget for PostContent {
    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme = *ForumTheme::of(ctx);

        Column::new().children([
            post_title(theme, self.title.clone()),
            forum_markdown(&self.markdown, &theme, LayoutSpacing::all(24)),
        ])
    }
}

/// The title of a post, as the page shows it both while the post loads and once
/// it has, so the two never differ by a pixel.
fn post_title(theme: ForumTheme, title: String) -> AnyWidget {
    Container::new()
        .padding(LayoutSpacing::horizontal(24.into()))
        .box_child(
            Text::new(title).text_style(
                TextStyle::new()
                    .text_overflow(TextOverflow::Wrap)
                    .font_weight(FontWeight::Bold)
                    .font_size(28)
                    .color(theme.on_background_color()),
            ),
        )
}

/// Pulsing placeholders shown while the post request is loading.
#[derive(StatelessWidget)]
struct SkeletonPostContent {
    controller: AnimationController,
    /// Keeps the bars clear for [`SKELETON_DELAY`], then brings them to full over
    /// [`SKELETON_FADE_IN`].
    fade_in: AnimationController,
    /// The title, when it is already known; bars stand in for it otherwise.
    title: Option<String>,
}

impl SkeletonPostContent {
    fn new(title: Option<String>) -> Self {
        #[cfg(test)]
        BUILT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let controller = AnimationController::new(Duration::from_millis(1000), curves::EASE_IN_OUT);
        controller.set_repeat(true);
        controller.set_auto_reverse(true);
        controller.forward_from_first_tick();
        let fade_in = AnimationController::new(SKELETON_DELAY + SKELETON_FADE_IN, curves::LINEAR);
        fade_in.forward_from_first_tick();
        Self {
            controller,
            fade_in,
            title,
        }
    }
}

impl StatelessWidget for SkeletonPostContent {
    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme = *ForumTheme::of(ctx);
        let controller = self.controller.clone();
        let fade_in = self.fade_in.clone();
        let title = self.title.clone();
        AnimatedBuilder::new(controller, move |value| {
            let fade_in = fade_in.clone();
            let title = title.clone();
            AnimatedBuilder::new(fade_in, move |progress| {
            let elapsed = progress * (SKELETON_DELAY + SKELETON_FADE_IN).as_secs_f32();
            let fade = ((elapsed - SKELETON_DELAY.as_secs_f32()) / SKELETON_FADE_IN.as_secs_f32())
                .clamp(0.0, 1.0);
            let tone = theme.primary_color().with_brightness(1.1);
            let bar = tone.with_alpha((0.12 + 0.18 * value) * fade);

            Column::new().children([
                match title.clone() {
                    Some(title) => post_title(theme, title),
                    None => Container::new()
                        .padding(LayoutSpacing::horizontal(24.into()))
                        .child(
                            Column::new().gaps(LayoutSpacing::all(10)).children([
                                skeleton_bar(bar, 28.0, 75.0),
                                skeleton_bar(bar, 28.0, 45.0),
                            ]),
                        )
                        .boxed(),
                },
                Container::new().padding(LayoutSpacing::all(24)).child(
                    Column::new().gaps(LayoutSpacing::all(14)).children([
                        skeleton_bar(bar, 16.0, 100.0),
                        skeleton_bar(bar, 16.0, 96.0),
                        skeleton_bar(bar, 16.0, 87.0),
                        skeleton_bar(bar, 16.0, 100.0),
                        skeleton_bar(bar, 16.0, 66.0),
                        SizedBox::new().height(14).boxed(),
                        skeleton_bar(bar, 16.0, 92.0),
                        skeleton_bar(bar, 16.0, 100.0),
                        skeleton_bar(bar, 16.0, 72.0),
                    ]),
                )
                .boxed(),
            ])
            })
        })
    }
}

fn skeleton_bar(color: Color, height: f32, width_percent: f32) -> AnyWidget {
    Container::new()
        .height(Dimension::Px(height))
        .width(Dimension::Percent(width_percent))
        .box_decoration(
            BoxDecoration::new()
                .background_color(color)
                .border_radius(6),
        )
        .child(ZeroSizedBox)
        .boxed()
}
