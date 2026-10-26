use aimer::animation::{AnimatedBuilder, AnimationController};
use aimer::decoration::{BackdropFilter, BorderRadius, Decoration, DecorationBox};
use aimer::{
    AnyWidget, BuildContext, Color, Dimension, Opacity, Scalable, SizedBox, Stack, StatelessWidget, Widget,
};
use std::rc::Rc;

/// How large a button is when it starts to appear, and as it finishes leaving.
const SCALE_FROM: f32 = 1.4;
/// How much the backdrop behind a button is blurred, in logical pixels, when it
/// starts to appear and as it finishes leaving.
const BLUR_FROM: f32 = 60.0;
/// Below this the blur is not worth a pass of its own.
const BLUR_FLOOR: f32 = 0.5;

/// Brings a button in: it starts large (1.4×), faded out and over a heavily
/// blurred backdrop, and settles to its own size, fully shown and over a sharp
/// backdrop. Running the controller backwards takes it out the same way.
///
/// aimer can blur what is behind a widget but not the widget itself, so it is the
/// backdrop under the button that blurs, not the button.
///
/// The button is built once and kept, and only the motion around it is rebuilt on
/// each frame, so the cost of a frame does not grow with the button. It is still
/// given as a function: the function runs when this widget is built, so a new
/// button, such as a tab bar with another tab selected, replaces the old one.
#[derive(Clone, StatelessWidget)]
pub struct ButtonReveal {
    controller: AnimationController,
    width: f32,
    height: f32,
    /// Corner radius of the backdrop that blurs under the button.
    radius: f32,
    /// How large the button is when it starts to appear.
    scale_from: f32,
    button: Rc<dyn Fn() -> AnyWidget>,
}

impl ButtonReveal {
    pub fn new(
        controller: AnimationController,
        width: f32,
        height: f32,
        radius: f32,
        button: impl Fn() -> AnyWidget + 'static,
    ) -> Self {
        Self {
            controller,
            width,
            height,
            radius,
            scale_from: SCALE_FROM,
            button: Rc::new(button),
        }
    }

    /// Starts the button at `scale` times its size instead of [`SCALE_FROM`]: a
    /// button as wide as the window has no room to start at 1.4×.
    pub fn scale_from(mut self, scale: f32) -> Self {
        self.scale_from = scale;
        self
    }
}

/// One frame of the reveal at `progress` (`0.0` out, `1.0` in): `content` drawn
/// over a backdrop of the size of `slot`, large, faded and blurred at the start.
fn revealed(progress: f32, scale_from: f32, radius: f32, slot: SizedBox, content: AnyWidget) -> impl Widget {
    let blur = BLUR_FROM * (1.0 - progress);
    let backdrop = if blur > BLUR_FLOOR {
        BackdropFilter::new().blur_radius(blur).notype()
    } else {
        Color::Transparent.notype()
    };
    Opacity::new().opacity(progress).child(
        Scalable::new()
            .scale(scale_from + (1.0 - scale_from) * progress)
            .child(Stack::new().children([
                DecorationBox::new()
                    .decoration([BorderRadius::from(radius).notype(), backdrop])
                    .box_child(slot),
                content,
            ])),
    )
}

impl StatelessWidget for ButtonReveal {
    fn build(&self, _ctx: &BuildContext) -> impl Widget {
        let (width, height, radius, scale_from) = (self.width, self.height, self.radius, self.scale_from);

        // The slot keeps the button's own size, so the row around it does not move
        // while the button, drawn larger, does.
        SizedBox::new()
            .width(width)
            .height(height)
            .child(AnimatedBuilder::with_child(
                self.controller.clone(),
                (self.button)(),
                move |progress, button| {
                    revealed(progress, scale_from, radius, SizedBox::new().width(width).height(height), button.boxed())
                },
            ))
    }
}

/// [`ButtonReveal`] for a field that takes the width it is given: the same
/// motion, with the backdrop as wide as the field is.
///
/// Like the button, the field is given as a function that runs when this widget
/// is built; what it builds is kept for the whole motion, so the text typed and
/// the focus are never disturbed by it.
#[derive(Clone, StatelessWidget)]
pub struct FieldReveal {
    controller: AnimationController,
    height: f32,
    /// Corner radius of the backdrop that blurs under the field.
    radius: f32,
    /// How large the field is when it starts to appear.
    scale_from: f32,
    field: Rc<dyn Fn() -> AnyWidget>,
}

impl FieldReveal {
    pub fn new(
        controller: AnimationController,
        height: f32,
        radius: f32,
        field: impl Fn() -> AnyWidget + 'static,
    ) -> Self {
        Self {
            controller,
            height,
            radius,
            scale_from: SCALE_FROM,
            field: Rc::new(field),
        }
    }

    /// Starts the field at `scale` times its size instead of [`SCALE_FROM`]: a
    /// field that spans the window has no room to start at 1.4×.
    pub fn scale_from(mut self, scale: f32) -> Self {
        self.scale_from = scale;
        self
    }
}

impl StatelessWidget for FieldReveal {
    fn build(&self, _ctx: &BuildContext) -> impl Widget {
        let (height, radius, scale_from) = (self.height, self.radius, self.scale_from);

        AnimatedBuilder::with_child(self.controller.clone(), (self.field)(), move |progress, field| {
            revealed(
                progress,
                scale_from,
                radius,
                SizedBox::new().width(Dimension::Percent(100.0)).height(height),
                field.boxed(),
            )
        })
    }
}
