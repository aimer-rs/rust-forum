use crate::theme::ForumTheme;
use aimer::input::button::Button;
use aimer::style::{
    BorderSlice, BorderStyle, BoxBorder, BoxDecoration, LayoutSpacing, TextAlign, TextStyle,
};
use aimer::{
    Align, Alignment, AnyWidget, BoxAlignment, Color, Container, Dimension, Expanded, FontWeight,
    Row, StaticSvg, Svg, SvgDocument, SvgStyle, Text, Widget,
};

/// How tall a pill is: a comfortable target for a thumb.
pub const BUTTON_HEIGHT: f32 = 54.0;
const ICON_SIZE: u32 = 20;
/// The line round an outlined pill.
const OUTLINE_WIDTH: u32 = 2;

/// What a button's icon is drawn from.
#[derive(Clone, Copy)]
pub enum Glyph {
    Lucide(&'static StaticSvg),
    /// A brand mark Lucide doesn't have, embedded in the app as the text of an SVG.
    Embedded(&'static str),
}

/// The accent-coloured pill: the one thing on a page to press to go on.
pub fn filled_button(theme: ForumTheme, label: &str, on_press: impl Fn() + 'static) -> AnyWidget {
    pill(theme, true, None, label, on_press)
}

/// [`filled_button`] with an icon before its label.
pub fn filled_icon_button(
    theme: ForumTheme,
    glyph: Glyph,
    label: &str,
    on_press: impl Fn() + 'static,
) -> AnyWidget {
    pill(theme, true, Some(glyph), label, on_press)
}

/// A pill with a line round it and no fill, with an icon before its label when it
/// has one: an alternative way to go on.
pub fn outlined_button(
    theme: ForumTheme,
    glyph: Option<Glyph>,
    label: &str,
    on_press: impl Fn() + 'static,
) -> AnyWidget {
    pill(theme, false, glyph, label, on_press)
}

fn pill(
    theme: ForumTheme,
    filled: bool,
    glyph: Option<Glyph>,
    label: &str,
    on_press: impl Fn() + 'static,
) -> AnyWidget {
    let ink = if filled {
        theme.on_primary_color()
    } else {
        theme.on_background_color()
    };
    let shape = || BoxDecoration::new().border_radius(BUTTON_HEIGHT / 2.0);
    let (rest, hover, press) = if filled {
        (
            shape().background_color(theme.primary_color()),
            shape().background_color(theme.primary_color().darken(0.08)),
            shape().background_color(theme.primary_color().darken(0.18)),
        )
    } else {
        let outline = |fill: Color| {
            shape().background_color(fill).border(BoxBorder::all(
                BorderSlice::new()
                    .style(BorderStyle::Solid)
                    .stroke(OUTLINE_WIDTH)
                    .color(theme.on_background_color().with_alpha(0.18)),
            ))
        };
        (
            outline(Color::Transparent),
            outline(theme.primary_color().with_opacity(10)),
            outline(theme.primary_color().with_opacity(30)),
        )
    };

    let text = Text::new(label.to_string())
        .text_style(
            TextStyle::new()
                .font_weight(FontWeight::Bold)
                .font_size(17)
                .color(ink),
        )
        .boxed();
    let content = match glyph {
        Some(glyph) => Row::new()
            .vertical_alignment(BoxAlignment::Center)
            .gaps(LayoutSpacing::horizontal(10.into()))
            .children([glyph_icon(glyph, ink), text])
            .boxed(),
        None => text,
    };

    Button::new()
        .decoration(rest)
        .hover_decoration(hover)
        .press_decoration(press)
        .on_press(on_press)
        // A line takes room from inside the pill, so the outlined one is a
        // little shorter within it to be as tall as the filled one.
        .child(full_width(button_face(
            content,
            if filled {
                0.0
            } else {
                2.0 * OUTLINE_WIDTH as f32
            },
        )))
        .notype()
}

fn glyph_icon(glyph: Glyph, color: Color) -> AnyWidget {
    match glyph {
        Glyph::Lucide(icon) => Svg::new(icon)
            .style("path", SvgStyle::new().stroke(color))
            .width(ICON_SIZE)
            .height(ICON_SIZE)
            .boxed(),
        Glyph::Embedded(source) => {
            Svg::new(SvgDocument::from_svg(source).expect("the embedded icon is a valid SVG"))
                .style("path", SvgStyle::new().fill(color))
                .width(ICON_SIZE)
                .height(ICON_SIZE)
                .boxed()
        }
    }
}

/// A plain line of text that can be pressed: `color` is its colour.
pub fn text_button(
    label: &str,
    color: Color,
    size: u32,
    on_press: impl Fn() + 'static,
) -> AnyWidget {
    Button::new()
        .no_decor()
        .on_press(on_press)
        .child(
            Container::new().height(Dimension::Px(36.0)).child(
                Align::new().alignment(Alignment::MidCenter).child(
                    Text::new(label.to_string())
                        .text_align(TextAlign::MidCenter)
                        .text_style(TextStyle::new().font_size(size).color(color)),
                ),
            ),
        )
        .boxed()
}

/// Gives `widget` the whole width of the column it is in: a button is only as wide
/// as what is in it, unless a row hands it the room.
pub fn full_width(widget: impl Widget + 'static) -> AnyWidget {
    Container::new()
        .width(Dimension::Percent(100f32))
        .box_child(widget)
}

/// The room a button's contents are centred in, as wide as the button and
/// `inset` shorter than a pill.
fn button_face(content: AnyWidget, inset: f32) -> AnyWidget {
    Container::new()
        .height(Dimension::Px(BUTTON_HEIGHT - inset))
        .child(Align::new().alignment(Alignment::MidCenter).child(content))
        .boxed()
}
