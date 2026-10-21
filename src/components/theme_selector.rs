use std::time::Duration;

use crate::preferences;
use crate::store::AppStore;
use crate::theme::ForumTheme;
use aimer::animation::{ImplicitAnimatedBuilder, curves};
use aimer::style::{
    BorderSlice, BorderStyle, BoxBorder, BoxDecoration, LayoutSpacing, TextAlign, TextStyle, Theme,
    ThemeMode,
};
use aimer::{
    Align, Alignment, AnyWidget, BuildContext, Color, Container, Dimension, Expanded, FontWeight, ProviderContext,
    ProviderHandle, Row, StatelessWidget, Text, Widget,
};
use aimer::input::button::Button;

const BUTTON_HEIGHT: f32 = 40.0;
const TRANSITION_DURATION: Duration = Duration::from_millis(240);

/// A row of three theme-mode buttons — `Light`, `Dark`, and `System` — that share
/// the width they are given. The active mode uses the accent colour; the others
/// use the card decoration.
#[derive(Clone, StatelessWidget)]
pub struct ThemeSelector;

impl StatelessWidget for ThemeSelector {
    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme = *ForumTheme::of(ctx);
        let store = ProviderHandle::<AppStore>::of(ctx);
        let mode = ctx.select::<AppStore, ThemeMode>(|s| s.theme_mode);

        Row::new().gaps(LayoutSpacing::all(8)).children([
            Expanded::new().box_child(theme_button("Light", ThemeMode::Light, mode, theme, store.clone())),
            Expanded::new().box_child(theme_button("Dark", ThemeMode::Dark, mode, theme, store.clone())),
            Expanded::new().box_child(theme_button("System", ThemeMode::System, mode, theme, store)),
        ])
    }
}

/// One animated theme-mode button.
fn theme_button(
    label: &'static str,
    this_mode: ThemeMode,
    current: ThemeMode,
    theme: ForumTheme,
    store: ProviderHandle<AppStore>,
) -> AnyWidget {
    let active = current == this_mode;

    let active_bg = theme.primary_color();
    let inactive_bg = theme.surface_color();
    let active_fg = theme.on_primary_color();
    let inactive_fg = theme.on_surface_color();
    let inactive_border = theme.primary_color().darken(0.4).with_alpha(0.1);
    let active_border = Color::Transparent;

    Button::new()
        .on_press(move || preferences::set_theme_mode(&store, this_mode))
        .child(
            ImplicitAnimatedBuilder::new(
                if active { 1.0 } else { 0.0 },
                TRANSITION_DURATION,
                curves::FAST_OUT_SLOW_IN,
                move |progress| {
                    let progress = *progress;
                    let bg = inactive_bg.lerp(active_bg, progress);
                    let fg = inactive_fg.lerp(active_fg, progress);
                    let border = inactive_border.lerp(active_border, progress);

                    Container::new()
                        .width(Dimension::Percent(100.0))
                        .height(Dimension::Px(BUTTON_HEIGHT))
                        .box_decoration(
                            BoxDecoration::new()
                                .background_color(bg)
                                .border_radius(8)
                                .border(BoxBorder::all(
                                    BorderSlice::new()
                                        .style(BorderStyle::Dotted)
                                        .stroke(2)
                                        .color(border),
                                )),
                        )
                        .child(Align::new().alignment(Alignment::MidCenter).child(
                            Text::new(label)
                                .text_align(TextAlign::MidCenter)
                                .text_style(
                                    TextStyle::new()
                                        .font_size(16)
                                        .font_weight(FontWeight::Bold)
                                        .color(fg),
                                ),
                        ))
                },
            )
            .key(format!("theme-button-{label}")),
        )
        .boxed()
}
