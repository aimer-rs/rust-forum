//! The placeholder shown while a profile loads.

use crate::components::skeleton_topic_card::{PULSE_CYCLE, bar_px, bar_widget, pulse_color};
use crate::components::topic_card::divider_decor;
use crate::theme::ForumTheme;
use aimer::animation::{AnimatedBuilder, AnimationController, curves};
use aimer::style::{BoxDecoration, LayoutSpacing, Theme};
use aimer::{
    Align, Alignment, AnyWidget, BoxAlignment, BuildContext, Color, Column, Container, Dimension,
    Expanded, Row, SizedBox, StatelessWidget, Widget, ZeroSizedBox,
};

use super::AVATAR_SIZE;

/// The height of a line of counts on a topic row, which the placeholder keeps so
/// that the page does not shift when the real rows replace it.
const COUNT_ROW_HEIGHT: f32 = 28.0;

/// A grey placeholder that mirrors [`ProfileContent`](super::ProfileContent)'s
/// shape and pulses gently while the profile loads: the avatar with the lines
/// beside it, the nine stat tiles, and a few rows of topics and of categories.
#[derive(Clone, StatelessWidget)]
pub struct SkeletonProfile {
    controller: AnimationController,
}

impl SkeletonProfile {
    pub fn new() -> Self {
        let controller = AnimationController::new(PULSE_CYCLE, curves::EASE_IN_OUT);
        controller.set_repeat(true);
        controller.set_auto_reverse(true);
        controller.forward_from_first_tick();
        Self { controller }
    }
}

impl StatelessWidget for SkeletonProfile {
    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme = *ForumTheme::of(ctx);
        let controller = self.controller.clone();

        AnimatedBuilder::new(controller, move |value| {
            let bar = pulse_color(theme, value);

            Container::new().padding(LayoutSpacing::horizontal(24.into())).child(
                Column::new().gaps(LayoutSpacing::all(20)).children([
                    header(bar),
                    stats(theme, bar),
                    section_title(bar, 110.0),
                    topic_rows(theme, bar),
                    section_title(bar, 140.0),
                    category_rows(bar),
                ]),
            )
        })
    }
}

/// The card: the avatar with the name and three counts beside it, and the lines
/// of the user's title, role and dates under them.
fn header(bar: Color) -> AnyWidget {
    let count = || {
        Expanded::new().box_child(Column::new().gaps(LayoutSpacing::all(4)).children([
            bar_px(bar, 20.0, 40.0),
            bar_px(bar, 14.0, 52.0),
        ]))
    };
    Column::new()
        .gaps(LayoutSpacing::all(14))
        .children([
            Row::new()
                .vertical_alignment(BoxAlignment::Center)
                .gaps(LayoutSpacing::horizontal(24.into()))
                .children([
                    circle(bar, AVATAR_SIZE),
                    Expanded::new().box_child(Column::new().gaps(LayoutSpacing::all(10)).children([
                        bar_px(bar, 18.0, 140.0),
                        Row::new()
                            .gaps(LayoutSpacing::horizontal(8.into()))
                            .children([count(), count(), count()])
                            .boxed(),
                    ])),
                ])
                .boxed(),
            Column::new()
                .gaps(LayoutSpacing::all(4))
                .children([
                    bar_px(bar, 14.0, 120.0),
                    bar_px(bar, 14.0, 150.0),
                    bar_px(bar, 14.0, 110.0),
                ])
                .boxed(),
        ])
        .boxed()
}

/// The activity figures: nine tiles in a 3×3 grid.
fn stats(theme: ForumTheme, bar: Color) -> AnyWidget {
    let tile = || {
        Expanded::new().box_child(
            Container::new()
                .padding(LayoutSpacing::all(10))
                .box_decoration(
                    BoxDecoration::new()
                        .background_color(theme.on_background_color().with_alpha(0.06))
                        .border_radius(4),
                )
                .child(Column::new().gaps(LayoutSpacing::all(6)).children([
                    bar_px(bar, 20.0, 44.0),
                    bar_px(bar, 12.0, 70.0),
                ])),
        )
    };
    let row = || Row::new().gaps(LayoutSpacing::horizontal(10.into())).children([tile(), tile(), tile()]).boxed();

    Column::new().gaps(LayoutSpacing::all(4)).children([row(), row(), row()]).boxed()
}

/// A section's heading.
fn section_title(bar: Color, width: f32) -> AnyWidget {
    Container::new().padding(LayoutSpacing::new().top(8)).box_child(bar_px(bar, 20.0, width))
}

/// Three rows like the user's top topics: a title, then counts and a date.
fn topic_rows(theme: ForumTheme, bar: Color) -> AnyWidget {
    let row = |title_width: f32| {
        Container::new()
            .box_decoration(divider_decor(theme))
            .child(
                Container::new().padding(LayoutSpacing::vertical(14)).child(
                    Column::new().gaps(LayoutSpacing::all(4)).children([
                        bar_widget(bar, 17.0, title_width).boxed(),
                        Row::new()
                            .vertical_alignment(BoxAlignment::Center)
                            .gaps(LayoutSpacing::horizontal(20.into()))
                            .children([
                                count_slot(bar_px(bar, 16.0, 44.0)),
                                count_slot(bar_px(bar, 16.0, 44.0)),
                                Expanded::new().box_child(SizedBox::new()),
                                bar_px(bar, 13.0, 56.0),
                            ])
                            .boxed(),
                    ]),
                ),
            )
            .boxed()
    };
    Column::new().children([row(90.0), row(70.0), row(80.0)]).boxed()
}

/// Two rows like the user's top categories: a color dot, a name and the counts.
fn category_rows(bar: Color) -> AnyWidget {
    let row = |name_width: f32| {
        Row::new()
            .vertical_alignment(BoxAlignment::Center)
            .gaps(LayoutSpacing::horizontal(10.into()))
            .children([
                circle(bar, 12.0),
                bar_px(bar, 16.0, name_width),
                Expanded::new().box_child(SizedBox::new()),
                bar_px(bar, 14.0, 90.0),
            ])
            .boxed()
    };
    Column::new().gaps(LayoutSpacing::all(10)).children([row(120.0), row(96.0)]).boxed()
}

/// `content` centred in a line as tall as a topic row's counts.
fn count_slot(content: AnyWidget) -> AnyWidget {
    Container::new()
        .height(Dimension::Px(COUNT_ROW_HEIGHT))
        .child(Align::new().alignment(Alignment::MidCenter).child(content))
        .boxed()
}

fn circle(color: Color, size: f32) -> AnyWidget {
    Container::new()
        .width(Dimension::Px(size))
        .height(Dimension::Px(size))
        .box_decoration(BoxDecoration::new().background_color(color).border_radius(size / 2.0))
        .child(ZeroSizedBox)
        .boxed()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ForumTheme;
    use aimer::quiver::winit::dpi::PhysicalSize;
    use aimer::style::AnimatedTheme;
    use aimer::{AimerApp, HeadlessOptions};

    /// A skeleton on its own, under the theme the app gives every screen.
    fn skeleton_app() -> aimer::HeadlessAimerApp<impl aimer::Widget + 'static> {
        crate::testing::hold();
        let mut app = AimerApp::start_headless_with(
            AnimatedTheme::new().data(ForumTheme::light()).child(SkeletonProfile::new()),
            HeadlessOptions { size: PhysicalSize::new(450, 900), scale_factor: 1.0 },
        );
        app.pump_frames(6);
        app
    }

    #[test]
    fn the_skeleton_paints_a_whole_profile_through_the_retained_path() {
        let app = skeleton_app();
        let census = app.paint_source_census().expect("a mounted page has a root");

        // An avatar, a handful of lines beside it, nine stat tiles and the rows of
        // a section: about 170 nodes when this was measured, where a line of
        // "Loading…" is a handful.
        assert!(census.local_v2 >= 100, "only {} nodes paint", census.local_v2);
        assert!(census.drawn_unmapped.is_empty(), "unmapped {:?}", census.drawn_unmapped);
        assert!(census.invalid_bounds.is_empty(), "invalid bounds {:?}", census.invalid_bounds);
        assert!(census.declined_paint.is_empty(), "declined {:?}", census.declined_paint);
        assert!(census.sync_error.is_none(), "{:?}", census.sync_error);
    }

    #[test]
    fn the_skeleton_keeps_pulsing_until_it_is_replaced() {
        let mut app = skeleton_app();

        // A repeating animation never lets the app go idle.
        for _ in 0..5 {
            std::thread::sleep(std::time::Duration::from_millis(20));
            assert!(app.pump_frames(1) > 0, "the pulse stopped");
        }
    }
}
