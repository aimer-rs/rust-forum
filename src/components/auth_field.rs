use crate::components::caret::{blink, caret, restart};
use crate::theme::ForumTheme;
use aimer::animation::AnimationController;
use aimer::input::TextEditingController;
use aimer::input::input::{InputType, TextField};
use aimer::style::{
    BorderSlice, BorderStyle, BoxBorder, BoxDecoration, LayoutSpacing, TextStyle, Theme,
};
use aimer::{
    BoxAlignment, BuildContext, Container, Dimension, Expanded, Row, State, StateUpdater,
    StatefulWidget, StaticSvg, Svg, SvgStyle, Widget,
};
use std::rc::Rc;

/// How tall a field of the sign-in form is: a comfortable target for a finger.
const HEIGHT: f32 = 54.0;
const ICON_SIZE: u32 = 18;
const RADIUS: f32 = HEIGHT / 2.0;
/// The line round the field, lit in the accent colour while it has the keyboard.
const BORDER_WIDTH: u32 = 2;
/// The height of a 16 point line of text. The field draws its text from the top
/// of the room it has and clips it to the room that is left once it is padded, so
/// it must be left a whole line.
const LINE_HEIGHT: f32 = 20.0;
/// The room above and below the line: the line is centred in what the border
/// leaves of the box, which is why the border is taken off its height.
const TEXT_PADDING: u32 = ((HEIGHT - 2.0 * BORDER_WIDTH as f32 - LINE_HEIGHT) / 2.0) as u32;

/// One field of the sign-in form: a rounded box with an icon and the text being
/// typed, whose line takes the accent colour while it has the keyboard.
#[derive(Clone, StatefulWidget)]
pub struct AuthField {
    controller: TextEditingController,
    icon: &'static StaticSvg,
    hint: &'static str,
    input_type: InputType,
    max_length: Option<usize>,
    on_submitted: Option<Rc<dyn Fn()>>,
}

impl AuthField {
    pub fn new(
        controller: TextEditingController,
        icon: &'static StaticSvg,
        hint: &'static str,
    ) -> Self {
        Self {
            controller,
            icon,
            hint,
            input_type: InputType::Text,
            max_length: None,
            on_submitted: None,
        }
    }

    pub fn input_type(mut self, input_type: InputType) -> Self {
        self.input_type = input_type;
        self
    }

    pub fn max_length(mut self, max_length: usize) -> Self {
        self.max_length = Some(max_length);
        self
    }

    /// Runs when the user presses enter in the field.
    pub fn on_submitted(mut self, on_submitted: impl Fn() + 'static) -> Self {
        self.on_submitted = Some(Rc::new(on_submitted));
        self
    }
}

pub struct AuthFieldState {
    parent: AuthField,
    focused: bool,
    /// Runs while the field has focus; see [`caret`].
    blink: AnimationController,
    updater: StateUpdater<Self>,
}

impl StatefulWidget for AuthField {
    type State = AuthFieldState;

    fn create_state(self) -> Self::State {
        AuthFieldState { parent: self, focused: false, blink: blink(), updater: StateUpdater::new() }
    }
}

impl State<AuthField> for AuthFieldState {
    fn init_state(&mut self, updater: StateUpdater<Self>) {
        self.updater = updater;
    }

    fn adopt_config_from(&mut self, new: Self) {
        self.parent = new.parent;
    }

    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme = *ForumTheme::of(ctx);
        let field = &self.parent;
        let muted = theme.on_background_color().with_opacity(140);
        let line = if self.focused {
            theme.primary_color()
        } else {
            theme.on_background_color().with_alpha(0.16)
        };
        let (focus_in, focus_out) = (self.updater, self.updater);
        let on_submitted = field.on_submitted.clone();
        let (blink_caret, blink_on_edit, blink_on_focus, blink_on_blur) =
            (self.blink.clone(), self.blink.clone(), self.blink.clone(), self.blink.clone());

        let mut text = TextField::new()
            .controller(field.controller.clone())
            .input_type(field.input_type)
            .hint(field.hint)
            .hint_style(TextStyle::new().font_size(16).color(muted))
            .text_style(TextStyle::new().font_size(16).color(theme.on_background_color()))
            .selection_color(theme.primary_color().with_alpha(0.28))
            .decoration(BoxDecoration::new())
            .padding(LayoutSpacing::vertical(TEXT_PADDING))
            .caret(move |_| caret(theme, blink_caret.clone()))
            .on_submitted(move |_: String| {
                if let Some(on_submitted) = &on_submitted {
                    on_submitted();
                }
            })
            .on_changed(move |_: String| restart(&blink_on_edit))
            .on_focus(move |_: String| {
                restart(&blink_on_focus);
                focus_in.set_state(|state| state.focused = true)
            })
            .on_blur(move |_: String| {
                blink_on_blur.reset();
                focus_out.set_state(|state| state.focused = false)
            });
        if let Some(max_length) = field.max_length {
            text = text.max_length(Some(max_length));
        }

        Row::new().children([Expanded::new().box_child(Container::new()
            .height(Dimension::Px(HEIGHT))
            .padding(LayoutSpacing::horizontal(20.into()))
            .box_decoration(
                BoxDecoration::new()
                    .background_color(theme.surface_color())
                    .border_radius(RADIUS)
                    .border(BoxBorder::all(
                        BorderSlice::new()
                            .style(BorderStyle::Solid)
                            .stroke(BORDER_WIDTH)
                            .color(line),
                    )),
            )
            .child(
                Row::new()
                    .vertical_alignment(BoxAlignment::Center)
                    .gaps(LayoutSpacing::horizontal(10.into()))
                    .children([
                        Svg::new(field.icon)
                            .style(
                                "path",
                                SvgStyle::new().stroke(if self.focused {
                                    theme.primary_color()
                                } else {
                                    muted
                                }),
                            )
                            .width(ICON_SIZE)
                            .height(ICON_SIZE)
                            .boxed(),
                        Expanded::new().box_child(text),
                    ]),
            ))])
    }
}
