use crate::theme::ForumTheme;
use aimer::animation::{AnimatedBuilder, AnimationController, curves};
use aimer::style::BoxDecoration;
use aimer::{Container, Dimension, Widget, ZeroSizedBox};
use std::time::Duration;

/// The caret's thickness: wider than the hairline a field draws by default.
const CARET_WIDTH: f32 = 2.5;
/// A full on-off cycle of the caret: shown for half of it and hidden for half.
const CARET_PERIOD: Duration = Duration::from_millis(1000);

/// The timeline a field's [`caret`] blinks on: it repeats, and does not start
/// until [`restart`] is called.
pub fn blink() -> AnimationController {
    let blink = AnimationController::new(CARET_PERIOD, curves::LINEAR);
    blink.set_repeat(true);
    blink
}

/// The caret: a rounded bar of the accent colour that blinks on `blink`.
///
/// A field's own caret can only be one of a few named colours, and its blink is
/// driven from the field's repaints and stops after its first turn, so the caret
/// is drawn and blinked from here instead: `blink` runs while the field has
/// focus, and restarts with every key so that the caret stays solid while
/// typing. The host gives the caret no size while the field has no focus, so
/// there is nothing to hide in between.
pub fn caret(theme: ForumTheme, blink: AnimationController) -> impl Widget {
    AnimatedBuilder::new(blink, move |turn| {
        let shown = turn < 0.5;
        Container::new()
            .width(Dimension::Px(CARET_WIDTH))
            .box_decoration(
                BoxDecoration::new()
                    .background_color(theme.primary_color().with_alpha(if shown { 1.0 } else { 0.0 }))
                    .border_radius(CARET_WIDTH / 2.0),
            )
            .child(ZeroSizedBox)
    })
}

/// Starts the caret's blink over, so that it is shown at once and for a whole
/// half turn.
pub fn restart(blink: &AnimationController) {
    blink.reset();
    blink.forward_from_first_tick();
}
