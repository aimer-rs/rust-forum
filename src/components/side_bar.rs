mod category_row;
mod category_section;
mod glass;
mod mobile_bar;
mod nav_list;
pub mod skeleton_category_row;
mod user_menu;
use std::time::Duration;

use crate::components::side_bar::category_section::CategorySection;
use crate::components::side_bar::nav_list::{NavigationRow, SearchRow};
use crate::components::side_bar::user_menu::UserMenuButton;
use crate::router::AppRouter;
use crate::theme::ForumTheme;
use aimer::animation::{Animatable, ImplicitAnimatedBuilder, curves};
use aimer::style::{BoxDecoration, LayoutSpacing, Theme};
use aimer::{
    BoxAlignment, BuildContext, Color, Column, Container, Dimension, Direction, Expanded, Row,
    Scrollable, SizedBox, State, StateUpdater, StatefulWidget, Widget,
};
use aimer::{Resizable};

pub use mobile_bar::{BAR_HEIGHT, MOBILE_BREAKPOINT, MobileNavbar, bar_width, glass, icon_of, preview_bar, tab_icon};

pub(crate) const SIDE_BAR_ICONS: &[&str] = &[
    "assets/app-apps-applications-tiles-svgrepo-com(1).svg",
    "assets/clock-two-thirty-svgrepo-com.svg",
    "assets/hot-svgrepo-com.svg",
];

const START_WIDTH: f32 = 250.0;
const MIN_WIDTH: f32 = 200.0;
const MAX_WIDTH: f32 = 500.0;
const HANDLE_THICKNESS: f32 = 14.0;
const HANDLE_OUTSET: f32 = 12.0;

/// The border along the sidebar's right edge, in logical pixels.
const BORDER_WIDTH: f32 = 1.0;
/// What the resize highlight adds beside it.
const HIGHLIGHT_WIDTH: f32 = 2.0;

#[derive(StatefulWidget)]
pub struct AppSideBar;

impl AppSideBar {
    pub fn new() -> Self {
        Self
    }
}

pub struct AppSidebarState {
    resizable: bool,
    updater: StateUpdater<Self>,
}

impl StatefulWidget for AppSideBar {
    type State = AppSidebarState;

    fn create_state(self) -> Self::State {
        AppSidebarState {
            resizable: false,
            updater: StateUpdater::new(),
        }
    }
}

impl State<AppSideBar> for AppSidebarState {
    fn init_state(&mut self, updater: StateUpdater<Self>)
    where
        Self: Sized,
    {
        self.updater = updater;
    }

    fn adopt_config_from(&mut self, _new: Self) {}

    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let zone_updater = self.updater.clone();
        let theme = *ForumTheme::of(ctx);

        Container::new()
            .box_decoration(BoxDecoration::new().background_color(theme.surface_color()))
            .child(
                Resizable::new()
                    .height(f32::MAX)
                    .width(START_WIDTH)
                    .min_width(MIN_WIDTH)
                    .max_width(MAX_WIDTH)
                    .handle_thickness(HANDLE_THICKNESS)
                    .handle_outset(HANDLE_OUTSET)
                    .direction(Direction::RIGHT)
                    .on_resize_zone(move |zone| {
                        zone_updater.set_state(move |state| {
                            state.resizable = zone != Direction::NONE;
                        });
                    })
                    .child(
                        Row::new().children([
                            Expanded::new().child(sidebar_content()).boxed(),
                            // Only the strip animates, so the 300 ms highlight rebuilds
                            // and re-synchronizes two bars and not the whole sidebar.
                            ImplicitAnimatedBuilder::new(
                                if self.resizable { 1.0 } else { 0.0 },
                                Duration::from_millis(300),
                                curves::EASE_IN_OUT,
                                move |delta| border_strip(theme, *delta),
                            )
                            .boxed(),
                        ]),
                    ),
            )
    }
}

/// The sidebar's right border: a fixed-width strip whose colours do the animating.
///
/// A thin line at rest, a wider and brighter one while the resize edge is under the
/// pointer. Both bars keep their width through the fade and only change colour, so
/// nothing is laid out again while it runs. The strip sits inside the sidebar's box,
/// so a page transition beside the sidebar cannot paint over it.
fn border_strip(theme: ForumTheme, delta: f32) -> impl Widget {
    Row::new().children([
        SizedBox::new()
            .width(Dimension::Px(HIGHLIGHT_WIDTH))
            .height(Dimension::Percent(100.0))
            .color(theme.primary_color().with_alpha(delta))
            .boxed(),
        SizedBox::new()
            .width(Dimension::Px(BORDER_WIDTH))
            .height(Dimension::Percent(100.0))
            .color(theme.primary_color().with_alpha(0.1.lerp(&1.0, delta)))
            .boxed(),
    ])
}

/// The sidebar's rows.
fn sidebar_content() -> impl Widget {
    Container::new()
        .padding(LayoutSpacing::horizontal(12.into()))
        .child(
            Column::new()
                .vertical_alignment(BoxAlignment::Start)
                .children([
                    Expanded::new()
                        .child(
                            Scrollable::new().vertical_scroll_bar(None).child(
                                Column::new().children([
                                    // #[cfg(target_os = "macos")]
                                    SizedBox::new().height(36).boxed(),
                                    Column::new()
                                        .gaps(LayoutSpacing::all(2))
                                        .children([
                                            
                                            NavigationRow::new(
                                                "Latest",
                                                AppRouter::Latest,
                                                SIDE_BAR_ICONS[0],
                                            ).boxed(),
                                            SearchRow.boxed(),
                                            NavigationRow::new(
                                                "New",
                                                AppRouter::New,
                                                SIDE_BAR_ICONS[1],
                                            ).boxed(),
                                            NavigationRow::new(
                                                "Hot",
                                                AppRouter::Hot,
                                                SIDE_BAR_ICONS[2],
                                            ).boxed(),
                                        ])
                                        .boxed(),
                                    SizedBox::new().height(16).boxed(),
                                    CategorySection.boxed(),
                                ]),
                            ),
                        )
                        .boxed(),
                    SizedBox::new().height(12).boxed(),
                    UserMenuButton.boxed(),
                    SizedBox::new().height(12).boxed(),
                ]),
        )
}

pub(crate) fn parse_hex_color(hex: &str) -> Color {
    match u32::from_str_radix(hex, 16) {
        Ok(rgb) => Color::Hex(rgb),
        Err(_) => Color::GRAY,
    }
}
