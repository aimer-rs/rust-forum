use crate::app_shell::is_mobile;
use std::time::Duration;

use crate::components::page_title::PageTitle;
use crate::components::skeleton_topic_card::SkeletonTopicCard;
use crate::components::topic_card::TopicCard;
use crate::models::Topic;
use crate::store::{AppStore, StreamStatus, TopicStream};
use crate::theme::ForumTheme;
use crate::topic_stream::stream_topics;
use aimer::animation::{AnimatedSwitcher, curves};
use aimer::share::{ShareRef, Shared};
use aimer::style::{LayoutSpacing, TextStyle, Theme};
use aimer::{
    AnyWidget, AsyncBuilder, BuildContext, Column, Container, Expanded, FontWeight,
    ProviderContext, ProviderHandle, Scrollable, SizedBox, StatelessWidget, Text, Widget,
    ZeroSizedBox, case
};

const TRANSITION_DURATION: Duration = Duration::from_millis(250);

/// A scrollable list of topics streamed from a Discourse API path such as
/// `latest.json` (home) or `c/{slug}/{id}.json` (a category). Topics render
/// one-by-one as they arrive instead of waiting for the whole document.
#[derive(StatelessWidget, Clone)]
pub struct TopicListScreen {
    path: ShareRef<str>,
    title: ShareRef<str>,
}

impl TopicListScreen {
    pub fn new(path: ShareRef<str>, title: ShareRef<str>) -> Self {
        Self { path, title }
    }
}

impl StatelessWidget for TopicListScreen {
    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme = *ForumTheme::of(ctx);
        let store = ProviderHandle::<AppStore>::of(ctx);
        let category_store = store.clone();
        let path = self.path.clone();
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
            Column::new().children([
                SizedBox::new().height(case!(is_mobile(ctx), 60)).boxed(),
                PageTitle::new(self.title.clone()).boxed(),
                SizedBox::new().height(16).boxed(),
                stream_content(stream, theme),
            ]),
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
                            .key(format!("scrollable-{}", self.path))
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
    // A path that finished streaming before is already in the store; don't
    // re-fetch it.
    if store.read().topic_stream(&path).status == StreamStatus::Done {
        return Ok(());
    }

    // Signing in mid-stream empties the lists; what this request still brings was
    // fetched as the previous member (or as a guest) and does not belong in them.
    let epoch = store.read().session_epoch;

    let store_for_stream = store.clone();
    let result = stream_topics(&path, |topic| {
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
            Ok(()) => s.finish_stream(path),
            Err(error) => s.fail_stream(path, error),
        }
    });

    Ok(())
}

fn stream_content(stream: TopicStream, theme: ForumTheme) -> AnyWidget {
    match &stream.status {
        StreamStatus::Loading if stream.topics.is_empty() => skeleton_list(),
        StreamStatus::Error(error) if stream.topics.is_empty() => Text::new(error.clone())
            .text_style(TextStyle::new().color(theme.on_background_color()))
            .boxed(),
        _ => topics_list(stream.topics),
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

fn topics_list(topics: Vec<Shared<Topic>>) -> AnyWidget {
    Column::new()
        .list(topics)
        .builder(|topic| {
            Container::new().child(TopicCard::new(topic.as_share_ref()))
        })
        .boxed()
}
