//! The profile's tabs: Summary, Topics, Replies, Likes and Others, switched by
//! pressing a title or by swiping the page left or right.

use super::{message, others_tab, summary_tab};
use crate::api::Api;
use crate::app_shell::is_mobile;
use crate::components::topic_card::{divider_decor, relative_time};
use crate::models::user::{ActionFilter, UserAction, UserActionsRoot, UserProfile};
use crate::router::AppRouter;
use crate::store::AppStore;
use crate::theme::ForumTheme;
use aimer::animation::{AnimatedSwitcher, curves};
use aimer::input::button::Button;
use aimer::input::gesture::SwipeDirection;
use aimer::input::gesture::gesture_detector::GestureDetector;
use aimer::router::NavigatorController;
use aimer::share::ShareRef;
use aimer::style::{BoxDecoration, LayoutSpacing, TextOverflow, TextStyle, Theme};
use aimer::{
    Align, Alignment, AnyWidget, AsyncBuilder, AsyncSnapshot, BoxAlignment, BuildContext, Color,
    Column, Container, Dimension, Expanded, FontWeight, Icons, ProviderHandle, Row, StateUpdater,
    State, StatefulWidget, StatelessWidget, StaticSvg, Svg, SvgStyle, Text, Widget, ZeroSizedBox,
};
use std::time::Duration;

/// How long one tab's page takes to cross-fade into the next.
const TAB_FADE: Duration = Duration::from_millis(180);
const ICON_SIZE: u32 = 16;
/// The thickness of the line under the tab that is shown.
const UNDERLINE: f32 = 2.0;
/// The most characters of a post shown under its title.
const EXCERPT_LIMIT: usize = 140;

/// The tabs, in order: the title, and the icon beside it on a wide window.
const TABS: [(&str, &StaticSvg); 5] = [
    ("Summary", Icons::USER),
    ("Topics", Icons::LIST),
    ("Replies", Icons::REPLY),
    ("Likes", Icons::HEART),
    ("Others", Icons::LAYOUT_GRID),
];

/// The tab bar over the page of the tab that is shown.
#[derive(Clone, StatefulWidget)]
pub struct ProfileTabs {
    profile: UserProfile,
    /// Whose activity the Topics, Replies and Likes tabs list.
    username: String,
}

impl ProfileTabs {
    pub fn new(profile: UserProfile, username: String) -> Self {
        Self { profile, username }
    }
}

pub struct ProfileTabsState {
    parent: ProfileTabs,
    selected: usize,
    updater: StateUpdater<Self>,
}

impl StatefulWidget for ProfileTabs {
    type State = ProfileTabsState;

    fn create_state(self) -> Self::State {
        ProfileTabsState {
            parent: self,
            selected: 0,
            updater: StateUpdater::new(),
        }
    }
}

/// The tab a swipe leads to from `selected`: a swipe to the left brings the tab
/// on the right, and the other way round. At either end there is no further tab,
/// and the swipe stays where it is.
fn tab_after_swipe(selected: usize, direction: SwipeDirection) -> usize {
    match direction {
        SwipeDirection::Left => (selected + 1).min(TABS.len() - 1),
        SwipeDirection::Right => selected.saturating_sub(1),
        SwipeDirection::Up | SwipeDirection::Down => selected,
    }
}

impl State<ProfileTabs> for ProfileTabsState {
    fn init_state(&mut self, updater: StateUpdater<Self>) {
        self.updater = updater;
    }

    fn adopt_config_from(&mut self, new: Self) {
        // The same user's page rebuilt keeps its tab; another user starts again.
        if self.parent.username != new.parent.username {
            self.selected = 0;
        }
        self.parent = new.parent;
    }

    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme = *ForumTheme::of(ctx);
        let selected = self.selected;
        let updater = self.updater;
        let wide = !is_mobile(ctx);

        let bar = tab_bar(theme, selected, wide, move |index| {
            updater.set_state(move |state| state.selected = index);
        });

        let body = tab_body(theme, &self.parent, selected);
        let page = AnimatedSwitcher::new(TAB_FADE, curves::EASE_IN_OUT, body)
            .child_key(format!("tab-{selected}"))
            .key(format!("profile-tab-page-{}", self.parent.username));

        let swipe = GestureDetector::new()
            .on_swipe(move |direction: SwipeDirection| {
                updater.set_state(move |state| {
                    state.selected = tab_after_swipe(state.selected, direction);
                });
            })
            .child(Container::new().padding(LayoutSpacing::new().top(20)).child(page));

        Column::new().children([bar, swipe.boxed()])
    }
}

/// The page of the tab at `index`.
fn tab_body(theme: ForumTheme, tabs: &ProfileTabs, index: usize) -> AnyWidget {
    match index {
        0 => summary_tab(theme, &tabs.profile),
        1 => ActivityList::new(tabs.username.clone(), ActionFilter::Topics).boxed(),
        2 => ActivityList::new(tabs.username.clone(), ActionFilter::Replies).boxed(),
        3 => ActivityList::new(tabs.username.clone(), ActionFilter::Likes).boxed(),
        _ => others_tab(theme, &tabs.profile),
    }
}

/// The row of titles, with a line under the one that is shown.
fn tab_bar(
    theme: ForumTheme,
    selected: usize,
    wide: bool,
    on_select: impl Fn(usize) + Clone + 'static,
) -> AnyWidget {
    Container::new()
        .box_decoration(divider_decor(theme))
        .child(
            Row::new()
                .children(
                    TABS.iter()
                        .enumerate()
                        .map(|(index, (label, icon))| {
                            let on_select = on_select.clone();
                            Expanded::new().box_child(
                                Button::new()
                                    .no_decor()
                                    .on_press(move || on_select(index))
                                    .child(tab_title(theme, label, icon, index == selected, wide)),
                            )
                        })
                        .collect::<Vec<_>>(),
                )
                .boxed(),
        )
        .boxed()
}

/// One title: its icon (on a wide window) and name, centred, with the line under
/// it coloured when the tab is the one shown.
fn tab_title(
    theme: ForumTheme,
    label: &str,
    icon: &'static StaticSvg,
    selected: bool,
    wide: bool,
) -> AnyWidget {
    let color = if selected {
        theme.primary_color()
    } else {
        theme.on_background_color().with_opacity(180)
    };

    let mut content: Vec<AnyWidget> = Vec::new();
    if wide {
        content.push(
            Svg::new(icon)
                .style("path", SvgStyle::new().stroke(color))
                .width(ICON_SIZE)
                .height(ICON_SIZE)
                .boxed(),
        );
    }
    content.push(
        Text::new(label.to_string())
            .text_style(
                TextStyle::new()
                    .font_weight(if selected {
                        FontWeight::Bold
                    } else {
                        FontWeight::Normal
                    })
                    .font_size(14)
                    .color(color),
            )
            .boxed(),
    );

    Column::new()
        .children([
            Container::new()
                .height(Dimension::Px(44.0))
                .child(
                    Align::new().alignment(Alignment::MidCenter).child(
                        Row::new()
                            .vertical_alignment(BoxAlignment::Center)
                            .gaps(LayoutSpacing::horizontal(6.into()))
                            .children(content),
                    ),
                )
                .boxed(),
            Container::new()
                .height(Dimension::Px(UNDERLINE))
                .box_decoration(BoxDecoration::new().background_color(if selected {
                    theme.primary_color()
                } else {
                    Color::Transparent
                }))
                .child(ZeroSizedBox)
                .boxed(),
        ])
        .boxed()
}

/// A user's topics, replies or likes, loaded when its tab is first opened.
#[derive(Clone, StatelessWidget)]
struct ActivityList {
    username: String,
    filter: ActionFilter,
}

impl ActivityList {
    fn new(username: String, filter: ActionFilter) -> Self {
        Self { username, filter }
    }
}

impl StatelessWidget for ActivityList {
    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme = *ForumTheme::of(ctx);
        let store = ProviderHandle::<AppStore>::of(ctx);
        let username = self.username.clone();
        let filter = self.filter;
        // As on the page itself, a list seen before is shown from the first frame
        // rather than after a frame of waiting.
        let cached = store.read().cached_actions(&username, filter);
        let key = format!("activity-{username}-{}", filter.code());

        AsyncBuilder::new()
            .request_key(key.clone())
            .key(key)
            .future(move || {
                let store = store.clone();
                let username = username.clone();
                async move { fetch_cached_actions(store, username, filter).await }
            })
            .child(move |snapshot| match snapshot {
                AsyncSnapshot::Waiting => match &cached {
                    Some(actions) => action_list(theme, actions, filter),
                    None => message(theme, "Loading…"),
                },
                AsyncSnapshot::Error(error) => message(theme, error),
                AsyncSnapshot::Data(actions) => action_list(theme, actions, filter),
            })
    }
}

async fn fetch_cached_actions(
    store: ProviderHandle<AppStore>,
    username: String,
    filter: ActionFilter,
) -> Result<Vec<UserAction>, String> {
    if let Some(actions) = store.read().cached_actions(&username, filter) {
        return Ok(actions);
    }

    let root = Api::get::<UserActionsRoot>(&format!(
        "user_actions.json?offset=0&username={username}&filter={}",
        filter.code()
    ))
    .await
    .map_err(|e| format!("Error: {e}"))?;

    store.update(|s| s.cache_actions(&username, filter, root.user_actions.clone()));
    Ok(root.user_actions)
}

fn action_list(theme: ForumTheme, actions: &[UserAction], filter: ActionFilter) -> AnyWidget {
    if actions.is_empty() {
        return message(
            theme,
            match filter {
                ActionFilter::Topics => "No topics yet.",
                ActionFilter::Replies => "No replies yet.",
                ActionFilter::Likes => "No likes yet.",
            },
        );
    }
    Column::new()
        .children(
            actions
                .iter()
                .map(|action| {
                    ActionRow {
                        slug: ShareRef::from_str_slice(&action.slug),
                        id: action.topic_id,
                        title: action.title.clone(),
                        excerpt: action.excerpt.as_deref().map(clean_excerpt).unwrap_or_default(),
                        // A liked post is someone else's: say whose.
                        by: (filter == ActionFilter::Likes).then(|| action.username.clone()),
                        created: relative_time(&action.created_at),
                    }
                    .boxed()
                })
                .collect::<Vec<_>>(),
        )
        .boxed()
}

/// An excerpt as plain text of at most [`EXCERPT_LIMIT`] characters: Discourse
/// sends HTML text with entities.
fn clean_excerpt(raw: &str) -> String {
    let decoded = raw
        .replace("&hellip;", "…")
        .replace("&nbsp;", " ")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&");
    let text = decoded.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.chars().count() <= EXCERPT_LIMIT {
        return text;
    }
    let cut: String = text.chars().take(EXCERPT_LIMIT).collect();
    format!("{}…", cut.trim_end_matches(['…', ' ', '.']))
}

/// One entry of the activity: the topic's title, a few words of the post and when
/// it was; pressing it opens the topic.
#[derive(Clone, StatelessWidget)]
struct ActionRow {
    slug: ShareRef<str>,
    id: i64,
    title: String,
    excerpt: String,
    /// Whose post it is, when that is not the user whose page this is.
    by: Option<String>,
    created: String,
}

impl StatelessWidget for ActionRow {
    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme = *ForumTheme::of(ctx);
        let navigator = NavigatorController::<AppRouter>::of(ctx);
        let slug = self.slug.clone();
        let id = self.id;
        let muted = theme.on_background_color().with_opacity(160);

        let mut lines = vec![
            Text::new(self.title.clone())
                .text_style(
                    TextStyle::new()
                        .text_overflow(TextOverflow::Wrap)
                        .font_weight(FontWeight::Bold)
                        .font_size(17)
                        .color(theme.on_background_color()),
                )
                .boxed(),
        ];
        if !self.excerpt.is_empty() {
            lines.push(
                Text::new(self.excerpt.clone())
                    .text_style(
                        TextStyle::new()
                            .text_overflow(TextOverflow::Wrap)
                            .font_size(15)
                            .color(muted),
                    )
                    .boxed(),
            );
        }
        lines.push(
            Text::new(match &self.by {
                Some(author) => format!("{author} · {}", self.created),
                None => self.created.clone(),
            })
            .text_style(TextStyle::new().font_size(13).color(muted))
            .boxed(),
        );

        Button::new()
            .decoration(divider_decor(theme))
            .hover_decoration(
                divider_decor(theme).background_color(theme.primary_color().with_opacity(10)),
            )
            .on_press(move || {
                navigator.push(AppRouter::PostDetail {
                    slug: slug.clone(),
                    id,
                });
            })
            .child(
                Container::new()
                    .padding(LayoutSpacing::vertical(14))
                    .child(Column::new().gaps(LayoutSpacing::all(4)).children(lines)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_swipe_to_the_left_brings_the_tab_on_the_right() {
        assert_eq!(tab_after_swipe(0, SwipeDirection::Left), 1);
        assert_eq!(tab_after_swipe(3, SwipeDirection::Left), 4);
    }

    #[test]
    fn a_swipe_to_the_right_brings_the_tab_on_the_left() {
        assert_eq!(tab_after_swipe(4, SwipeDirection::Right), 3);
        assert_eq!(tab_after_swipe(1, SwipeDirection::Right), 0);
    }

    #[test]
    fn the_first_and_last_tabs_have_nothing_beyond_them() {
        assert_eq!(tab_after_swipe(0, SwipeDirection::Right), 0);
        assert_eq!(tab_after_swipe(TABS.len() - 1, SwipeDirection::Left), TABS.len() - 1);
    }

    #[test]
    fn a_vertical_swipe_keeps_the_tab() {
        assert_eq!(tab_after_swipe(2, SwipeDirection::Up), 2);
        assert_eq!(tab_after_swipe(2, SwipeDirection::Down), 2);
    }

    #[test]
    fn an_excerpt_is_plain_text_and_cut_short() {
        assert_eq!(clean_excerpt("a &amp; b\n  &quot;c&quot;"), "a & b \"c\"");
        let long = clean_excerpt(&"word ".repeat(100));
        assert!(long.chars().count() <= EXCERPT_LIMIT + 1);
        assert!(long.ends_with('…'));
    }
}
