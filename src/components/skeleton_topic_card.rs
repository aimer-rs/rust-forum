use std::time::Duration;

use crate::components::topic_card::divider_decor;
use crate::theme::ForumTheme;
use aimer::animation::{AnimatedBuilder, AnimationController, curves};
use aimer::style::{BoxDecoration, LayoutSpacing, Theme};
use aimer::{
    AnyWidget, BoxAlignment, BuildContext, Color, Column, Container, Dimension, Expanded, Row,
    SizedBox, StatelessWidget, Widget, ZeroSizedBox,
};

/// How many skeleton cards have been built, for tests that look for one flashing.
#[cfg(test)]
pub(crate) static BUILT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

pub(crate) const PULSE_CYCLE: Duration = Duration::from_millis(1000);

/// A grey placeholder that mirrors [`super::topic_card::TopicCard`]'s shape and
/// pulses gently while the real topic streams in.
#[derive(Clone, StatelessWidget)]
pub struct SkeletonTopicCard {
    controller: AnimationController,
}

impl SkeletonTopicCard {
    pub fn new() -> Self {
        #[cfg(test)]
        BUILT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let controller = AnimationController::new(PULSE_CYCLE, curves::EASE_IN_OUT);
        controller.set_repeat(true);
        controller.set_auto_reverse(true);
        controller.forward_from_first_tick();
        Self { controller }
    }
}

impl StatelessWidget for SkeletonTopicCard {
    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme = *ForumTheme::of(ctx);
        let controller = self.controller.clone();

        AnimatedBuilder::new(controller, move |value| {
            let bar = pulse_color(theme, value);

            Container::new()
                .width(Dimension::Auto)
                .box_decoration(divider_decor(theme))
                .child(
                    Container::new().padding(LayoutSpacing::all(20)).child(
                        Column::new().gaps(LayoutSpacing::all(12)).children([
                            header_row(bar),
                            bar_widget(bar, 22.0, 80.0).boxed(),
                            bar_widget(bar, 16.0, 100.0).boxed(),
                            bar_widget(bar, 16.0, 65.0).boxed(),
                            footer_row(bar),
                        ]),
                    ),
                )
        })
    }
}

/// Mirrors the card's header: a round badge and a short name.
fn header_row(color: Color) -> AnyWidget {
    Row::new()
        .vertical_alignment(BoxAlignment::Center)
        .gaps(LayoutSpacing::horizontal(8.into()))
        .children([
            Container::new()
                .width(Dimension::Px(24.0))
                .height(Dimension::Px(24.0))
                .box_decoration(
                    BoxDecoration::new()
                        .background_color(color)
                        .border_radius(12),
                )
                .child(ZeroSizedBox)
                .boxed(),
            bar_px(color, 14.0, 110.0),
        ])
        .boxed()
}

/// Mirrors the card's footer: two counts on the left, one on the right.
fn footer_row(color: Color) -> AnyWidget {
    Row::new()
        .vertical_alignment(BoxAlignment::Center)
        .gaps(LayoutSpacing::horizontal(20.into()))
        .children([
            bar_px(color, 16.0, 44.0),
            bar_px(color, 16.0, 44.0),
            Expanded::new().box_child(SizedBox::new()),
            bar_px(color, 16.0, 44.0),
        ])
        .boxed()
}

/// The color of a placeholder `value` of the way through its pulse, `0.0` to `1.0`.
pub(crate) fn pulse_color(theme: ForumTheme, value: f32) -> Color {
    let tone = theme.primary_color().with_brightness(1.1);
    tone.with_alpha(0.12).lerp(tone.with_alpha(0.3), value)
}

/// A placeholder bar of a fixed width.
pub(crate) fn bar_px(color: Color, height: f32, width: f32) -> AnyWidget {
    Container::new()
        .height(Dimension::Px(height))
        .width(Dimension::Px(width))
        .box_decoration(
            BoxDecoration::new()
                .background_color(color)
                .border_radius(6),
        )
        .child(ZeroSizedBox)
        .boxed()
}

/// A placeholder bar that is `width_percent` of the width it is given.
pub(crate) fn bar_widget(color: Color, height: f32, width_percent: f32) -> impl Widget {
    Container::new()
        .height(Dimension::Px(height))
        .width(Dimension::Percent(width_percent))
        .box_decoration(
            BoxDecoration::new()
                .background_color(color)
                .border_radius(6),
        )
        .child(ZeroSizedBox)
}
