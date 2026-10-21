use crate::components::profile_link::profile_link;
use crate::components::side_bar::parse_hex_color;
use crate::models::categories::Category;
use crate::models::{Author, Topic};
use crate::router::AppRouter;
use crate::store::AppStore;
use crate::theme::ForumTheme;
use aimer::animation::SharedElement;
use aimer::input::button::Button;
use aimer::router::NavigatorController;
use aimer::share::{ShareRef, Shared};
use aimer::style::{
    BorderSlice, BorderStyle, BoxBorder, BoxDecoration, BoxFit, LayoutSpacing, TextAlign,
    TextOverflow, TextStyle, Theme,
};
use aimer::{
    Align, Alignment, AnyWidget, BoxAlignment, BuildContext, Column, Container, Dimension,
    Expanded, FontWeight, Icons, Key, Margin, NetworkImage, ProviderContext, Row, SizedBox,
    StatelessWidget, StaticSvg, Svg, SvgStyle, Text, Widget,
};
use std::rc::Rc;

const AVATAR_SIZE: f32 = 24.0;
const ROW_HEIGHT: f32 = 24.0;
const ICON_SIZE: u32 = 16;
/// The longest excerpt shown, in characters, before it is cut with an ellipsis.
const EXCERPT_LIMIT: usize = 180;

/// A topic as a feed card: who started it and when on top, the title, a short
/// excerpt, and a row with its category and the likes, replies and views.
#[derive(Clone, StatelessWidget)]
pub struct TopicCard {
    topic: ShareRef<Topic>,
}

impl TopicCard {
    pub fn new(topic: ShareRef<Topic>) -> Self {
        Self { topic }
    }
}

impl StatelessWidget for TopicCard {
    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let navigator = NavigatorController::<AppRouter>::of(ctx);
        let theme = *ForumTheme::of(ctx);
        let topic = self.topic.clone();
        let slug = topic.project(|item| item.slug.as_str());
        let title = self.topic.clone().project(|item| item.title.as_str());
        let id = self.topic.id;

        let category_id = self.topic.category_id;
        let category = ctx
            .select::<AppStore, Option<Rc<Category>>>(move |store| store.category(category_id))
            .unwrap_or_else(|| {
                Rc::new(Category {
                    id: category_id,
                    name: format!("Category {category_id}"),
                    ..Default::default()
                })
            });

        let replies = self.topic.posts_count.saturating_sub(1);
        let since = relative_time(&self.topic.created_at);
        let author = self.topic.author.clone();
        let fallback_name = self.topic.last_poster_username.clone();
        let excerpt = self
            .topic
            .excerpt
            .as_deref()
            .map(clean_excerpt)
            .unwrap_or_default();

        let likes = self.topic.like_count;
        let views = self.topic.views;

        // The card is a shared element: opening the post grows the page out of
        // it, and going back shrinks the page into it. The closure builds the
        // card afresh each time, since the transition draws its own copy.
        SharedElement::new().tag(topic_tag(id)).child_with(move || {
            let mut children = vec![
                Text::new(title.clone())
                    .text_style(
                        TextStyle::new()
                            .text_overflow(TextOverflow::Wrap)
                            .font_weight(FontWeight::Bold)
                            .font_size(20)
                            .color(theme.on_surface_color()),
                    )
                    .boxed(),
            ];

            children.push(category_row(theme, &category));

            if !excerpt.is_empty() {
                children.push(
                    Text::new(excerpt.clone())
                        .text_style(
                            TextStyle::new()
                                .text_overflow(TextOverflow::Wrap)
                                .font_size(16)
                                .color(theme.on_surface_color().with_opacity(160)),
                        )
                        .boxed(),
                );
            }
            children.push(stats_row(theme, likes, replies, views));

            let header = header(
                theme,
                navigator.clone(),
                author.as_ref(),
                &fallback_name,
                since.clone(),
            );
            let navigator = navigator.clone();
            let slug = slug.clone();
            // The header sits outside the button: a button hides its child
            // from pointer events, and the author must be tappable.
            Container::new()
                .width(Dimension::Auto)
                .box_decoration(divider_decor(theme).background_color(theme.background_color()))
                .child(
                    Column::new().children([
                        Container::new()
                            .padding(LayoutSpacing::new().top(20).left(20).right(20))
                            .box_child(header),
                        Button::new()
                            .decoration(BoxDecoration::new())
                            .hover_decoration(BoxDecoration::new())
                            .on_press(move || {
                                navigator.push(AppRouter::PostDetail {
                                    slug: slug.clone(),
                                    id,
                                });
                            })
                            .child(
                                Container::new()
                                    .padding(
                                        LayoutSpacing::new().top(6).left(20).right(20).bottom(20),
                                    )
                                    .child(
                                        Column::new()
                                            .gaps(LayoutSpacing::all(6))
                                            .children(children),
                                    ),
                            )
                            .boxed(),
                    ]),
                )
        })
    }
}

/// The tag tying a topic's card to the post page that grows out of it.
pub fn topic_tag(id: i64) -> Key {
    Key::from(format!("topic-card-{id}"))
}

/// The top line: the author's avatar and name, when the topic was posted, and
/// an overflow mark at the far end.
fn header(
    theme: ForumTheme,
    navigator: NavigatorController<AppRouter>,
    author: Option<&Author>,
    fallback_name: &str,
    since: String,
) -> AnyWidget {
    let username = author.map_or(fallback_name, |author| author.username.as_str());
    let muted = theme.on_surface_color().with_opacity(160);

    let who = Row::new()
        .vertical_alignment(BoxAlignment::Center)
        .gaps(LayoutSpacing::horizontal(8.into()))
        .children([
            avatar(theme, author, username),
            Text::new(username.to_string())
                .text_style(
                    TextStyle::new()
                        .font_weight(FontWeight::Bold)
                        .font_size(14)
                        .color(theme.on_surface_color()),
                )
                .boxed(),
        ]);

    Row::new()
        .vertical_alignment(BoxAlignment::Center)
        .gaps(LayoutSpacing::horizontal(8.into()))
        .children([
            profile_link(navigator, username, who),
            Text::new(format!("· {since}"))
                .text_style(TextStyle::new().font_size(14).color(muted))
                .boxed(),
            Expanded::new().box_child(SizedBox::new()),
            icon(Icons::ELLIPSIS, muted),
        ])
        .boxed()
}

/// A round avatar; with no image to draw, a badge with the name's initial.
fn avatar(theme: ForumTheme, author: Option<&Author>, username: &str) -> AnyWidget {
    let round = BoxDecoration::new().border_radius(AVATAR_SIZE / 2.0);
    match author.filter(|author| !author.avatar_url.is_empty()) {
        Some(author) => Container::new()
            .width(Dimension::Px(AVATAR_SIZE))
            .height(Dimension::Px(AVATAR_SIZE))
            .box_decoration(round)
            .child(
                // Avatars are square, so `Contain` fills the box just as `Cover`
                // would. `Cover` makes the image push its own square clip, which
                // replaces the container's rounded one and leaves the corners.
                NetworkImage::new(author.avatar_url.clone())
                    .width(AVATAR_SIZE)
                    .height(AVATAR_SIZE)
                    .fit(BoxFit::Contain),
            )
            .boxed(),
        None => {
            let initial = username
                .chars()
                .next()
                .map(|c| c.to_uppercase().to_string())
                .unwrap_or_default();
            Container::new()
                .width(Dimension::Px(AVATAR_SIZE))
                .height(Dimension::Px(AVATAR_SIZE))
                .box_decoration(round.background_color(theme.primary_color()))
                .child(
                    Align::new().alignment(Alignment::MidCenter).child(
                        Text::new(initial).text_style(
                            TextStyle::new()
                                .font_size(12)
                                .color(theme.on_primary_color()),
                        ),
                    ),
                )
                .boxed()
        }
    }
}

/// The category line: just the category's pill, at the left.
fn category_row(theme: ForumTheme, category: &Category) -> AnyWidget {
    Row::new()
        .vertical_alignment(BoxAlignment::Center)
        .children([
            category_pill(theme, category),
            Expanded::new().box_child(SizedBox::new()),
        ])
        .boxed()
}

/// The bottom line: likes and replies on the left, views on the right.
fn stats_row(theme: ForumTheme, likes: i64, replies: i64, views: i64) -> AnyWidget {
    Row::new()
        .vertical_alignment(BoxAlignment::Center)
        .gaps(LayoutSpacing::horizontal(20.into()))
        .children([
            count_label(theme, Icons::HEART, likes),
            count_label(theme, Icons::MESSAGE_CIRCLE, replies),
            Expanded::new().box_child(SizedBox::new()),
            count_label(theme, Icons::EYE, views),
        ])
        .boxed()
}

/// The category as a pill: a Lucide icon in the category's color and its name,
/// on a chip that stands out from the card (light on dark, dark on light).
fn category_pill(theme: ForumTheme, category: &Category) -> AnyWidget {
    let accent = if category.color.is_empty() {
        theme.primary_color()
    } else {
        parse_hex_color(&category.color)
    };
    Container::new()
        .height(Dimension::Px(ROW_HEIGHT))
        .padding(LayoutSpacing::horizontal(12.into()))
        .box_decoration(
            BoxDecoration::new()
                .background_color(theme.on_surface_color())
                .border_radius(ROW_HEIGHT / 2.0),
        )
        .child(
            Align::new().alignment(Alignment::MidCenter).child(
                Row::new()
                    .vertical_alignment(BoxAlignment::Center)
                    .gaps(LayoutSpacing::horizontal(6.into()))
                    .children([
                        icon(category_icon(&category.slug), accent),
                        Margin::new()
                            .margin(LayoutSpacing::new().bottom(4))
                            .child(
                                Text::new(category.name.clone())
                                    .text_align(TextAlign::MidLeft)
                                    .text_style(
                                        TextStyle::new()
                                            .font_weight(FontWeight::Bold)
                                            .font_size(14)
                                            .color(theme.background_color()),
                                    ),
                            )
                            .boxed(),
                    ]),
            ),
        )
        .boxed()
}

/// The Lucide icon standing for a category, picked by its slug.
fn category_icon(slug: &str) -> &'static StaticSvg {
    match slug {
        "help" => Icons::CIRCLE_QUESTION_MARK,
        "code-review" => Icons::GIT_PULL_REQUEST,
        "community" => Icons::USERS,
        "embedded" => Icons::CPU,
        "ide" => Icons::SQUARE_PEN,
        "tutorial" => Icons::GRADUATION_CAP,
        "meta" => Icons::INFO,
        "announcements" => Icons::MEGAPHONE,
        "project" | "projects" => Icons::HAMMER,
        "uncategorized" => Icons::FOLDER,
        _ => Icons::TAG,
    }
}

/// The card has no box: a thin line under it separates it from the next one.
pub(crate) fn divider_decor(theme: ForumTheme) -> BoxDecoration {
    BoxDecoration::new().border(
        BoxBorder::new().bottom(
            BorderSlice::new()
                .style(BorderStyle::Solid)
                .stroke(1)
                .color(theme.on_surface_color().with_alpha(0.15)),
        ),
    )
}

/// An icon with a count beside it.
fn count_row(theme: ForumTheme, glyph: &'static StaticSvg, count: i64) -> AnyWidget {
    let color = theme.on_surface_color();
    Row::new()
        .vertical_alignment(BoxAlignment::Center)
        .gaps(LayoutSpacing::horizontal(6.into()))
        .children([
            icon(glyph, color),
            Text::new(format_count(count))
                .text_style(TextStyle::new().font_size(14).color(color))
                .boxed(),
        ])
        .boxed()
}

/// A count as shown on the card: exact below 10,000, then in thousands (`10K`,
/// `12.5K`) and millions (`1.2M`).
pub(crate) fn format_count(count: i64) -> String {
    fn scaled(count: i64, unit: i64, suffix: &str) -> String {
        // Truncate to one decimal, not round, so 19,999 never reads as `20K`.
        let tenths = count / (unit / 10);
        if tenths % 10 == 0 {
            format!("{}{suffix}", tenths / 10)
        } else {
            format!("{}.{}{suffix}", tenths / 10, tenths % 10)
        }
    }
    match count {
        ..10_000 => count.to_string(),
        10_000..1_000_000 => scaled(count, 1_000, "K"),
        _ => scaled(count, 1_000_000, "M"),
    }
}

/// An icon with its count, as plain text: nothing to press.
pub(crate) fn count_label(theme: ForumTheme, glyph: &'static StaticSvg, count: i64) -> AnyWidget {
    Container::new()
        .height(Dimension::Px(ROW_HEIGHT))
        .child(
            Align::new()
                .alignment(Alignment::MidCenter)
                .child(count_row(theme, glyph, count)),
        )
        .boxed()
}

fn icon(glyph: &'static StaticSvg, color: aimer::Color) -> AnyWidget {
    Svg::new(glyph)
        .style("path", SvgStyle::new().stroke(color))
        .width(ICON_SIZE)
        .height(ICON_SIZE)
        .boxed()
}

/// Turns the excerpt Discourse sends (HTML text with entities) into plain text
/// of at most [`EXCERPT_LIMIT`] characters.
pub(crate) fn clean_excerpt(raw: &str) -> String {
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

/// Formats a Discourse ISO-8601 timestamp as a short relative time
/// (e.g. `just now`, `5m ago`, `3h ago`, `12d ago`, `4mo ago`), falling back
/// to the raw date for anything older than a year.
pub fn relative_time(iso: &str) -> String {
    let Some((year, month, day, hour, minute, second)) = parse_iso8601(iso) else {
        return "—".to_string();
    };
    let days = days_from_civil(year, month as i64, day as i64);
    let ts = days * 86_400 + hour as i64 * 3_600 + minute as i64 * 60 + second as i64;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let delta = now - ts;
    if delta < 60 {
        "just now".to_string()
    } else if delta < 3_600 {
        format!("{}m ago", delta / 60)
    } else if delta < 86_400 {
        format!("{}h ago", delta / 3_600)
    } else if delta < 7 * 86_400 {
        format!("{}d ago", delta / 86_400)
    } else if delta < 365 * 86_400 {
        format!("{}mo ago", delta / (30 * 86_400))
    } else {
        format!("{year}-{month:02}-{day:02}")
    }
}

/// Parses a timestamp like `2022-06-24T15:56:48.339Z` into its date/time parts.
fn parse_iso8601(iso: &str) -> Option<(i64, u32, u32, u32, u32, u32)> {
    let year = iso.get(0..4)?.parse::<i64>().ok()?;
    let month = iso.get(5..7)?.parse::<u32>().ok()?;
    let day = iso.get(8..10)?.parse::<u32>().ok()?;
    let hour = iso.get(11..13)?.parse::<u32>().ok()?;
    let minute = iso.get(14..16)?.parse::<u32>().ok()?;
    let second = iso.get(17..19)?.parse::<u32>().ok()?;
    Some((year, month, day, hour, minute, second))
}

/// Days since 1970-01-01 for a proleptic Gregorian date (Howard Hinnant's
/// algorithm), used to turn a timestamp's calendar date into a second count.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_from_civil_matches_known_dates() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(1970, 1, 2), 1);
        assert_eq!(days_from_civil(2000, 1, 1), 10_957);
        assert_eq!(days_from_civil(2022, 6, 24), 19_167);
    }

    #[test]
    fn clean_excerpt_decodes_entities_and_collapses_whitespace() {
        assert_eq!(
            clean_excerpt("a &amp; b\n  &quot;c&quot; &hellip;"),
            "a & b \"c\" …"
        );
    }

    #[test]
    fn clean_excerpt_truncates_long_text() {
        let cleaned = clean_excerpt(&"word ".repeat(100));
        assert!(cleaned.chars().count() <= EXCERPT_LIMIT + 1);
        assert!(cleaned.ends_with('…'));
    }

    #[test]
    fn format_count_abbreviates_from_ten_thousand() {
        assert_eq!(format_count(9_999), "9999");
        assert_eq!(format_count(10_000), "10K");
        assert_eq!(format_count(12_500), "12.5K");
        assert_eq!(format_count(19_999), "19.9K");
        assert_eq!(format_count(1_200_000), "1.2M");
    }

    #[test]
    fn parse_iso8601_reads_parts() {
        assert_eq!(
            parse_iso8601("2022-06-24T15:56:48.339Z"),
            Some((2022, 6, 24, 15, 56, 48))
        );
    }
}
