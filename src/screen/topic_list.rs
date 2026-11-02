use crate::app_shell::{beside_buttons, is_mobile};
use crate::layout::{ShellLayout, shell_layout};
use std::time::Duration;

use crate::components::page_title::PageTitle;
use crate::components::skeleton_topic_card::SkeletonTopicCard;
use crate::components::topic_card::TopicCard;
use crate::models::Topic;
use crate::screen::post_detail::comments::{LOAD_AHEAD, near_end};
use crate::store::{AppStore, StreamStatus, TopicStream};
use crate::theme::ForumTheme;
use crate::topic_stream::stream_topics;
use aimer::animation::{AnimatedSwitcher, curves};
use aimer::share::{ShareRef, Shared};
use aimer::style::{LayoutSpacing, TextStyle, Theme};
use aimer::{
    AnyWidget, AsyncBuilder, BuildContext, Column, Container, Expanded, FontWeight,
    ProviderContext, ProviderHandle, ScrollController, Scrollable, SizedBox, State,
    StateUpdater, StatefulWidget, Text, Vec2d, Venus, Widget, ZeroSizedBox, case
};
use std::rc::Rc;

const TRANSITION_DURATION: Duration = Duration::from_millis(250);

/// The room above the title on a phone, clear of the bar along its top.
const PHONE_TOP: u32 = 60;
/// The room above the title where the header buttons stand in a column on the
/// trailing edge, as on the outer display of an iPhone Duo: no bar lies along the
/// top, so less is needed.
const RAIL_TOP: u32 = 8;

/// How far down the title starts.
fn top_space(ctx: &BuildContext) -> u32 {
    match shell_layout(ctx) {
        ShellLayout::Rail => RAIL_TOP,
        _ => case!(is_mobile(ctx), PHONE_TOP),
    }
}

/// A scrollable list of topics streamed from a Discourse API path such as
/// `latest.json` (home) or `c/{slug}/{id}.json` (a category). Topics render
/// one-by-one as they arrive instead of waiting for the whole document. The next
/// page of the list is fetched as the reader nears the end of what is loaded.
#[derive(StatefulWidget, Clone)]
pub struct TopicListScreen {
    path: ShareRef<str>,
    title: ShareRef<str>,
}

impl TopicListScreen {
    pub fn new(path: ShareRef<str>, title: ShareRef<str>) -> Self {
        Self { path, title }
    }
}

pub struct TopicListScreenState {
    parent: TopicListScreen,
    scroll_controller: ScrollController,
    /// A second handle on `scroll_controller` for its own scroll callback to
    /// reach through a `Weak`: the controller keeps that callback alive, so a
    /// callback holding the controller itself would keep both alive forever.
    scroll_probe: Rc<ScrollController>,
}

impl StatefulWidget for TopicListScreen {
    type State = TopicListScreenState;

    fn create_state(self) -> Self::State {
        let scroll_controller = ScrollController::new();
        TopicListScreenState {
            parent: self,
            scroll_probe: Rc::new(scroll_controller.clone()),
            scroll_controller,
        }
    }
}

impl State<TopicListScreen> for TopicListScreenState {
    fn init_state(&mut self, _updater: StateUpdater<Self>) {}

    fn adopt_config_from(&mut self, new: Self) {
        self.parent = new.parent;
    }

    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme = *ForumTheme::of(ctx);
        let store = ProviderHandle::<AppStore>::of(ctx);
        let category_store = store.clone();
        let path = self.parent.path.clone();

        // The next page is fetched as the reader nears the end of the list.
        // Registered on every build so it always carries the current path.
        let paging_store = store.clone();
        let paging_path = path.clone();
        let paging_scroll = Rc::downgrade(&self.scroll_probe);
        self.scroll_controller.on_scroll(move |offset: Vec2d| {
            if let Some(scroll) = paging_scroll.upgrade() {
                load_more_if_near_end(&paging_store, &paging_path, offset.y, scroll.max_extent().y);
            }
        });
        let stream = ctx.select::<AppStore, TopicStream>({
            let path = path.clone();
            move |s| s.topic_stream(&path)
        });

        // The forum lists differently for a member than for a guest, so the list
        // is asked for again when who is signed in changes.
        let member = ctx.select::<AppStore, Option<String>>(|s| s.current_user.clone());

        let phase = if matches!(&stream.status, StreamStatus::Loading) && stream.topics.is_empty() {
            "loading"
        } else {
            "loaded"
        };
        let content = AnimatedSwitcher::new(
            TRANSITION_DURATION,
            curves::EASE_IN_OUT,
            beside_buttons(
                ctx,
                Column::new().children([
                    SizedBox::new().height(top_space(ctx)).boxed(),
                    PageTitle::new(self.parent.title.clone()).boxed(),
                    SizedBox::new().height(16).boxed(),
                    stream_content(stream, theme),
                ]),
            ),
        )
        .child_key(phase)
        .key(format!("topic-stream-switcher-{path}"))
        .boxed();

        let driver = AsyncBuilder::new()
            .request_key(format!("{path}|{}", member.as_deref().unwrap_or("")))
            .key(format!("topics-stream-{path}"))
            .future(move || {
                let store = store.clone();
                let path = path.clone();
                async move { stream_topics_task(store, path).await }
            })
            .child(|_| ZeroSizedBox.boxed());

        // The cards name their category from the store. The sidebar fills it,
        // but it is not built on mobile, so load the categories here too.
        let categories_driver = AsyncBuilder::new()
            .key("topic-list-categories")
            .future(move || {
                let store = category_store.clone();
                async move { load_categories(store).await }
            })
            .child(|_| ZeroSizedBox.boxed());

        // Text::new(self.title.clone())
        //     .text_style(TextStyle::new().font_size(32).font_weight(FontWeight::Bold))
        //     .boxed(),

        Container::new().color(theme.background_color()).child(
            Column::new().children([
                driver.boxed(),
                categories_driver.boxed(),
                Expanded::new()
                    .child(
                        Scrollable::new()
                            .controller(self.scroll_controller.clone())
                            .key(format!("scrollable-{}", self.parent.path))
                            .child(content),
                    )
                    .boxed(),
            ]),
        )
    }
}

/// Fills the store's categories unless they are already there.
async fn load_categories(store: ProviderHandle<AppStore>) -> Result<(), String> {
    if !store.read().categories.is_empty() {
        return Ok(());
    }
    let root = AppStore::fetch_categories().await?;
    store.update(|s| s.cache_categories(&root.category_list.categories));
    Ok(())
}

async fn stream_topics_task(
    store: ProviderHandle<AppStore>,
    path: ShareRef<str>,
) -> Result<(), String> {
    // A list that has topics already (or has streamed to its end) is in the
    // store; don't re-fetch it. Its later pages come with the reader's scrolling.
    let known = store.read().topic_stream(&path);
    if !known.topics.is_empty() || known.status == StreamStatus::Done {
        return Ok(());
    }

    stream_page(store, path, 0).await;
    Ok(())
}

/// Streams one page of a list into the store, topic by topic, then settles the
/// list: ready for a further page, at its end, or failed.
async fn stream_page(store: ProviderHandle<AppStore>, path: ShareRef<str>, page: u32) {
    // Signing in mid-stream empties the lists; what this request still brings was
    // fetched as the previous member (or as a guest) and does not belong in them.
    let epoch = store.read().session_epoch;

    let url = if page == 0 {
        path.to_string()
    } else {
        format!("{path}?page={page}")
    };
    let store_for_stream = store.clone();
    let result = stream_topics(&url, |topic| {
        store_for_stream.update(|s| {
            if s.session_epoch == epoch {
                s.push_stream_topic(path.clone(), topic);
            }
        });
    })
    .await;

    store.update(|s| {
        if s.session_epoch != epoch {
            return;
        }
        match result {
            Ok(more) => s.finish_stream(path, page, more),
            Err(error) => s.fail_stream(path, error),
        }
    });
}

/// Asks for the next page of a list if the reader is within [`LOAD_AHEAD`] of the
/// end of it.
///
/// Cheap enough to call on every scroll frame: it returns before changing the
/// store unless a page is actually waiting to be asked for.
fn load_more_if_near_end(
    store: &ProviderHandle<AppStore>,
    path: &ShareRef<str>,
    offset: f32,
    max_extent: f32,
) {
    if !near_end(offset, max_extent, LOAD_AHEAD) || !store.read().wants_more_topics(path) {
        return;
    }
    let Some(venus) = Venus::current() else {
        return;
    };
    let mut claimed = None;
    store.update(|s| claimed = s.claim_topic_page(path));
    if let Some(page) = claimed {
        venus.spawn(stream_page(store.clone(), path.clone(), page));
    }
}

fn stream_content(stream: TopicStream, theme: ForumTheme) -> AnyWidget {
    match &stream.status {
        StreamStatus::Loading if stream.topics.is_empty() => skeleton_list(),
        StreamStatus::Error(error) if stream.topics.is_empty() => Text::new(error.clone())
            .text_style(TextStyle::new().color(theme.on_background_color()))
            .boxed(),
        _ => topics_list(stream, theme),
    }
}

fn skeleton_list() -> AnyWidget {
    Column::new()
        .list((0..6).collect::<Vec<usize>>())
        .builder(|_| {
            Container::new().child(SkeletonTopicCard::new())
        })
        .boxed()
}

fn topics_list(stream: TopicStream, theme: ForumTheme) -> AnyWidget {
    let footer = match &stream.status {
        StreamStatus::Loading => Some("Loading more…".to_string()),
        StreamStatus::Error(error) => Some(format!("Couldn't load more topics: {error}")),
        StreamStatus::Idle | StreamStatus::Done => None,
    };
    let mut parts: Vec<AnyWidget> = vec![Column::new()
        .list(stream.topics)
        .builder(|topic| {
            Container::new().child(TopicCard::new(topic.as_share_ref()))
        })
        .boxed()];
    if let Some(footer) = footer {
        parts.push(
            Container::new()
                .padding(LayoutSpacing::all(16))
                .child(
                    Text::new(footer).text_style(
                        TextStyle::new()
                            .font_size(14)
                            .color(theme.on_background_color().with_opacity(160)),
                    ),
                )
                .boxed(),
        );
    }
    Column::new().children(parts).boxed()
}
