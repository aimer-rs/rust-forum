mod skeleton;
mod tabs;

use crate::api::Api;
use crate::app_shell::{MOBILE_TOP, is_mobile};
use crate::components::windows::Placement;
use crate::components::side_bar::parse_hex_color;
use crate::components::topic_card::{count_label, divider_decor, format_count, relative_time};
use crate::html_markdown::{forum_markdown, html_to_markdown};
use crate::models::comments::avatar_url;
use crate::models::user::{SummaryTopic, UserProfile, UserRoot, UserSummaryRoot, format_duration};
use crate::router::AppRouter;
use crate::screen::user_profile::skeleton::SkeletonProfile;
use crate::screen::user_profile::tabs::ProfileTabs;
use crate::store::AppStore;
use crate::theme::ForumTheme;
use aimer::animation::{AnimatedSwitcher, curves};
use aimer::input::button::Button;
use aimer::router::NavigatorController;
use aimer::share::ShareRef;
use aimer::style::{BoxDecoration, BoxFit, LayoutSpacing, TextOverflow, TextStyle, Theme};
use aimer::{
    Align, Alignment, AnyWidget, AsyncBuilder, AsyncSnapshot, BoxAlignment, BuildContext, Column,
    Container, Dimension, Expanded, FontWeight, Icons, NetworkImage, ProviderHandle, Row,
    Scrollable, SizedBox, Stack, StatelessWidget, StaticSvg, Svg, SvgStyle, Text, Widget,
    ZeroSizedBox, case,
};

/// How long the profile takes to cross-fade in over its skeleton once it arrives.
const CONTENT_FADE: std::time::Duration = std::time::Duration::from_millis(250);
/// The room above the profile in a window of its own: the title bar is drawn over
/// the top of the window, so the profile starts below its buttons.
const WINDOW_TOP: u32 = 44;
const AVATAR_SIZE: f32 = 96.0;
const ICON_SIZE: u32 = 16;

/// A forum member's profile: who they are, how active they are, their best
/// topics, where they post and the badges they hold.
#[derive(Clone, StatelessWidget)]
pub struct UserProfileScreen {
    username: ShareRef<str>,
    placement: Placement,
}

impl UserProfileScreen {
    pub fn new(username: ShareRef<str>) -> Self {
        Self { username, placement: Placement::Page }
    }

    /// Where the profile is shown, which says how much room it leaves at its top.
    pub fn placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }
}

/// The room a screen leaves above its first line: the bar over a page, with the
/// status bar of a narrow window, or the room the title bar of a window of its own
/// takes, with the buttons that close it.
pub fn top(ctx: &BuildContext, placement: Placement) -> u32 {
    match placement {
        Placement::Page => case!(is_mobile(ctx), MOBILE_TOP) + 70,
        Placement::Window => WINDOW_TOP,
    }
}

impl StatelessWidget for UserProfileScreen {
    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme = *ForumTheme::of(ctx);
        let store = ProviderHandle::<AppStore>::of(ctx);
        let username = self.username.clone();
        let placement = self.placement;
        let key = username.to_string();
        // A profile seen before is already in the store. `AsyncBuilder` always
        // starts in `Waiting`, so without this it would flash its skeleton for a
        // frame before the (instant) fetch delivers it.
        let cached = store.read().cached_profile(&username);
        let key_for_switcher = key.clone();

        let data = AsyncBuilder::new()
            .request_key(key.clone())
            .key(format!("profile-{key}"))
            .future(move || {
                let store = store.clone();
                let username = username.clone();
                async move { fetch_cached_profile(store, username).await }
            })
            .child(move |snapshot| {
                // The skeleton and the profile cross-fade when the profile
                // arrives. A cached profile never shows the skeleton, so both of
                // its states share a key and nothing fades.
                let (phase, content) = match snapshot {
                    AsyncSnapshot::Waiting => match &cached {
                        Some(profile) => (
                            "content",
                            ProfileContent {
                                profile: profile.clone(),
                            }
                            .boxed(),
                        ),
                        None => ("skeleton", SkeletonProfile::new().boxed()),
                    },
                    AsyncSnapshot::Error(error) => ("error", message(theme, error)),
                    AsyncSnapshot::Data(profile) => (
                        "content",
                        ProfileContent {
                            profile: profile.clone(),
                        }
                        .boxed(),
                    ),
                };
                AnimatedSwitcher::new(CONTENT_FADE, curves::EASE_IN_OUT, content)
                    .child_key(phase)
                    .key(format!("profile-switcher-{key_for_switcher}"))
                    .boxed()
            });

        Container::new().color(theme.background_color()).child(
            Stack::new().children([Scrollable::new()
                .key(self.placement.key("scrollable-profile") + &format!("-{key}"))
                .box_child(
                    Column::new().children([
                        SizedBox::new().height(top(ctx, placement)).boxed(),
                        data.boxed(),
                        SizedBox::new().height(90).boxed(),
                    ]),
                )]),
        )
    }
}

async fn fetch_cached_profile(
    store: ProviderHandle<AppStore>,
    username: ShareRef<str>,
) -> Result<UserProfile, String> {
    if let Some(profile) = store.read().cached_profile(&username) {
        return Ok(profile);
    }

    let detail = Api::get::<UserRoot>(&format!("u/{username}.json"))
        .await
        .map_err(|e| format!("Error: {e}"))?
        .user;
    let summary = Api::get::<UserSummaryRoot>(&format!("u/{username}/summary.json"))
        .await
        .map_err(|e| format!("Error: {e}"))?;

    let profile = UserProfile { detail, summary };
    store.update(|s| s.cache_profile(&username, profile.clone()));
    Ok(profile)
}

/// A line of plain text standing in for the profile while it loads or fails.
fn message(theme: ForumTheme, text: &str) -> AnyWidget {
    Container::new().padding(LayoutSpacing::all(24)).box_child(
        Text::new(text.to_string()).text_style(
            TextStyle::new()
                .text_overflow(TextOverflow::Wrap)
                .font_size(16)
                .color(theme.on_background_color().with_opacity(160)),
        ),
    )
}

/// The loaded profile. Reads the theme in its own `build` so it follows theme
/// changes, independent of the `AsyncBuilder`'s snapshot.
#[derive(Clone, StatelessWidget)]
struct ProfileContent {
    profile: UserProfile,
}

impl StatelessWidget for ProfileContent {
    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme = *ForumTheme::of(ctx);
        let profile = &self.profile;

        let mut sections = vec![header(theme, profile)];
        if let Some(bio) = profile
            .detail
            .bio_cooked
            .as_deref()
            .filter(|bio| !bio.is_empty())
        {
            sections.push(forum_markdown(
                &html_to_markdown(bio),
                &theme,
                LayoutSpacing::new(),
            ));
        }
        sections.push(ProfileTabs::new(profile.clone(), profile.detail.username.clone()).boxed());

        Container::new()
            .padding(LayoutSpacing::horizontal(24.into()))
            .child(
                Column::new()
                    .gaps(LayoutSpacing::all(20))
                    .children(sections),
            )
    }
}

/// The Summary tab: the activity figures and the user's top topics.
fn summary_tab(theme: ForumTheme, profile: &UserProfile) -> AnyWidget {
    let mut sections = vec![stats(theme, profile)];

    if !profile.summary.topics.is_empty() {
        sections.push(section_title(theme, "Top topics"));
        sections.push(
            Column::new()
                .children(
                    profile
                        .summary
                        .topics
                        .iter()
                        .map(|topic| {
                            TopicRow {
                                slug: ShareRef::from_str_slice(&topic.slug),
                                id: topic.id,
                                title: topic.title.clone(),
                                likes: topic.like_count,
                                replies: replies_of(topic),
                                created: relative_time(&topic.created_at),
                            }
                            .boxed()
                        })
                        .collect::<Vec<_>>(),
                )
                .boxed(),
        );
    }

    Column::new()
        .gaps(LayoutSpacing::all(20))
        .children(sections)
        .boxed()
}

/// The Others tab: where the user posts, the badges they hold and the links
/// they share.
fn others_tab(theme: ForumTheme, profile: &UserProfile) -> AnyWidget {
    let summary = &profile.summary.user_summary;
    let mut sections: Vec<AnyWidget> = Vec::new();

    if !summary.top_categories.is_empty() {
        sections.push(section_title(theme, "Top categories"));
        sections.push(
            Column::new()
                .gaps(LayoutSpacing::all(10))
                .children(
                    summary
                        .top_categories
                        .iter()
                        .map(|category| {
                            category_row(
                                theme,
                                &category.color,
                                &category.name,
                                &format!(
                                    "{} topics · {} posts",
                                    format_count(category.topic_count),
                                    format_count(category.post_count),
                                ),
                            )
                        })
                        .collect::<Vec<_>>(),
                )
                .boxed(),
        );
    }

    let badges: Vec<AnyWidget> = summary
        .badges
        .iter()
        .filter_map(|granted| {
            let name = profile.badge_name(granted.badge_id)?;
            let label = if granted.count > 1 {
                format!("{name} ×{}", granted.count)
            } else {
                name.to_string()
            };
            Some(icon_label(theme, Icons::AWARD, &label, 16))
        })
        .collect();
    if !badges.is_empty() {
        sections.push(section_title(theme, "Badges"));
        sections.push(
            Column::new()
                .gaps(LayoutSpacing::all(10))
                .children(badges)
                .boxed(),
        );
    }

    if !summary.links.is_empty() {
        sections.push(section_title(theme, "Popular links"));
        sections.push(
            Column::new()
                .gaps(LayoutSpacing::all(10))
                .children(
                    summary
                        .links
                        .iter()
                        .map(|link| {
                            LinkRow {
                                url: link.url.clone(),
                                title: link.title.clone().filter(|title| !title.is_empty()),
                                clicks: link.clicks,
                            }
                            .boxed()
                        })
                        .collect::<Vec<_>>(),
                )
                .boxed(),
        );
    }

    if sections.is_empty() {
        return message(theme, "Nothing else to show.");
    }
    Column::new()
        .gaps(LayoutSpacing::all(20))
        .children(sections)
        .boxed()
}

/// One of the links the user shares most; pressing it opens it in the browser.
#[derive(Clone, StatelessWidget)]
struct LinkRow {
    url: String,
    title: Option<String>,
    clicks: i64,
}

impl StatelessWidget for LinkRow {
    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme = *ForumTheme::of(ctx);
        let url = self.url.clone();
        let muted = theme.on_background_color().with_opacity(160);

        Button::new()
            .no_decor()
            .on_press(move || {
                if let Err(error) = webbrowser::open(&url) {
                    aimer::console::error!("could not open {url}: {error}");
                }
            })
            .child(
                Column::new().gaps(LayoutSpacing::all(2)).children([
                    Text::new(self.title.clone().unwrap_or_else(|| self.url.clone()))
                        .text_style(
                            TextStyle::new()
                                .text_overflow(TextOverflow::Wrap)
                                .font_weight(FontWeight::Bold)
                                .font_size(16)
                                .color(theme.on_background_color()),
                        )
                        .boxed(),
                    Text::new(format!(
                        "{} clicks · {}",
                        format_count(self.clicks),
                        self.url
                    ))
                    .text_style(TextStyle::new().font_size(13).color(muted))
                    .boxed(),
                ]),
            )
    }
}

/// The user's card: the avatar with the name and three counts beside it, and
/// under them the user's title, role and dates. The handle is in the bar above
/// the page, next to the back button.
fn header(theme: ForumTheme, profile: &UserProfile) -> AnyWidget {
    let detail = &profile.detail;
    let summary = &profile.summary.user_summary;
    let muted = theme.on_background_color().with_opacity(160);
    let display_name = detail
        .name
        .as_deref()
        .filter(|name| !name.is_empty())
        .unwrap_or(&detail.username);

    let card = Row::new()
        .vertical_alignment(BoxAlignment::Center)
        .gaps(LayoutSpacing::horizontal(24.into()))
        .children([
            avatar(theme, &detail.avatar_template, display_name),
            Expanded::new().box_child(
                Column::new().gaps(LayoutSpacing::all(10)).children([
                    Text::new(display_name.to_string())
                        .text_style(
                            TextStyle::new()
                                .text_overflow(TextOverflow::Wrap)
                                .font_weight(FontWeight::Bold)
                                .font_size(18)
                                .color(theme.on_background_color()),
                        )
                        .boxed(),
                    Row::new()
                        .gaps(LayoutSpacing::horizontal(8.into()))
                        .children([
                            card_count(theme, &format_count(summary.post_count), "posts"),
                            card_count(theme, &format_count(summary.topic_count), "topics"),
                            card_count(theme, &format_count(summary.likes_received), "likes"),
                        ])
                        .boxed(),
                ]),
            ),
        ])
        .boxed();

    let mut details = Vec::new();
    if let Some(title) = detail.title.as_deref().filter(|title| !title.is_empty()) {
        details.push(
            Text::new(title.to_string())
                .text_style(TextStyle::new().font_size(14).color(muted))
                .boxed(),
        );
    }
    details.push(icon_label(theme, Icons::AWARD, &profile.role(), 14));
    details.push(icon_label(
        theme,
        Icons::CALENDAR,
        &format!("Joined {}", detail.created_at.get(..10).unwrap_or("—")),
        14,
    ));
    if let Some(seen) = detail.last_seen_at.as_deref() {
        details.push(icon_label(
            theme,
            Icons::EYE,
            &format!("Seen {}", relative_time(seen)),
            14,
        ));
    }

    Column::new()
        .gaps(LayoutSpacing::all(14))
        .children([
            card,
            Column::new()
                .gaps(LayoutSpacing::all(4))
                .children(details)
                .boxed(),
        ])
        .boxed()
}

/// One of the three counts in the card: the figure, and what it counts under it.
pub(crate) fn card_count(theme: ForumTheme, value: &str, label: &str) -> AnyWidget {
    Expanded::new().box_child(
        Column::new().children([
            Text::new(value.to_string())
                .text_style(
                    TextStyle::new()
                        .font_weight(FontWeight::Bold)
                        .font_size(20)
                        .color(theme.on_background_color()),
                )
                .boxed(),
            Text::new(label.to_string())
                .text_style(
                    TextStyle::new()
                        .font_size(14)
                        .color(theme.on_background_color().with_opacity(180)),
                )
                .boxed(),
        ]),
    )
}

fn avatar(theme: ForumTheme, template: &str, name: &str) -> AnyWidget {
    let round = BoxDecoration::new().border_radius(AVATAR_SIZE / 2.0);
    if template.is_empty() {
        let initial = name
            .chars()
            .next()
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_default();
        return Container::new()
            .width(Dimension::Px(AVATAR_SIZE))
            .height(Dimension::Px(AVATAR_SIZE))
            .box_decoration(round.background_color(theme.primary_color()))
            .child(
                Align::new().alignment(Alignment::MidCenter).child(
                    Text::new(initial).text_style(
                        TextStyle::new()
                            .font_size(36)
                            .color(theme.on_primary_color()),
                    ),
                ),
            )
            .boxed();
    }
    Container::new()
        .width(Dimension::Px(AVATAR_SIZE))
        .height(Dimension::Px(AVATAR_SIZE))
        .box_decoration(round)
        .child(
            // `Contain`, not `Cover`: a square avatar fills the box either way,
            // and `Cover` would drop the container's rounded clip.
            NetworkImage::new(avatar_url(template, 192))
                .width(AVATAR_SIZE)
                .height(AVATAR_SIZE)
                .fit(BoxFit::Contain),
        )
        .boxed()
}

/// The activity figures: nine tiles in a 3×3 grid.
fn stats(theme: ForumTheme, profile: &UserProfile) -> AnyWidget {
    let summary = &profile.summary.user_summary;
    let tiles = [
        ("Days visited", format_count(summary.days_visited)),
        ("Read time", format_duration(summary.time_read)),
        ("Topics viewed", format_count(summary.topics_entered)),
        ("Posts read", format_count(summary.posts_read_count)),
        ("Likes given", format_count(summary.likes_given)),
        ("Likes received", format_count(summary.likes_received)),
        ("Topics", format_count(summary.topic_count)),
        ("Posts", format_count(summary.post_count)),
        ("Solutions", format_count(summary.solved_count)),
    ];

    Column::new()
        .gaps(LayoutSpacing::all(4))
        .children(
            tiles
                .chunks(3)
                .map(|pair| {
                    Row::new()
                        .gaps(LayoutSpacing::horizontal(10.into()))
                        .children(
                            pair.iter()
                                .map(|(label, value)| {
                                    Expanded::new().box_child(stat_tile(theme, label, value))
                                })
                                .collect::<Vec<_>>(),
                        )
                        .boxed()
                })
                .collect::<Vec<_>>(),
        )
        .boxed()
}

fn stat_tile(theme: ForumTheme, label: &str, value: &str) -> AnyWidget {
    Container::new()
        .padding(LayoutSpacing::all(10))
        .box_decoration(
            BoxDecoration::new()
                .background_color(theme.on_background_color().with_alpha(0.06))
                .border_radius(4),
        )
        .child(
            Column::new().gaps(LayoutSpacing::all(2)).children([
                Text::new(value.to_string())
                    .text_style(
                        TextStyle::new()
                            .font_weight(FontWeight::Bold)
                            .font_size(20)
                            .color(theme.on_background_color()),
                    )
                    .boxed(),
                Text::new(label.to_string())
                    .text_style(
                        TextStyle::new()
                            .text_overflow(TextOverflow::Wrap)
                            .font_size(12)
                            .color(theme.on_background_color().with_opacity(160)),
                    )
                    .boxed(),
            ]),
        )
        .boxed()
}

fn section_title(theme: ForumTheme, title: &str) -> AnyWidget {
    Container::new()
        .padding(LayoutSpacing::new().top(8))
        .box_child(
            Text::new(title.to_string()).text_style(
                TextStyle::new()
                    .font_weight(FontWeight::Bold)
                    .font_size(20)
                    .color(theme.on_background_color()),
            ),
        )
}

/// A category the user posts in: its color, name and how much they posted.
fn category_row(theme: ForumTheme, color: &str, name: &str, counts: &str) -> AnyWidget {
    Row::new()
        .vertical_alignment(BoxAlignment::Center)
        .gaps(LayoutSpacing::horizontal(10.into()))
        .children([
            Container::new()
                .width(Dimension::Px(12.0))
                .height(Dimension::Px(12.0))
                .box_decoration(
                    BoxDecoration::new()
                        .background_color(parse_hex_color(color))
                        .border_radius(6),
                )
                .child(ZeroSizedBox)
                .boxed(),
            Text::new(name.to_string())
                .text_style(
                    TextStyle::new()
                        .font_weight(FontWeight::Bold)
                        .font_size(16)
                        .color(theme.on_background_color()),
                )
                .boxed(),
            Expanded::new().box_child(SizedBox::new()),
            Text::new(counts.to_string())
                .text_style(
                    TextStyle::new()
                        .font_size(14)
                        .color(theme.on_background_color().with_opacity(160)),
                )
                .boxed(),
        ])
        .boxed()
}

/// A small icon with a line of text beside it.
fn icon_label(theme: ForumTheme, glyph: &'static StaticSvg, text: &str, size: u32) -> AnyWidget {
    let color = theme.on_background_color().with_opacity(180);
    Row::new()
        .vertical_alignment(BoxAlignment::Center)
        .gaps(LayoutSpacing::horizontal(8.into()))
        .children([
            Svg::new(glyph)
                .style("path", SvgStyle::new().stroke(color))
                .width(ICON_SIZE)
                .height(ICON_SIZE)
                .boxed(),
            Text::new(text.to_string())
                .text_style(TextStyle::new().font_size(size).color(color))
                .boxed(),
        ])
        .boxed()
}

/// How many replies a topic has: its posts after the opening one, never fewer
/// than none.
fn replies_of(topic: &SummaryTopic) -> i64 {
    topic.posts_count.saturating_sub(1).max(0)
}

/// One of the user's top topics; pressing it opens the post.
///
/// Its numbers read as they do on a topic card of a feed: an icon with a count
/// for the likes and for the replies, at the left. The card shows its views at
/// the right, which a profile's summary does not have, so the row shows when the
/// topic was started there instead.
#[derive(Clone, StatelessWidget)]
struct TopicRow {
    slug: ShareRef<str>,
    id: i64,
    title: String,
    likes: i64,
    replies: i64,
    created: String,
}

impl StatelessWidget for TopicRow {
    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme = *ForumTheme::of(ctx);
        let navigator = NavigatorController::<AppRouter>::of(ctx);
        let slug = self.slug.clone();
        let id = self.id;

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
                Container::new().padding(LayoutSpacing::vertical(14)).child(
                    Column::new().gaps(LayoutSpacing::all(4)).children([
                        Text::new(self.title.clone())
                            .text_style(
                                TextStyle::new()
                                    .text_overflow(TextOverflow::Wrap)
                                    .font_weight(FontWeight::Bold)
                                    .font_size(17)
                                    .color(theme.on_background_color()),
                            )
                            .boxed(),
                        Row::new()
                            .vertical_alignment(BoxAlignment::Center)
                            .gaps(LayoutSpacing::horizontal(20.into()))
                            .children([
                                count_label(theme, Icons::HEART, self.likes),
                                count_label(theme, Icons::MESSAGE_CIRCLE, self.replies),
                                Expanded::new().box_child(SizedBox::new()),
                                Text::new(self.created.clone())
                                    .text_style(
                                        TextStyle::new()
                                            .font_size(13)
                                            .color(theme.on_background_color().with_opacity(160)),
                                    )
                                    .boxed(),
                            ])
                            .boxed(),
                    ]),
                ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::user::SummaryTopic;

    fn topic(posts_count: i64) -> SummaryTopic {
        SummaryTopic {
            posts_count,
            ..Default::default()
        }
    }

    #[test]
    fn replies_are_the_posts_after_the_opening_one() {
        assert_eq!(replies_of(&topic(5)), 4);
        assert_eq!(replies_of(&topic(2)), 1);
        assert_eq!(replies_of(&topic(1)), 0);
    }

    /// The app on the profile of `alice`, already loaded, with one top topic or
    /// with none.
    fn profile_app(with_top_topic: bool) -> aimer::HeadlessAimerApp<impl aimer::Widget + 'static> {
        profile_app_in(with_top_topic, 450)
    }

    /// [`profile_app`] in a window `width` wide: a phone, or, wide enough for the
    /// sidebar, a desktop.
    fn profile_app_in(
        with_top_topic: bool,
        width: u32,
    ) -> aimer::HeadlessAimerApp<impl aimer::Widget + 'static> {
        use aimer::quiver::winit::dpi::PhysicalSize;
        use aimer::router::Navigator;
        use aimer::{AimerApp, HeadlessOptions, Provider};

        let mut store = AppStore::default();
        store.cache_profile(
            "alice",
            UserProfile {
                summary: UserSummaryRoot {
                    topics: with_top_topic
                        .then(|| SummaryTopic {
                            id: 7,
                            title: "A post worth reading".to_owned(),
                            slug: "a-post".to_owned(),
                            posts_count: 12,
                            like_count: 3400,
                            created_at: "2024-01-01T00:00:00Z".to_owned(),
                            ..Default::default()
                        })
                        .into_iter()
                        .collect(),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        crate::testing::hold();
        let mut app = AimerApp::start_headless_with(
            aimer::Provider::new()
                .create(move || store.clone())
                .child(Navigator::new(
                    AppRouter::UserProfile {
                        username: ShareRef::from_static("alice"),
                    },
                    |r| r.boxed(),
                )),
            HeadlessOptions {
                size: PhysicalSize::new(width, 900),
                scale_factor: 1.0,
            },
        );
        for _ in 0..10 {
            std::thread::sleep(std::time::Duration::from_millis(20));
            app.pump_frames(3);
        }
        app
    }

    #[test]
    fn a_profile_that_is_still_loading_shows_the_skeleton() {
        use aimer::quiver::winit::dpi::PhysicalSize;
        use aimer::router::Navigator;
        use aimer::{AimerApp, HeadlessOptions, Provider};

        // Nothing is cached for this user, so the screen has to ask for the
        // profile. The first frames are before any answer could arrive.
        let store = AppStore::default();
        crate::testing::hold();
        let mut app = AimerApp::start_headless_with(
            Provider::new()
                .create(move || store.clone())
                .child(Navigator::new(
                    AppRouter::UserProfile {
                        username: ShareRef::from_static("nobody"),
                    },
                    |r| r.boxed(),
                )),
            HeadlessOptions {
                size: PhysicalSize::new(450, 900),
                scale_factor: 1.0,
            },
        );
        app.pump_frames(3);
        let census = app
            .paint_source_census()
            .expect("a mounted page has a root");

        // The shell around the profile paints about 120 nodes, and a line of
        // "Loading…" adds a few; the skeleton, with its avatar, tiles and rows, adds
        // about 170 (293 in all when this was measured).
        assert!(
            census.local_v2 >= 200,
            "only {} nodes paint",
            census.local_v2
        );
    }

    #[test]
    fn a_trackpad_swipe_back_from_a_post_is_not_left_behind_by_the_page_sliding_away() {
        use aimer::quiver::winit::dpi::PhysicalPosition;
        use aimer::quiver::winit::event::{
            DeviceId, ElementState, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent,
        };
        use std::time::Duration;

        // A desktop window, so the cursor is a good way into the page, and scroll
        // events go where the cursor is, not where the page is.
        let mut app = profile_app_in(true, 1500);
        let cursor = |app: &mut aimer::HeadlessAimerApp<_>, x: f64, y: f64| {
            app.send_window_event(WindowEvent::CursorMoved {
                device_id: DeviceId::dummy(),
                position: PhysicalPosition::new(x, y),
            });
        };
        let frames = |app: &mut aimer::HeadlessAimerApp<_>, rounds: u32| {
            let mut total = 0;
            for _ in 0..rounds {
                std::thread::sleep(Duration::from_millis(10));
                total += app.pump_frames(1);
            }
            total
        };

        // Let the profile settle, then open the post by pressing its row, which is
        // somewhere in this band, below the tabs. Hovering the page animates too, so what says a post
        // opened is the page growing by the post's worth of painted nodes.
        frames(&mut app, 30);
        let painted = |app: &aimer::HeadlessAimerApp<_>| {
            app.paint_source_census().expect("a mounted page").local_v2
        };
        // The action bar is the shell's now and starts out faded away, so the page
        // alone brings about fifty nodes; hovering brings none like that many.
        const POST_NODES: usize = 40;
        let mut opened = false;
        for y in (440..880).step_by(8) {
            cursor(&mut app, 700.0, y as f64);
            let before = painted(&app);
            for state in [ElementState::Pressed, ElementState::Released] {
                app.send_window_event(WindowEvent::MouseInput {
                    device_id: DeviceId::dummy(),
                    state,
                    button: MouseButton::Left,
                });
            }
            let animated = frames(&mut app, 8);
            if animated >= 8 && painted(&app) >= before + POST_NODES {
                opened = true;
                break;
            }
        }
        assert!(opened, "a press on the row opens the post");
        frames(&mut app, 60);

        // The fingers go a long way right, with the cursor where it was.
        let wheel = |phase, dx: f64| WindowEvent::MouseWheel {
            device_id: DeviceId::dummy(),
            delta: MouseScrollDelta::PixelDelta(PhysicalPosition::new(dx, 0.0)),
            phase,
        };
        for _ in 0..2 {
            app.send_window_event(wheel(TouchPhase::Started, 2.0));
            frames(&mut app, 1);
        }
        for _ in 0..16 {
            app.send_window_event(wheel(TouchPhase::Moved, 600.0 / 16.0));
            std::thread::sleep(Duration::from_millis(8));
            app.pump_frames(1);
        }
        app.send_window_event(wheel(TouchPhase::Ended, 0.0));

        // It finishes, and the app goes quiet. A swipe left hanging half way holds
        // the app in frames for ever.
        let mut quiet = false;
        for _ in 0..300 {
            std::thread::sleep(Duration::from_millis(10));
            if app.pump_frames(1) == 0 {
                quiet = true;
                break;
            }
        }
        assert!(
            quiet,
            "the swipe never finished: the page slid out from under the cursor"
        );
    }

    #[test]
    fn a_profile_with_a_top_topic_paints_it_through_the_retained_path() {
        let without = profile_app(false)
            .paint_source_census()
            .expect("a mounted page has a root");
        let app = profile_app(true);
        let census = app
            .paint_source_census()
            .expect("a mounted page has a root");

        // The row (its title, the two icons and their counts, the date) is on
        // the page: it adds nodes that paint, and none that is dropped.
        assert!(
            census.local_v2 > without.local_v2,
            "{} vs {}",
            census.local_v2,
            without.local_v2
        );
        assert!(
            census.drawn_unmapped.is_empty(),
            "unmapped {:?}",
            census.drawn_unmapped
        );
        assert!(
            census.invalid_bounds.is_empty(),
            "invalid bounds {:?}",
            census.invalid_bounds
        );
        assert!(census.sync_error.is_none(), "{:?}", census.sync_error);
        assert!(
            census.declined_paint.len() <= without.declined_paint.len(),
            "the row declined to paint: {:?}",
            census.declined_paint
        );
    }

    #[test]
    fn a_topic_with_no_posts_has_no_replies_rather_than_a_negative_count() {
        assert_eq!(replies_of(&topic(0)), 0);
        assert_eq!(replies_of(&topic(-3)), 0);
    }
}
