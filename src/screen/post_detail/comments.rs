use crate::api::Api;
use crate::components::avatar::round_avatar;
use crate::components::profile_link::profile_link;
use crate::router::AppRouter;
use aimer::router::{Navigator, NavigatorController};
use crate::components::topic_card::{format_count, relative_time};
use crate::html_markdown::forum_markdown;
use crate::models::comments::{Comment, CommentNode, build_comment_tree};
use crate::models::posts::{Post, PostStream, PostsPage};
use crate::store::{AppStore, CommentThread, StreamStatus};
use crate::theme::ForumTheme;
use aimer::style::{
    BorderSlice, BorderStyle, BoxBorder, BoxDecoration, LayoutSpacing, TextStyle, Theme,
};
use aimer::{
    AnyWidget, AsyncBuilder, BoxAlignment, BuildContext, Column, Container, Expanded, FontWeight,
    Icons, ProviderContext, ProviderHandle, Row, SizedBox, StatelessWidget, StaticSvg, Svg,
    SvgStyle, Text, Venus, Widget, ZeroSizedBox,
};
use std::collections::HashSet;
use std::rc::Rc;

/// How many posts one `posts.json` request asks for.
const POSTS_PER_REQUEST: usize = 20;
const AVATAR_DIAMETER: f32 = 32.0;
const ACTION_ICON_SIZE: u32 = 18;
/// How far each level of replies is set in from the comment it answers.
const INDENT: u32 = 14;
/// How many levels of replies indent. Deeper ones stay at the last level's
/// indent so a long chain keeps a readable width.
const MAX_INDENT_DEPTH: usize = 5;

/// The comments under a topic's first post, nested by who replied to whom.
///
/// Replies start collapsed beneath the comment they answer. The topic document
/// only carries the first batch of posts, so the rest are fetched in the
/// background and appear as they arrive.
#[derive(StatelessWidget)]
pub struct CommentsSection {
    topic_id: i64,
    /// The first batch of posts plus the ids of every post in the topic.
    initial: PostStream,
}

impl CommentsSection {
    pub fn new(topic_id: i64, initial: PostStream) -> Self {
        Self { topic_id, initial }
    }
}

impl StatelessWidget for CommentsSection {
    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme = *ForumTheme::of(ctx);
        let store = ProviderHandle::<AppStore>::of(ctx);
        let topic_id = self.topic_id;
        let initial = self.initial.clone();
        let thread = ctx.select::<AppStore, CommentThread>(move |s| s.comment_thread(topic_id));

        let driver = AsyncBuilder::new()
            .request_key(topic_id)
            .key(format!("comments-{topic_id}"))
            .future(move || {
                let store = store.clone();
                let initial = initial.clone();
                async move { load_comments(store, topic_id, initial).await }
            })
            .child(|_| ZeroSizedBox.boxed());

        let tree = build_comment_tree(&thread.comments);
        let mut sections: Vec<AnyWidget> = vec![
            driver.boxed(),
            section_text(heading(thread.comments.len()), &theme, 20, FontWeight::Bold),
        ];
        sections.push(
            Column::new()
                .gaps(LayoutSpacing::all(6))
                .list(tree)
                .builder(|node| CommentThreadView::new(node.clone(), 0))
                .boxed(),
        );
        match &thread.status {
            // Waiting for the reader to near the end; nothing to say yet.
            StreamStatus::Idle => {}
            StreamStatus::Loading => {
                sections.push(section_text("Loading comments…".into(), &theme, 14, FontWeight::Normal))
            }
            StreamStatus::Error(error) => sections.push(section_text(
                format!("Couldn't load more comments: {error}"),
                &theme,
                14,
                FontWeight::Normal,
            )),
            StreamStatus::Done => {}
        }

        Container::new()
            .padding(LayoutSpacing::all(24).top(8))
            .child(Column::new().gaps(LayoutSpacing::all(12)).children(sections))
    }
}

fn heading(loaded: usize) -> String {
    match loaded {
        0 => "Comments".to_string(),
        1 => "1 comment".to_string(),
        n => format!("{n} comments"),
    }
}

fn section_text(text: String, theme: &ForumTheme, size: u32, weight: FontWeight) -> AnyWidget {
    Text::new(text)
        .text_style(
            TextStyle::new()
                .font_size(size)
                .font_weight(weight)
                .color(theme.on_background_color().with_opacity(if size > 16 { 255 } else { 160 })),
        )
        .boxed()
}

/// How far from the end of the page, in logical pixels, the reader is when the
/// next chunk of comments is asked for: far enough that it has usually arrived
/// before they get there.
pub const LOAD_AHEAD: f32 = 1200.0;

/// Whether a page scrolled to `offset`, with `max_extent` to scroll in all, is
/// within `lead` of its end. A page that fits without scrolling (`max_extent`
/// of zero) is at its end.
pub fn near_end(offset: f32, max_extent: f32, lead: f32) -> bool {
    max_extent - offset <= lead
}

/// Opens a topic's thread in the store: the first batch comes straight from the
/// topic document and one more chunk is asked for at once, so the page is
/// never too short to scroll; the rest wait for the reader, see
/// [`load_more_if_near_end`].
async fn load_comments(
    store: ProviderHandle<AppStore>,
    topic_id: i64,
    initial: PostStream,
) -> Result<(), String> {
    if store.read().has_comment_thread(topic_id) {
        return Ok(());
    }

    let loaded: HashSet<i64> = initial.posts.iter().map(|post| post.id).collect();
    let missing: Vec<i64> = initial
        .stream
        .iter()
        .copied()
        .filter(|id| !loaded.contains(id))
        .collect();
    store.update(|s| s.start_comments(topic_id, comments_from(&initial.posts), missing));
    load_more_comments(&store, topic_id);
    Ok(())
}

/// Asks for the next chunk of comments if the reader is within [`LOAD_AHEAD`]
/// of the end of the page.
///
/// Cheap enough to call on every scroll frame: it returns before changing the
/// store unless a chunk is actually waiting to be asked for.
pub fn load_more_if_near_end(
    store: &ProviderHandle<AppStore>,
    topic_id: i64,
    offset: f32,
    max_extent: f32,
) {
    if near_end(offset, max_extent, LOAD_AHEAD) && store.read().wants_more_comments(topic_id) {
        load_more_comments(store, topic_id);
    }
}

/// Claims the next chunk of a topic's comments and fetches it in the
/// background. Does nothing while a chunk is already on its way, after the last,
/// or after a failure.
fn load_more_comments(store: &ProviderHandle<AppStore>, topic_id: i64) {
    let mut claimed = None;
    store.update(|s| claimed = s.claim_comment_chunk(topic_id, POSTS_PER_REQUEST));
    let Some(ids) = claimed else {
        return;
    };
    let Some(venus) = Venus::current() else {
        return;
    };
    let store = store.clone();
    venus.spawn(async move {
        let query = ids
            .iter()
            .map(|id| format!("post_ids[]={id}"))
            .collect::<Vec<_>>()
            .join("&");
        match Api::get::<PostsPage>(&format!("t/{topic_id}/posts.json?{query}")).await {
            Ok(page) => {
                let comments = comments_from(&page.post_stream.posts);
                store.update(|s| {
                    s.push_comments(topic_id, comments);
                    s.settle_comments(topic_id);
                });
            }
            Err(error) => store.update(|s| s.fail_comments(topic_id, error.to_string())),
        }
    });
}

/// The replies among `posts`: everything but the topic's first post, whispers
/// and small system actions ("closed this topic"), which carry no text.
fn comments_from(posts: &[Post]) -> Vec<Comment> {
    const REGULAR: i64 = 1;
    const MODERATOR_ACTION: i64 = 2;
    posts
        .iter()
        .filter(|post| post.post_number > 1)
        .filter(|post| matches!(post.post_type, REGULAR | MODERATOR_ACTION))
        .map(Comment::from_post)
        .collect()
}

/// A comment and, beneath it, the replies to it, drawn along a guide line.
#[derive(StatelessWidget)]
struct CommentThreadView {
    node: Rc<CommentNode>,
    depth: usize,
}

impl CommentThreadView {
    fn new(node: Rc<CommentNode>, depth: usize) -> Self {
        Self { node, depth }
    }
}

impl StatelessWidget for CommentThreadView {
    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme = *ForumTheme::of(ctx);
        let mut parts: Vec<AnyWidget> = vec![CommentCard::new(self.node.comment.clone()).boxed()];

        if !self.node.replies.is_empty() {
            let depth = self.depth + 1;
            // Each level of replies sits behind a guide line of its own, drawn
            // at the parent's edge; past the last level a chain stays flat.
            let step = if self.depth < MAX_INDENT_DEPTH { INDENT } else { 0 };
            let guide = BoxBorder {
                left: BorderSlice::new()
                    .style(BorderStyle::Solid)
                    .stroke(if step > 0 { 2 } else { 0 })
                    .color(theme.on_background_color().with_opacity(40)),
                ..Default::default()
            };
            parts.push(
                Container::new()
                    .padding(LayoutSpacing::new().left(step))
                    .box_decoration(BoxDecoration::new().border(guide))
                    .child(
                        Column::new()
                            .list(self.node.replies.clone())
                            .builder(move |reply| CommentThreadView::new(reply.clone(), depth)),
                    )
                    .boxed(),
            );
        }

        Column::new().children(parts)
    }
}

/// One comment: who wrote it and when on top, the body, and a row of actions
/// at the right.
#[derive(StatelessWidget)]
struct CommentCard {
    comment: Rc<Comment>,
}

impl CommentCard {
    fn new(comment: Rc<Comment>) -> Self {
        Self { comment }
    }
}

/// How many comment cards have been built, for tests that count the work an
/// append of comments costs.
#[cfg(test)]
pub(crate) static CARDS_BUILT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

impl StatelessWidget for CommentCard {
    fn build(&self, ctx: &BuildContext) -> impl Widget {
        #[cfg(test)]
        CARDS_BUILT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let theme = *ForumTheme::of(ctx);
        let comment = &self.comment;
        let navigator = Navigator::<AppRouter>::of(ctx);
        let muted = theme.on_background_color().with_opacity(160);

        let who = Row::new()
            .vertical_alignment(BoxAlignment::Center)
            .gaps(LayoutSpacing::horizontal(8.into()))
            .children([
                round_avatar(theme, &comment.avatar_url, &comment.username, AVATAR_DIAMETER),
                Text::new(comment.username.clone())
                    .text_style(
                        TextStyle::new()
                            .font_size(14)
                            .font_weight(FontWeight::Bold)
                            .color(theme.on_background_color()),
                    )
                    .boxed(),
            ]);
        let header = Row::new()
            .vertical_alignment(BoxAlignment::Center)
            .gaps(LayoutSpacing::horizontal(8.into()))
            .children([
                profile_link(navigator, &comment.username, who),
                Text::new(format!("· {}", relative_time(&comment.created_at)))
                    .text_style(TextStyle::new().font_size(14).color(muted))
                    .boxed(),
            ]);

        let body = forum_markdown(&comment.markdown, &theme, LayoutSpacing::new().top(2));

        let actions = Row::new()
            .vertical_alignment(BoxAlignment::Center)
            .gaps(LayoutSpacing::horizontal(20.into()))
            .children([
                Expanded::new().box_child(SizedBox::new()),
                action_icon(Icons::ELLIPSIS, muted),
                action_icon(Icons::REPLY, muted),
                Row::new()
                    .vertical_alignment(BoxAlignment::Center)
                    .gaps(LayoutSpacing::horizontal(6.into()))
                    .children([
                        action_icon(Icons::HEART, muted),
                        Text::new(format_count(comment.like_count))
                            .text_style(TextStyle::new().font_size(14).color(muted))
                            .boxed(),
                    ])
                    .boxed(),
            ]);

        Container::new()
            .padding(LayoutSpacing::vertical(10))
            .child(
                Column::new()
                    .gaps(LayoutSpacing::all(6))
                    .children([header.boxed(), body, actions.boxed()]),
            )
    }
}

fn action_icon(glyph: &'static StaticSvg, color: aimer::Color) -> AnyWidget {
    Svg::new(glyph)
        .style("path", SvgStyle::new().stroke(color))
        .width(ACTION_ICON_SIZE)
        .height(ACTION_ICON_SIZE)
        .boxed()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_end_is_near_once_less_than_the_lead_remains() {
        // 5000 px of scrolling in all, 1200 px of lead.
        assert!(!near_end(0.0, 5000.0, 1200.0));
        assert!(!near_end(3799.0, 5000.0, 1200.0));
        assert!(near_end(3800.0, 5000.0, 1200.0));
        assert!(near_end(5000.0, 5000.0, 1200.0));
    }

    #[test]
    fn overscroll_past_the_end_is_still_near_it() {
        assert!(near_end(5200.0, 5000.0, 1200.0));
    }

    #[test]
    fn a_page_that_fits_without_scrolling_is_near_its_end() {
        assert!(near_end(0.0, 0.0, 1200.0));
    }

    mod profile {
        use super::*;
        use crate::router::AppRouter;
        use crate::screen::post_detail::comments::CommentsSection;
        use aimer::quiver::winit::dpi::PhysicalSize;
        use aimer::router::Navigator;
        use aimer::style::AnimatedTheme;
        use aimer::{AimerApp, HeadlessOptions, Provider, Scrollable};
        use std::time::Instant;

        thread_local! {
            static SEEN: std::cell::RefCell<Option<ProviderHandle<AppStore>>> =
                const { std::cell::RefCell::new(None) };
            static SCROLL: aimer::ScrollController = aimer::ScrollController::new();
        }

        #[derive(Clone, StatelessWidget)]
        struct Host;

        impl StatelessWidget for Host {
            fn build(&self, ctx: &BuildContext) -> impl Widget {
                SEEN.with(|seen| *seen.borrow_mut() = Some(ProviderHandle::<AppStore>::of(ctx)));
                AnimatedTheme::new().data(ForumTheme::light()).child(
                    Scrollable::new().controller(SCROLL.with(|c| c.clone())).box_child(
                        aimer::Column::new().children([
                            aimer::SizedBox::new().height(100).boxed(),
                            CommentsSection::new(1, PostStream::default()).boxed(),
                            aimer::SizedBox::new().height(90).boxed(),
                        ]),
                    ),
                )
            }
        }

        /// Real topic bodies from the repository, converted the way the app does,
        /// so the comments of a test thread differ in structure and cost: a
        /// lightbox image, links, code, quotes.
        fn bodies() -> Vec<Rc<str>> {
            let real = [
                include_str!("../../../sample_html.html"),
                include_str!("../../../special_html/138788.html"),
                include_str!("../../../special_html/141777.html"),
                include_str!("../../../special_html/141914.html"),
                include_str!("../../../special_html/576751.html"),
            ];
            let mut bodies: Vec<Rc<str>> = real.iter().map(|html| crate::html_markdown::html_to_markdown(html)).collect();
            bodies.push(Rc::from("+1"));
            bodies.push(Rc::from("Thanks, this fixed it for me."));
            bodies.push(Rc::from(
                "I tried this on **macOS** and it works.\n\n```rust\nfn main() {\n    println!(\"hi\");\n}\n```\n\nSee [the docs](https://example.com) for more.",
            ));
            bodies.push(Rc::from("A longer one. ".repeat(60)));
            bodies
        }

        fn chunk(first: i64, count: i64) -> Vec<Comment> {
            thread_local! {
                static BODIES: Vec<Rc<str>> = bodies();
            }
            (first..first + count)
                .map(|n| Comment {
                    post_number: n,
                    // Every third comment answers the one before it, so the
                    // tree has real nesting.
                    reply_to: (n % 3 == 0).then_some(n - 1),
                    username: format!("user{n}"),
                    avatar_url: String::new(),
                    created_at: "2026-10-01T10:00:00.000Z".to_owned(),
                    like_count: n % 7,
                    markdown: BODIES.with(|bodies| bodies[(n as usize * 7) % bodies.len()].clone()),
                })
                .collect()
        }

        /// How many render operations the last frame planned: a page whose rows
        /// were not built paints far fewer than one whose rows were.
        fn painted_in_view(app: &aimer::HeadlessAimerApp<impl Widget + 'static>) -> usize {
            app.render_ops().len()
        }

        /// What converting a topic's HTML to markdown costs, per body.
        #[test]
        #[ignore = "profile harness"]
        fn profile_html_to_markdown() {
            let real = [
                ("sample_html", include_str!("../../../sample_html.html")),
                ("138788", include_str!("../../../special_html/138788.html")),
                ("141777", include_str!("../../../special_html/141777.html")),
                ("141914", include_str!("../../../special_html/141914.html")),
                ("576751", include_str!("../../../special_html/576751.html")),
            ];
            for (name, html) in real {
                let _ = crate::html_markdown::html_to_markdown(html);
                let started = Instant::now();
                for _ in 0..50 {
                    std::hint::black_box(crate::html_markdown::html_to_markdown(html));
                }
                println!(
                    "html_to_markdown {name:>12} ({:>5} bytes): {:>6.2} ms each",
                    html.len(),
                    started.elapsed().as_secs_f64() * 1e3 / 50.0
                );
            }
        }

        /// Scrolls through a long thread a few pixels per frame and times every
        /// frame, as a reader flicking down the page does.
        ///
        /// Run: `COMMENTS=1800 cargo nextest run --bin rust_forum -E
        /// 'test(profile_scrolling_a_long_thread)' --run-ignored only --no-capture`.
        #[test]
        #[ignore = "profile harness"]
        fn profile_scrolling_a_long_thread() {
            crate::testing::hold();
            let total: i64 = std::env::var("COMMENTS").ok().and_then(|n| n.parse().ok()).unwrap_or(1800);
            let mut store = AppStore::default();
            store.start_comments(1, chunk(2, total), vec![]);
            let started = Instant::now();
            let mut app = AimerApp::start_headless_with(
                Provider::new()
                    .create(move || store.clone())
                    .child(Navigator::new(AppRouter::Latest, |_| Host.boxed())),
                HeadlessOptions {
                    size: PhysicalSize::new(1100, 800),
                    scale_factor: 1.0,
                },
            );
            for _ in 0..5 {
                app.pump_frames(3);
            }
            app.render_frame();
            println!("first frames: {:.0} ms", started.elapsed().as_secs_f64() * 1e3);
            let scroll = SCROLL.with(|c| c.clone());
            println!("extent {:.0}", scroll.max_extent().y);

            // From the middle of the thread, at the pace of a slow drag and of
            // two flicks.
            let frames: usize = std::env::var("FRAMES").ok().and_then(|n| n.parse().ok()).unwrap_or(200);
            let steps: Vec<f32> = match std::env::var("STEP").ok().and_then(|n| n.parse().ok()) {
                Some(step) => vec![step],
                None => vec![12.0, 60.0, 240.0],
            };
            for step in steps {
                let start = scroll.max_extent().y / 4.0;
                scroll.jump_to(aimer::Vec2d { x: 0.0, y: start });
                app.pump_frames(3);
                let cards_before = CARDS_BUILT.load(std::sync::atomic::Ordering::SeqCst);
                let mut times = Vec::new();
                let mut offset = start;
                for _ in 0..frames {
                    offset += step;
                    let frame = Instant::now();
                    scroll.jump_to(aimer::Vec2d { x: 0.0, y: offset });
                    app.render_frame();
                    times.push(frame.elapsed().as_secs_f64() * 1e3);
                }
                let cards = CARDS_BUILT.load(std::sync::atomic::Ordering::SeqCst) - cards_before;
                times.sort_by(|a, b| a.partial_cmp(b).unwrap());
                println!(
                    "{step:>5.0} px/frame: p50 {:>6.2} ms | p95 {:>6.2} ms | max {:>6.2} ms | mean {:>6.2} ms | {:.2} cards built per frame",
                    times[frames / 2],
                    times[frames * 95 / 100],
                    times[frames - 1],
                    times.iter().sum::<f64>() / frames as f64,
                    cards as f64 / frames as f64
                );
            }
        }

        /// Scrolls to within a screen of the end, as a reader does, then lets a
        /// chunk land, and follows every frame until the page is quiet again.
        ///
        /// Run: `cargo nextest run --bin rust_forum -E 'test(profile_scrolled_append)'
        /// --run-ignored only --no-capture`.
        #[test]
        #[ignore = "profile harness"]
        fn profile_scrolled_append() {
            crate::testing::hold();
            let mut store = AppStore::default();
            store.start_comments(1, chunk(2, 40), vec![]);
            let mut app = AimerApp::start_headless_with(
                Provider::new()
                    .create(move || store.clone())
                    .child(Navigator::new(AppRouter::Latest, |_| Host.boxed())),
                HeadlessOptions {
                    size: PhysicalSize::new(1100, 800),
                    scale_factor: 1.0,
                },
            );
            for _ in 0..5 {
                app.pump_frames(3);
            }
            app.render_frame();
            let store = SEEN.with(|seen| seen.borrow().clone().unwrap());
            let scroll = SCROLL.with(|c| c.clone());

            let mut next = 42;
            for round in 0..4 {
                let extent = scroll.max_extent().y;
                scroll.jump_to(aimer::Vec2d { x: 0.0, y: extent - 900.0 });
                for _ in 0..4 {
                    app.pump_frames(3);
                }
                println!(
                    "round {round}: at offset {:>8.1} of {:>8.1}, render ops {}",
                    scroll.offset().y,
                    scroll.max_extent().y,
                    painted_in_view(&app)
                );
                let comments = chunk(next, 20);
                next += 20;
                store.update(|s| s.push_comments(1, comments));
                for frame in 0..8 {
                    if app.pump_frames(1) == 0 {
                        println!("   quiet after {frame} frames");
                        break;
                    }
                    println!(
                        "   frame {frame}: offset {:>8.1} of {:>8.1}, render ops {}",
                        scroll.offset().y,
                        scroll.max_extent().y,
                        painted_in_view(&app)
                    );
                }
            }
        }

        /// Appends 20 comments at a time and times what each append costs in
        /// the headless app: the store update, the rebuild and a rendered frame.
        ///
        /// Run: `cargo nextest run --bin rust_forum -E 'test(profile_appending)'
        /// --run-ignored only --no-capture`.
        #[test]
        #[ignore = "profile harness"]
        fn profile_appending_a_chunk() {
            crate::testing::hold();
            let mut store = AppStore::default();
            store.start_comments(1, chunk(2, 20), vec![]);
            let mut app = AimerApp::start_headless_with(
                Provider::new()
                    .create(move || store.clone())
                    .child(Navigator::new(AppRouter::Latest, |_| Host.boxed())),
                HeadlessOptions {
                    size: PhysicalSize::new(1100, 800),
                    scale_factor: 1.0,
                },
            );
            for _ in 0..5 {
                app.pump_frames(3);
            }
            app.render_frame();
            let store = SEEN.with(|seen| seen.borrow().clone().unwrap());

            let mut next = 22;
            let rounds = std::env::var("APPEND_ROUNDS").ok().and_then(|n| n.parse().ok()).unwrap_or(10);
            for round in 0..rounds {
                let comments = chunk(next, 20);
                next += 20;
                let cards_before = CARDS_BUILT.load(std::sync::atomic::Ordering::SeqCst);
                let started = Instant::now();
                store.update(|s| s.push_comments(1, comments));
                let pushed = started.elapsed();
                let frames = app.pump_frames(200);
                let pumped = started.elapsed();
                app.render_frame();
                let total = started.elapsed();
                let cards = CARDS_BUILT.load(std::sync::atomic::Ordering::SeqCst) - cards_before;
                println!(
                    "append {round:>2}: {:>3} loaded | {cards:>4} cards built | {frames:>3} frames | extent {:>8.1} | update {:>6.1} ms | rebuild {:>7.1} ms | frame {:>6.1} ms | total {:>7.1} ms",
                    next - 22 + 20,
                    SCROLL.with(|c| c.max_extent().y),
                    pushed.as_secs_f64() * 1e3,
                    (pumped - pushed).as_secs_f64() * 1e3,
                    (total - pumped).as_secs_f64() * 1e3,
                    total.as_secs_f64() * 1e3,
                );
            }
        }
    }
}
