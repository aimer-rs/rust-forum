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
    SvgStyle, Text, Widget, ZeroSizedBox,
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

/// Streams a topic's comments into the store: the first batch comes straight
/// from the topic document, the remainder from `posts.json` in chunks.
async fn load_comments(
    store: ProviderHandle<AppStore>,
    topic_id: i64,
    initial: PostStream,
) -> Result<(), String> {
    if store.read().comment_thread(topic_id).status == StreamStatus::Done {
        return Ok(());
    }

    let loaded: HashSet<i64> = initial.posts.iter().map(|post| post.id).collect();
    store.update(|s| s.push_comments(topic_id, comments_from(&initial.posts)));

    let missing: Vec<i64> = initial
        .stream
        .iter()
        .copied()
        .filter(|id| !loaded.contains(id))
        .collect();
    for chunk in missing.chunks(POSTS_PER_REQUEST) {
        let query = chunk
            .iter()
            .map(|id| format!("post_ids[]={id}"))
            .collect::<Vec<_>>()
            .join("&");
        match Api::get::<PostsPage>(&format!("t/{topic_id}/posts.json?{query}")).await {
            Ok(page) => {
                let comments = comments_from(&page.post_stream.posts);
                store.update(|s| s.push_comments(topic_id, comments));
            }
            Err(error) => {
                store.update(|s| s.fail_comments(topic_id, error.to_string()));
                return Ok(());
            }
        }
    }

    store.update(|s| s.finish_comments(topic_id));
    Ok(())
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

impl StatelessWidget for CommentCard {
    fn build(&self, ctx: &BuildContext) -> impl Widget {
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
