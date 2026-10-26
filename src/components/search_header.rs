use crate::app_shell::is_mobile;
use crate::components::button_reveal::{ButtonReveal, FieldReveal};
use crate::components::caret::{blink, caret, restart};
use crate::components::side_bar::glass;
use crate::models::search::SearchMode;
use crate::store::AppStore;
use crate::theme::ForumTheme;
use aimer::screen::MediaQuery;
use aimer::animation::{AnimationController, ImplicitAnimatedBuilder, curves};
use aimer::FocusNode;
use aimer::input::TextEditingController;
use aimer::indicator::{TabBar, TabItem, indicator_styles};
use aimer::input::input::{InputType, TextField};
use aimer::style::{BorderSlice, BorderStyle, BoxBorder, BoxDecoration, LayoutSpacing, TextStyle, Theme};
use aimer::{
    AnyWidget, BoxAlignment, BuildContext, Column, Container, Dimension, Expanded,
    FontWeight, Icons, ProviderContext, ProviderHandle, Row, Scalable, SizedBox, Stack, State, StateUpdater, StatefulWidget, Svg,
    SvgStyle, Text, Widget, ZeroSizedBox,
};
use std::time::Duration;

/// Height of the search field and of the toggle beside it, matching the buttons
/// of the bar they sit in.
const HEIGHT: f32 = 40.0;
const ICON_SIZE: u32 = 18;
/// The toggle is a little shorter than the field beside it.
const TOGGLE_HEIGHT: f32 = HEIGHT - 4.0;
/// What the toggle pads its segments by on every side.
const TOGGLE_PADDING: f32 = 4.0;
const TOGGLE_ITEM_WIDTH: f32 = 60.0;
/// The space between the two rows of the header on a narrow window.
const ROW_GAP: f32 = 8.0;
/// The space the bar leaves at each side of the window.
const BAR_MARGIN: f32 = 10.0;
/// How much taller the field is on a narrow window, where it is the touch target
/// of the page, than the buttons it sits among.
const MOBILE_FIELD_EXTRA: f32 = 5.0;
/// How much taller the header is on a narrow window, where the toggle has a row
/// of its own and the field is a little taller, than on a wide one.
pub const STACKED_EXTRA: f32 = TOGGLE_HEIGHT + MOBILE_FIELD_EXTRA + ROW_GAP;
/// How much larger the field is, at most, while it has focus.
const FOCUS_SCALE: f32 = 0.03;
/// How long the field takes to come into focus and to leave it.
const FOCUS_DURATION: Duration = Duration::from_millis(280);
/// The thickness of the focus ring around the field.
const RING_WIDTH: f32 = 2.0;
/// The space above and below the text in the field, in logical pixels: half of
/// what a 16 point line leaves of the pill, which is about 19 points shorter
/// than the pill's 40.
/// How much of the room beside the toggle the field takes: 30% narrower than it
/// would fill.
const FIELD_WIDTH_SHARE: f32 = 0.7;
/// What is left of the room, split between the two sides of the field.
const FIELD_MARGIN_SHARE: f32 = (1.0 - FIELD_WIDTH_SHARE) / 2.0;
/// The room above and below a line of 16 point text that the field's own height
/// does not hold: what is left of the height once the line is taken, halved, is
/// the room the text is padded by.
const FIELD_LINE_HEIGHT: f32 = 20.0;

/// The search field and the Blog / User toggle, as the bar over the search page
/// shows them.
///
/// What is typed and which mode is picked go to the store, where the page reads
/// them: the bar belongs to the shell and the results to the page. The field is
/// filled from the store when it is built, so coming back to the search from a
/// post finds the words as they were left.
///
/// The field and the toggle each come in and go out on a controller of their own.
/// The toggle's is not the field's: the shell holds it back until the buttons of
/// the page before have gone out, so that the two never share the end of the bar.
#[derive(Clone, StatefulWidget)]
pub struct SearchHeader {
    initial: String,
    /// Drives the field: `0.0` out, `1.0` in.
    reveal: AnimationController,
    /// Drives the toggle: `0.0` out, `1.0` in.
    toggle: AnimationController,
    /// Whether the field takes the keyboard as it arrives. Coming back from a page
    /// opened from the results it does not: the user is there to read them.
    auto_focus: bool,
}

impl SearchHeader {
    pub fn new(
        initial: String,
        reveal: AnimationController,
        toggle: AnimationController,
        auto_focus: bool,
    ) -> Self {
        Self { initial, reveal, toggle, auto_focus }
    }
}

pub struct SearchHeaderState {
    /// Whether the field takes the keyboard as it arrives; see [`SearchHeader`].
    auto_focus: bool,
    /// Lets the field give up the keyboard when the user submits.
    focus_node: FocusNode,
    reveal: AnimationController,
    toggle: AnimationController,
    controller: TextEditingController,
    /// Whether the field has the keyboard, which lifts it; see [`FOCUS_SCALE`].
    focused: bool,
    /// Runs while the field has focus; the caret is shown for the first half of
    /// each turn and hidden for the second. See [`caret`].
    blink: AnimationController,
    updater: StateUpdater<Self>,
}

#[cfg(test)]
thread_local! {
    /// The focus node of the field most recently built, for tests to ask who has
    /// the keyboard.
    pub(crate) static LAST_FIELD_NODE: std::cell::RefCell<Option<FocusNode>> = const { std::cell::RefCell::new(None) };
}

impl StatefulWidget for SearchHeader {
    type State = SearchHeaderState;

    fn create_state(self) -> Self::State {
        let focus_node = FocusNode::new();
        #[cfg(test)]
        LAST_FIELD_NODE.with(|last| *last.borrow_mut() = Some(focus_node.clone()));
        SearchHeaderState {
            auto_focus: self.auto_focus,
            focus_node,
            reveal: self.reveal,
            toggle: self.toggle,
            controller: TextEditingController::with_text(self.initial),
            focused: false,
            blink: blink(),
            updater: StateUpdater::new(),
        }
    }
}

impl State<SearchHeader> for SearchHeaderState {
    fn init_state(&mut self, updater: StateUpdater<Self>) {
        self.updater = updater;
    }

    fn adopt_config_from(&mut self, new: Self) {
        self.reveal = new.reveal;
        self.toggle = new.toggle;
    }

    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme = *ForumTheme::of(ctx);
        let store = ProviderHandle::<AppStore>::of(ctx);
        let mode = ctx.select::<AppStore, SearchMode>(|s| s.search_mode);
        let controller = self.controller.clone();
        let updater = self.updater.clone();
        let blink = self.blink.clone();
        let field_store = store.clone();

        // The field comes in as the buttons do: large, faded and over a blurred
        // backdrop, settling to its size, and leaves the same way. A little less
        // large, as it is wide: at the buttons' 1.4× it would leave the window.
        let focused = self.focused;
        let focus_node = self.focus_node.clone();
        let auto_focus = self.auto_focus;
        // A little taller on a narrow window, where it is pressed with a finger.
        let height = HEIGHT + if is_mobile(ctx) { MOBILE_FIELD_EXTRA } else { 0.0 };
        let field = FieldReveal::new(self.reveal.clone(), height, height / 2.0, move || {
            field_with_focus(
                theme,
                height,
                controller.clone(),
                field_store.clone(),
                updater.clone(),
                blink.clone(),
                focus_node.clone(),
                auto_focus,
                focused,
            )
        })
        .scale_from(FIELD_SCALE_FROM);

        if is_mobile(ctx) {
            // On a narrow window the field has the whole width, and the toggle
            // sits centred on a row of its own under it.
            return Column::new()
                .gaps(LayoutSpacing::vertical((ROW_GAP / 2.0) as u32))
                .children([
                    // Left to itself the field would take all the height there is.
                    SizedBox::new().height(Dimension::Px(height)).box_child(field).boxed(),
                    revealed_toggle(theme, mode, store, self.toggle.clone(), mobile_toggle_item_width(ctx), MOBILE_TOGGLE_SCALE),
                ])
                .boxed();
        }

        Row::new()
            .vertical_alignment(BoxAlignment::Center)
            .gaps(LayoutSpacing::horizontal(8.into()))
            .children([
                // The field is a share of the room the toggle leaves, centred in it
                // by an equal share of space on each side.
                Expanded::new().box_child(
                    Row::new().children([
                        Expanded::new().flex(FIELD_MARGIN_SHARE).box_child(SizedBox::new()),
                        Expanded::new().flex(FIELD_WIDTH_SHARE).box_child(field),
                        Expanded::new().flex(FIELD_MARGIN_SHARE).box_child(SizedBox::new()),
                    ]),
                ),
                revealed_toggle(theme, mode, store, self.toggle.clone(), TOGGLE_ITEM_WIDTH, TOGGLE_SCALE),
            ])
            .boxed()
    }
}

/// How large the field is when it starts to appear.
const FIELD_SCALE_FROM: f32 = 1.08;

/// The field as it is with or without focus.
///
/// Focus lifts the field like the search of Apple's apps: it swells a little and a
/// ring of the accent colour draws around it, both easing in together and out
/// again on blur.
fn field_with_focus(
    theme: ForumTheme,
    height: f32,
    controller: TextEditingController,
    store: ProviderHandle<AppStore>,
    updater: StateUpdater<SearchHeaderState>,
    blink: AnimationController,
    focus_node: FocusNode,
    auto_focus: bool,
    focused: bool,
) -> AnyWidget {
    ImplicitAnimatedBuilder::new(
        if focused { 1.0 } else { 0.0 },
        FOCUS_DURATION,
        curves::FAST_OUT_SLOW_IN,
        move |focus| {
            Scalable::new().scale(1.0 + FOCUS_SCALE * *focus).child(search_field(
                theme,
                height,
                controller.clone(),
                store.clone(),
                updater.clone(),
                blink.clone(),
                focus_node.clone(),
                auto_focus,
                *focus,
            ))
        },
    )
    .boxed()
}

/// The pill with a magnifier and the text being typed, `focus` (0 to 1) of the
/// way to its focused look.
fn search_field(
    theme: ForumTheme,
    height: f32,
    controller: TextEditingController,
    store: ProviderHandle<AppStore>,
    updater: StateUpdater<SearchHeaderState>,
    blink: AnimationController,
    focus_node: FocusNode,
    auto_focus: bool,
    focus: f32,
) -> AnyWidget {
    let muted = theme.on_background_color().with_opacity(140);
    let icon = Svg::new(Icons::SEARCH)
        .style(
            "path",
            SvgStyle::new().stroke(muted.lerp(theme.primary_color(), focus)),
        )
        .width(ICON_SIZE)
        .height(ICON_SIZE);
    let (focus_in, focus_out) = (updater.clone(), updater);
    let (blink_on_focus, blink_on_blur, blink_on_edit, blink_caret) =
        (blink.clone(), blink.clone(), blink.clone(), blink);

    // The glass is as tall as the toggle's. The ring is drawn over its rim, always
    // as thick, and only its colour comes and goes, so nothing moves when it does.
    let ring = Container::new()
        .width(Dimension::Percent(100.0))
        .height(Dimension::Px(height))
        .box_decoration(
            BoxDecoration::new()
                .border(BoxBorder::all(
                    BorderSlice::new()
                        .style(BorderStyle::Solid)
                        .stroke(RING_WIDTH as u32)
                        .color(theme.primary_color().with_alpha(focus)),
                ))
                .border_radius(height / 2.0),
        )
        .child(ZeroSizedBox);

    let pill = Container::new()
                .width(Dimension::Percent(100.0))
                .height(Dimension::Px(height))
                .padding(LayoutSpacing::horizontal(12.into()))
                .decoration(glass(theme))
                .child(
                    // The row is as tall as the pill, and centres what is in it.
                    Row::new()
                        .vertical_alignment(BoxAlignment::Center)
                        .gaps(LayoutSpacing::horizontal(8.into()))
                        .children([
                            icon.boxed(),
                            Expanded::new().box_child(
                                TextField::new()
                                    .controller(controller)
                                    .input_type(InputType::Text)
                                    .hint("Search")
                                    .hint_style(TextStyle::new().font_size(16).color(muted))
                                    .text_style(
                                        TextStyle::new()
                                            .font_size(16)
                                            .color(theme.on_background_color()),
                                    )
                                    .selection_color(theme.primary_color().with_alpha(0.28))
                                    .caret(move |_| caret(theme, blink_caret.clone()))
                                    .decoration(BoxDecoration::new())
                                    // A field draws its text from the top of the room it
                                    // has, so the room above is what centres it: what is
                                    // left of the pill's height once a line of 16 point
                                    // text is taken, halved.
                                    .padding(LayoutSpacing::vertical(((height - FIELD_LINE_HEIGHT) / 2.0) as u32))
                                    .auto_focus(auto_focus)
                                    .focus_node(focus_node.clone())
                                    // Enter is the user saying they are done typing: the
                                    // keyboard goes away, and the results are what is left.
                                    .on_submitted(move |_: String| focus_node.unfocus())
                                    .on_changed(move |text: String| {
                                        restart(&blink_on_edit);
                                        store.update(|s| s.search_term = text.clone());
                                    })
                                    .on_focus(move |_: String| {
                                        restart(&blink_on_focus);
                                        focus_in.set_state(|state| state.focused = true)
                                    })
                                    .on_blur(move |_: String| {
                                        blink_on_blur.reset();
                                        focus_out.set_state(|state| state.focused = false)
                                    }),
                            ),
                        ]),
                );

    Stack::new().children([pill.boxed(), ring.boxed()]).boxed()
}

/// How wide each half of the toggle is when it spans the window: the width left
/// by the bar's own margins and the toggle's padding, shared between the two.
fn mobile_toggle_item_width(ctx: &BuildContext) -> f32 {
    let window = MediaQuery::select(ctx, |media| media.size.width);
    (window - BAR_MARGIN * 2.0 - TOGGLE_PADDING * 2.0) / SearchMode::ALL.len() as f32
}

/// How large the toggle is when it starts to appear: the usual 1.4× for the small
/// one, and little more than its size for the one as wide as the window.
const TOGGLE_SCALE: f32 = 1.4;
const MOBILE_TOGGLE_SCALE: f32 = 1.06;

/// [`mode_toggle`], coming in and going out on `controller`. It keeps its place
/// when out, so the field beside it doesn't move when it arrives.
fn revealed_toggle(
    theme: ForumTheme,
    mode: SearchMode,
    store: ProviderHandle<AppStore>,
    controller: AnimationController,
    item_width: f32,
    scale_from: f32,
) -> AnyWidget {
    let width = item_width * SearchMode::ALL.len() as f32 + TOGGLE_PADDING * 2.0;
    ButtonReveal::new(controller, width, TOGGLE_HEIGHT, TOGGLE_HEIGHT / 2.0, move || {
        mode_toggle(theme, mode, store.clone(), item_width)
    })
    .scale_from(scale_from)
    .boxed()
}

/// The two-way switch between searching the blog and searching people.
///
/// It is the bar of the mobile navigation, with the same frosted glass and the
/// same liquid glass indicator that slides to the chosen segment and lifts into
/// a lens while it moves, holding a label per mode.
fn mode_toggle(
    theme: ForumTheme,
    mode: SearchMode,
    store: ProviderHandle<AppStore>,
    item_width: f32,
) -> AnyWidget {
    TabBar::new()
        .selection(mode.index())
        .on_select(move |index| {
            let mode = SearchMode::from_index(index);
            store.update(|s| s.search_mode = mode);
        })
        .item_size(item_width, TOGGLE_HEIGHT - TOGGLE_PADDING * 2.0)
        .bar_padding(LayoutSpacing::all(TOGGLE_PADDING as u32))
        .bar_decoration(glass(theme))
        .inactive_color(theme.on_surface_color())
        .selected_color(theme.primary_color())
        .style(indicator_styles::LIQUID_GLASS.rest_fill(theme.primary_color().with_alpha(0.2)))
        .items(SearchMode::ALL.iter().map(|mode| {
            let label = mode.label();
            // The label goes in as the item's icon, which is handed the tint the
            // item is drawn in, so that it is bigger than a tab's own caption.
            TabItem::new().icon(move |tint| {
                Text::new(label).text_style(
                    TextStyle::new().font_size(15).font_weight(FontWeight::Bold).color(tint),
                )
            })
        }))
        .boxed()
}
