use crate::app_shell::is_mobile;
use crate::components::button_reveal::ButtonReveal;
use crate::components::general_button::GeneralButton;
use crate::components::search_header::{STACKED_EXTRA, SearchHeader};
use crate::components::side_bar::{RAIL_MARGIN, RAIL_WIDTH, rail_height};
use crate::layout::{RailPlace, ShellLayout, rail_band, rail_margin, rail_place, shell_layout};
use crate::preferences::NavTab;
use crate::router::AppRouter;
use crate::store::AppStore;
use crate::theme::ForumTheme;
use aimer::animation::{AnimationController, FadeTransition};
use aimer::console::info;
use aimer::router::NavigatorController;
use aimer::screen::MediaQuery;
use aimer::style::{LayoutSpacing, TextOverflow, TextStyle, Theme};
use aimer::{
    Align, Alignment, AnyWidget, BoxAlignment, BuildContext, Column, Container, Dimension,
    Expanded, FontWeight, Icons, NumberExt, ProviderContext, ProviderHandle, Row, SizedBox, Stack,
    StatelessWidget, StaticSvg, Svg, SvgStyle, Text, Widget, ZeroSizedBox,
};

/// Diameter of the round back button, and the height of the like/comment buttons.
const BUTTON_SIZE: f32 = 40.0;
const ICON_SIZE: u32 = 20;
/// The height of the bar over a page.
const BAR_HEIGHT: f32 = 60.0;
/// The like and comment buttons are wide enough for an icon and a short count.
const COUNT_BUTTON_WIDTH: f32 = 72.0;
/// The room above the column of buttons on the trailing edge, where the status bar
/// and the camera are.
const VERTICAL_PADDING: u32 = 8;
/// The space between the buttons of the column.
const VERTICAL_GAP: u32 = 6;
/// The like and comment buttons in the column are vertical pills, centred on the
/// line of the tab rail below them: narrow, and tall enough for an icon with its
/// count under it.
const COLUMN_BUTTON_WIDTH: f32 = 48.0;
const COLUMN_BUTTON_HEIGHT: f32 = 60.0;
/// The room the column keeps above its first button, below the top it is given.
const COLUMN_TOP_PADDING: u32 = 12;
/// The space between the icon and the count of a button in the column.
const LABEL_GAP: u32 = 2;
const LABEL_SIZE: u32 = 11;
/// How far down the trailing edge the system's own controls reach: the camera, the
/// time and the connection indicator stacked at the top, on the outer display of
/// an iPhone Duo. The column of buttons starts below them.
const STATUS_CLEARANCE: f32 = 156.0;
/// The tallest the column gets: the back and settings buttons, round, and the like
/// and comment pills.
const COLUMN_MAX_HEIGHT: f32 = COLUMN_BUTTON_WIDTH * 2.0
    + COLUMN_BUTTON_HEIGHT * 2.0
    + VERTICAL_GAP as f32 * 3.0
    + VERTICAL_PADDING as f32 * 2.0;

/// The size and arrangement of a button: the back and settings buttons are round,
/// 40 px; the like and comment buttons lie as wide pills along the top, and in the
/// column on the trailing edge stand as tall pills with the icon on top and the
/// count below.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ButtonShape {
    width: f32,
    height: f32,
    stacked: bool,
}

impl ButtonShape {
    /// The round button that holds an icon alone. In the column it is as wide as
    /// the pills and the tab rail, so all of them share one width and one line.
    fn round(vertical: bool) -> Self {
        let size = if vertical {
            COLUMN_BUTTON_WIDTH
        } else {
            BUTTON_SIZE
        };
        Self {
            width: size,
            height: size,
            stacked: false,
        }
    }

    /// The button that holds an icon and a count: beside each other in a wide pill
    /// along the top, one over the other in a tall pill in the column.
    fn counted(vertical: bool) -> Self {
        if vertical {
            Self {
                width: COLUMN_BUTTON_WIDTH,
                height: COLUMN_BUTTON_HEIGHT,
                stacked: true,
            }
        } else {
            Self {
                width: COUNT_BUTTON_WIDTH,
                height: BUTTON_SIZE,
                stacked: false,
            }
        }
    }

    /// The radius of the pill the button reveals itself over.
    fn radius(self) -> f32 {
        self.width.min(self.height) / 2.0
    }
}

/// How far from the top of a window `height` tall the column starts: right below
/// the system's controls (`system_top`, where it reports them, and at least
/// [`STATUS_CLEARANCE`] where it does not), but no lower than leaves room for the
/// whole column in the window.
///
/// On a page that keeps the tab rail (`rail` is its tab count), the rail makes
/// way for the column, so the column stays under the status area; only where the
/// rail cannot, in the bottom corner, does the column rise above it.
fn column_top(height: f32, system_top: f32, rail: Option<usize>) -> u32 {
    let wanted = STATUS_CLEARANCE.max(system_top);
    let room = match rail {
        Some(tabs)
            if rail_place(height, tabs, column_clear(system_top)) == RailPlace::Corner =>
        {
            height - rail_height(tabs) - RAIL_MARGIN as f32 * 2.0 - COLUMN_MAX_HEIGHT
        }
        _ => height - COLUMN_MAX_HEIGHT,
    };
    wanted.min(room).max(VERTICAL_PADDING as f32).round() as u32
}

/// How far down a single button of the column reaches when the column starts
/// where [`column_top`] puts it below the status area: what the tab rail, on the
/// page that has both, keeps clear of.
pub fn column_clear(system_top: f32) -> f32 {
    STATUS_CLEARANCE.max(system_top) + COLUMN_TOP_PADDING as f32 + COLUMN_BUTTON_WIDTH
}

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
    /// Whether the buttons stand in a column on the trailing edge, on the line the
    /// status bar and the tab rail are on, instead of lying along the top.
    vertical: bool,
    /// Whether the page keeps the tab rail, which the column has to share the edge
    /// with.
    beside_rail: bool,
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
            vertical: false,
            beside_rail: false,
        }
    }

    /// Tells the bar the page keeps the tab rail on the trailing edge, so the
    /// column leaves room for it where it needs to.
    pub fn beside_rail(mut self, beside_rail: bool) -> Self {
        self.beside_rail = beside_rail;
        self
    }

    /// Stands the buttons along the trailing edge, for a window that is wide but
    /// short: the controls keep to the vertical axis the system's own are on, the
    /// primary navigation (back) first, and the page keeps its height.
    pub fn vertical(mut self, vertical: bool) -> Self {
        self.vertical = vertical;
        self
    }
}

impl StatelessWidget for ActionBar {
    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme = *ForumTheme::of(ctx);
        let navigator = NavigatorController::<AppRouter>::of(ctx);
        // The back and settings buttons are round; the like and comment buttons are
        // pills for a count, tall ones in the column.
        let round = ButtonShape::round(self.vertical);
        let counted = ButtonShape::counted(self.vertical);

        let back = match self.back.clone() {
            Some(controller) => ButtonReveal::new(
                controller,
                round.width,
                round.height,
                round.radius(),
                move || back_button(navigator.clone(), theme, round),
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
                    .height(BUTTON_SIZE.px())
                    .padding(LayoutSpacing::horizontal(8.into()))
                    .child(
                        Align::new().alignment(Alignment::MidLeft).child(
                            Text::new(name.clone()).text_style(
                                TextStyle::new()
                                    .text_overflow(TextOverflow::Ellipsis)
                                    .font_weight(FontWeight::Bold)
                                    .font_size(20)
                                    .color(theme.on_background_color()),
                            ),
                        ),
                    ),
            )
            .notype(),
            None => ZeroSizedBox.notype(),
        };

        // Past the others, at the far end of the bar.
        let settings = self.settings.clone().map(|controller| {
            let navigator = NavigatorController::<AppRouter>::of(ctx);
            ButtonReveal::new(
                controller,
                round.width,
                round.height,
                round.radius(),
                move || settings_button(navigator.clone(), theme, round),
            )
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
                        counted.width,
                        counted.height,
                        counted.radius(),
                        move || like_button(theme, like_count, id, counted),
                    )
                    .boxed(),
                    ButtonReveal::new(
                        controller.clone(),
                        counted.width,
                        counted.height,
                        counted.radius(),
                        move || comment_button(theme, comment_count, id, counted),
                    )
                    .boxed(),
                )
            }
            None => (ZeroSizedBox.boxed(), ZeroSizedBox.boxed()),
        };

        // The buttons' row is the same row whatever is in it, so that a page that
        // brings or takes away buttons moves nothing else. A place with no button
        // is empty, and the spacer between the two ends takes what is left.
        let settings = settings.unwrap_or_else(|| ZeroSizedBox.boxed());
        let buttons = if self.vertical {
            // The back button is first, as the top of the vertical axis is for the
            // primary navigation; the title, which cannot stand on its side, keeps
            // the top of the page, clear of the column.
            let height = MediaQuery::of(ctx).size.height;
            let system = ctx.watch_safe_area_insets();
            let rail = (self.beside_rail && shell_layout(ctx) == ShellLayout::Rail)
                .then(|| ctx.select::<AppStore, usize>(|s| s.navbar.len()));
            let top = column_top(height, system.top, rail);
            let column = Container::new()
                .width(Dimension::Px(RAIL_WIDTH))
                .padding(
                    LayoutSpacing::new()
                        .top(top + COLUMN_TOP_PADDING)
                        .bottom(VERTICAL_PADDING),
                )
                .child(
                    Column::new()
                        .horizontal_alignment(BoxAlignment::Center)
                        .gaps(LayoutSpacing::vertical(VERTICAL_GAP))
                        .children([back, likes, comments, settings]),
                );
            Row::new()
                .vertical_alignment(BoxAlignment::Start)
                .children([
                    Container::new()
                        .height(Dimension::Px(BUTTON_SIZE + VERTICAL_PADDING as f32 * 2.0))
                        .padding(LayoutSpacing::horizontal(8.into()))
                        .child(title)
                        .boxed(),
                    Expanded::new().child(SizedBox::new()).boxed(),
                    column.boxed(),
                    // The same margin the tab rail keeps from the edge, so the two
                    // stand on one vertical line, the system's own.
                    SizedBox::new()
                        .width(Dimension::Px(rail_margin(system.right) as f32))
                        .boxed(),
                ])
                .boxed()
        } else {
            Container::new()
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
                            settings,
                        ]),
                )
                .boxed()
        };

        // The search is a layer of its own under the buttons, with the whole bar to
        // itself. Moving between it and a post is then a cross-fade of two layers
        // that don't share a row, so the field doesn't stretch or shrink as the
        // buttons come and go, and it stays where it is for the whole motion.
        let Some(search_controls) = &self.search else {
            return buttons;
        };
        let initial = ProviderHandle::<AppStore>::of(ctx)
            .read()
            .search_term
            .clone();
        // On a narrow window the search has two rows, and its layer grows to hold
        // them; the buttons keep the height of the bar they are centred in.
        let height = BAR_HEIGHT + if is_mobile(ctx) { STACKED_EXTRA } else { 0.0 };
        // The field and the toggle come in and go out on their own controllers, so
        // the layer around them has no fade of its own.
        // Beside the column of buttons, the search keeps clear of the rail's width.
        let clear = if self.vertical {
            rail_band(&ctx.watch_safe_area_insets()).round() as u32
        } else {
            10
        };
        let search = Container::new()
            .height(Dimension::Px(height))
            .padding(LayoutSpacing::all(10).right(clear))
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

fn like_button(theme: ForumTheme, count: i64, post_id: i64, shape: ButtonShape) -> AnyWidget {
    GeneralButton::with_child(
        button_content(theme, Icons::HEART, Some(count), count.to_string(), shape),
        move || info!("Like requested for post {}", post_id),
    )
    .width(shape.width)
    .boxed()
}

fn comment_button(theme: ForumTheme, count: i64, post_id: i64, shape: ButtonShape) -> AnyWidget {
    GeneralButton::with_child(
        button_content(
            theme,
            Icons::MESSAGE_CIRCLE,
            Some(count),
            count.to_string(),
            shape,
        ),
        move || info!("Comment requested for post {}", post_id),
    )
    .width(shape.width)
    .boxed()
}

/// The button that opens the settings.
fn settings_button(
    navigator: NavigatorController<AppRouter>,
    theme: ForumTheme,
    shape: ButtonShape,
) -> AnyWidget {
    GeneralButton::with_child(
        button_content(theme, Icons::SETTINGS, None, String::new(), shape),
        move || navigator.push(AppRouter::Settings),
    )
    .width(shape.width)
    .boxed()
}

/// The button that goes back a page, or home when there is no page behind.
fn back_button(
    navigator: NavigatorController<AppRouter>,
    theme: ForumTheme,
    shape: ButtonShape,
) -> AnyWidget {
    GeneralButton::with_child(
        button_content(theme, Icons::CHEVRON_LEFT, None, String::new(), shape),
        move || {
            if navigator.can_pop() {
                navigator.pop();
            } else {
                navigator.push(AppRouter::Latest);
            }
        },
    )
    .width(shape.width)
    .boxed()
}

/// What goes inside a button. Lying along the top: the icon, with its count beside
/// it when it has one. In a stacked button: the icon on top and `label` under it.
fn button_content(
    theme: ForumTheme,
    icon: &'static StaticSvg,
    count: Option<i64>,
    label: String,
    shape: ButtonShape,
) -> impl Widget {
    let icon = Svg::new(icon)
        .style("path", SvgStyle::new().stroke(theme.on_background_color()))
        .width(ICON_SIZE)
        .height(ICON_SIZE);

    let content = if shape.stacked {
        Column::new()
            .horizontal_alignment(BoxAlignment::Center)
            .gaps(LayoutSpacing::vertical(LABEL_GAP))
            .children([
                icon.boxed(),
                Text::new(label)
                    .text_style(
                        TextStyle::new()
                            .font_size(LABEL_SIZE)
                            .color(theme.on_background_color()),
                    )
                    .boxed(),
            ])
            .boxed()
    } else {
        match count {
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
        }
    };

    Container::new()
        .width(Dimension::Px(shape.width))
        .height(Dimension::Px(shape.height))
        .child(Align::new().alignment(Alignment::MidCenter).child(content))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_the_column_like_and_comment_are_tall_pills_with_the_icon_over_the_count() {
        let counted = ButtonShape::counted(true);
        assert!(counted.stacked);
        assert!(counted.height > counted.width);
        assert!(counted.width <= RAIL_WIDTH);
        // Room for the icon, the gap and the count.
        assert!(counted.height >= ICON_SIZE as f32 + LABEL_GAP as f32 + LABEL_SIZE as f32 * 1.3);
    }

    #[test]
    fn the_back_button_stays_round_with_just_its_icon_in_the_column_too() {
        let round = ButtonShape::round(true);
        assert!(!round.stacked);
        assert_eq!(
            (round.width, round.height),
            (COLUMN_BUTTON_WIDTH, COLUMN_BUTTON_WIDTH)
        );
        assert_eq!(round.width, ButtonShape::counted(true).width);
        assert_eq!(ButtonShape::round(false).width, BUTTON_SIZE);
    }

    #[test]
    fn along_the_top_like_and_comment_keep_the_icon_beside_the_count() {
        let counted = ButtonShape::counted(false);
        assert!(!counted.stacked);
        assert_eq!(
            (counted.width, counted.height),
            (COUNT_BUTTON_WIDTH, BUTTON_SIZE)
        );
    }

    #[test]
    fn the_column_starts_right_below_the_camera_and_status() {
        assert_eq!(column_top(678.0, 0.0, None), STATUS_CLEARANCE as u32);
        // The system's own larger clearance wins.
        assert_eq!(column_top(900.0, 200.0, None), 200);
        // A window too short for it lifts the column so that it fits.
        assert_eq!(column_top(400.0, 0.0, None) as f32, 400.0 - COLUMN_MAX_HEIGHT);
    }

    #[test]
    fn the_column_keeps_its_place_and_the_rail_makes_way_where_it_can() {
        // The rail is pushed below the column, so the column stays put.
        assert_eq!(column_top(678.0, 0.0, Some(5)), STATUS_CLEARANCE as u32);
        assert!(matches!(
            rail_place(678.0, 5, column_clear(0.0)),
            RailPlace::Below(_)
        ));
        // Where the rail is in the corner, the column rises above it instead.
        assert_eq!(rail_place(400.0, 5, column_clear(0.0)), RailPlace::Corner);
        // There is no room for all of it above the rail in so short a window, so
        // the column goes as high as it can.
        assert_eq!(column_top(400.0, 0.0, Some(5)), VERTICAL_PADDING);
        // Taller, the room above the rail decides.
        let height = 600.0;
        assert_eq!(rail_place(height, 5, column_clear(0.0)), RailPlace::Corner);
        let top = column_top(height, 0.0, Some(5)) as f32;
        assert!(top + COLUMN_MAX_HEIGHT <= height - rail_height(5) - RAIL_MARGIN as f32 * 2.0 + 0.5);
    }

    #[test]
    fn in_a_short_window_the_column_gives_up_the_clearance_to_fit() {
        let height = 400.0;
        let top = column_top(height, 0.0, None) as f32;
        assert!(top < STATUS_CLEARANCE);
        assert!(top + COLUMN_MAX_HEIGHT <= height + 0.5);
    }
}
