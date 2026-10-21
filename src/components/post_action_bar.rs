use crate::components::button_reveal::ButtonReveal;
use crate::components::general_button::GeneralButton;
use crate::app_shell::is_mobile;
use crate::components::search_header::{STACKED_EXTRA, SearchHeader};
use crate::router::AppRouter;
use crate::store::AppStore;
use crate::theme::ForumTheme;
use aimer::animation::{AnimationController, FadeTransition};
use aimer::console::info;
use aimer::router::NavigatorController;
use aimer::style::{LayoutSpacing, TextOverflow, TextStyle, Theme};
use aimer::{
    Align, Alignment, AnyWidget, BoxAlignment, BuildContext, Container, Dimension, Expanded, Icons,
    FontWeight, ProviderContext, ProviderHandle, Row, SizedBox, Stack, StatelessWidget, StaticSvg, Svg, SvgStyle, Text, Widget,
    ZeroSizedBox,
};

/// Diameter of the round back button, and the height of the like/comment buttons.
const BUTTON_SIZE: f32 = 40.0;
const ICON_SIZE: u32 = 20;
/// The height of the bar over a page.
const BAR_HEIGHT: f32 = 60.0;
/// The like and comment buttons are wide enough for an icon and a short count.
const COUNT_BUTTON_WIDTH: f32 = 72.0;

/// The bar over a page: a back button on the left and, for a post, like and
/// comment buttons on the right.
///
/// Each button has its own controller and comes in or goes out by itself, so
/// moving from a page with a back button to a post brings in only the new
/// buttons and leaves the back button where it is.
#[derive(Clone, StatelessWidget)]
pub struct ActionBar {
    /// The back button's controller, while it is shown.
    back: Option<AnimationController>,
    /// The title beside the back button (a user's name), with its controller.
    title: Option<(AnimationController, String)>,
    /// The like and comment buttons' controller and the post they count.
    counts: Option<(AnimationController, i64)>,
    /// The search field and its toggle, each with its controller, on the search
    /// page.
    search: Option<SearchControls>,
    /// The settings button, with its controller, on the account page.
    settings: Option<AnimationController>,
}

/// What the bar needs to show the search field and its toggle.
#[derive(Clone)]
pub struct SearchControls {
    /// Drives the field: `0.0` out, `1.0` in.
    pub field: AnimationController,
    /// Drives the toggle: `0.0` out, `1.0` in.
    pub toggle: AnimationController,
    /// Whether the field takes the keyboard as it arrives.
    pub auto_focus: bool,
}

impl ActionBar {
    pub fn new(
        back: Option<AnimationController>,
        title: Option<(AnimationController, String)>,
        counts: Option<(AnimationController, i64)>,
        search: Option<SearchControls>,
        settings: Option<AnimationController>,
    ) -> Self {
        Self {
            back,
            title,
            counts,
            search,
            settings,
        }
    }
}

impl StatelessWidget for ActionBar {
    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme = *ForumTheme::of(ctx);
        let navigator = NavigatorController::<AppRouter>::of(ctx);

        let back = match self.back.clone() {
            Some(controller) => ButtonReveal::new(
                controller,
                BUTTON_SIZE,
                BUTTON_SIZE,
                BUTTON_SIZE / 2.0,
                move || back_button(navigator.clone(), theme),
            )
            .boxed(),
            None => ZeroSizedBox.boxed(),
        };

        let title = match &self.title {
            // As tall as the buttons, with the text centred in that height, so that
            // it sits level with the back button however the row lines it up.
            Some((controller, name)) => FadeTransition::new(
                controller.clone(),
                Container::new()
                    .height(Dimension::Px(BUTTON_SIZE))
                    .padding(LayoutSpacing::horizontal(8.into()))
                    .child(Align::new().alignment(Alignment::MidLeft).child(
                        Text::new(name.clone()).text_style(
                            TextStyle::new()
                                .text_overflow(TextOverflow::Ellipsis)
                                .font_weight(FontWeight::Bold)
                                .font_size(20)
                                .color(theme.on_background_color()),
                        ),
                    )),
            )
            .boxed(),
            None => ZeroSizedBox.boxed(),
        };

        // Past the others, at the far end of the bar.
        let settings = self.settings.clone().map(|controller| {
            let navigator = NavigatorController::<AppRouter>::of(ctx);
            ButtonReveal::new(controller, BUTTON_SIZE, BUTTON_SIZE, BUTTON_SIZE / 2.0, move || {
                settings_button(navigator.clone(), theme)
            })
            .boxed()
        });

        let (likes, comments) = match &self.counts {
            Some((controller, id)) => {
                let id = *id;
                let (like_count, comment_count) =
                    ctx.select::<AppStore, (i64, i64)>(move |s| s.post_counts(id));
                (
                    ButtonReveal::new(
                        controller.clone(),
                        COUNT_BUTTON_WIDTH,
                        BUTTON_SIZE,
                        BUTTON_SIZE / 2.0,
                        move || like_button(theme, like_count, id),
                    )
                    .boxed(),
                    ButtonReveal::new(
                        controller.clone(),
                        COUNT_BUTTON_WIDTH,
                        BUTTON_SIZE,
                        BUTTON_SIZE / 2.0,
                        move || comment_button(theme, comment_count, id),
                    )
                    .boxed(),
                )
            }
            None => (ZeroSizedBox.boxed(), ZeroSizedBox.boxed()),
        };

        // The buttons' row is the same row whatever is in it, so that a page that
        // brings or takes away buttons moves nothing else. A place with no button
        // is empty, and the spacer between the two ends takes what is left.
        let buttons = Container::new()
            .height(Dimension::Px(BAR_HEIGHT))
            .padding(LayoutSpacing::all(10))
            .child(
                Row::new()
                    .vertical_alignment(BoxAlignment::Center)
                    .horizontal_alignment(BoxAlignment::Center)
                    .gaps(LayoutSpacing::horizontal(4.into()))
                    .children([
                        back,
                        title,
                        Expanded::new().child(SizedBox::new()).boxed(),
                        likes,
                        comments,
                        settings.unwrap_or_else(|| ZeroSizedBox.boxed()),
                    ]),
            )
            .boxed();

        // The search is a layer of its own under the buttons, with the whole bar to
        // itself. Moving between it and a post is then a cross-fade of two layers
        // that don't share a row, so the field doesn't stretch or shrink as the
        // buttons come and go, and it stays where it is for the whole motion.
        let Some(search_controls) = &self.search else {
            return buttons;
        };
        let initial = ProviderHandle::<AppStore>::of(ctx).read().search_term.clone();
        // On a narrow window the search has two rows, and its layer grows to hold
        // them; the buttons keep the height of the bar they are centred in.
        let height = BAR_HEIGHT + if is_mobile(ctx) { STACKED_EXTRA } else { 0.0 };
        // The field and the toggle come in and go out on their own controllers, so
        // the layer around them has no fade of its own.
        let search = Container::new()
            .height(Dimension::Px(height))
            .padding(LayoutSpacing::all(10))
            .child(
                Row::new()
                    .vertical_alignment(BoxAlignment::Center)
                    .horizontal_alignment(BoxAlignment::Center)
                    .children([Expanded::new()
                        .child(SearchHeader::new(
                            initial,
                            search_controls.field.clone(),
                            search_controls.toggle.clone(),
                            search_controls.auto_focus,
                        ))
                        .boxed()]),
            )
            .boxed();

        Stack::new().children([search, buttons]).boxed()
    }
}

fn like_button(theme: ForumTheme, count: i64, post_id: i64) -> AnyWidget {
    GeneralButton::with_child(
        button_content(theme, Icons::HEART, Some(count), COUNT_BUTTON_WIDTH),
        move || info!("Like requested for post {}", post_id),
    )
    .width(COUNT_BUTTON_WIDTH)
    .boxed()
}

fn comment_button(theme: ForumTheme, count: i64, post_id: i64) -> AnyWidget {
    GeneralButton::with_child(
        button_content(
            theme,
            Icons::MESSAGE_CIRCLE,
            Some(count),
            COUNT_BUTTON_WIDTH,
        ),
        move || info!("Comment requested for post {}", post_id),
    )
    .width(COUNT_BUTTON_WIDTH)
    .boxed()
}

/// The round button that opens the settings.
fn settings_button(navigator: NavigatorController<AppRouter>, theme: ForumTheme) -> AnyWidget {
    GeneralButton::with_child(
        button_content(theme, Icons::SETTINGS, None, BUTTON_SIZE),
        move || navigator.push(AppRouter::Settings),
    )
    .width(BUTTON_SIZE)
    .boxed()
}

/// The round button that goes back a page, or home when there is no page behind.
fn back_button(navigator: NavigatorController<AppRouter>, theme: ForumTheme) -> AnyWidget {
    GeneralButton::with_child(
        button_content(theme, Icons::CHEVRON_LEFT, None, BUTTON_SIZE),
        move || {
            if navigator.can_pop() {
                navigator.pop();
            } else {
                navigator.push(AppRouter::Latest);
            }
        },
    )
    .width(BUTTON_SIZE)
    .boxed()
}

/// What goes inside a button: the icon, with its count beside it when it has one.
fn button_content(
    theme: ForumTheme,
    icon: &'static StaticSvg,
    count: Option<i64>,
    width: f32,
) -> impl Widget {
    let icon = Svg::new(icon)
        .style("path", SvgStyle::new().stroke(theme.on_background_color()))
        .width(ICON_SIZE)
        .height(ICON_SIZE);

    let content = match count {
        Some(count) => Row::new()
            .vertical_alignment(BoxAlignment::Center)
            .gaps(LayoutSpacing::horizontal(6.into()))
            .children([
                icon.boxed(),
                Text::new(count.to_string())
                    .text_style(
                        TextStyle::new()
                            .font_size(14)
                            .color(theme.on_background_color()),
                    )
                    .boxed(),
            ])
            .boxed(),
        None => icon.boxed(),
    };

    Container::new()
        .width(Dimension::Px(width))
        .height(Dimension::Px(BUTTON_SIZE))
        .child(Align::new().alignment(Alignment::MidCenter).child(content))
}
