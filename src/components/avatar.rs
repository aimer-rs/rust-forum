use crate::theme::ForumTheme;
use aimer::style::{BoxDecoration, BoxFit, TextStyle};
use aimer::{Align, Alignment, AnyWidget, Container, Dimension, NetworkImage, Text, Widget};

/// A round avatar `size` across; with no image to draw, a badge with the name's
/// initial.
pub fn round_avatar(theme: ForumTheme, url: &str, name: &str, size: f32) -> AnyWidget {
    let round = BoxDecoration::new().border_radius(size / 2.0);
    if !url.is_empty() {
        return Container::new()
            .width(Dimension::Px(size))
            .height(Dimension::Px(size))
            .box_decoration(round)
            .child(
                // `Contain`, not `Cover`: a square avatar fills the box either
                // way, and `Cover` would drop the container's rounded clip.
                NetworkImage::new(url.to_string())
                    .width(size)
                    .height(size)
                    .fit(BoxFit::Contain),
            )
            .boxed();
    }
    let initial = name
        .chars()
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_default();
    Container::new()
        .width(Dimension::Px(size))
        .height(Dimension::Px(size))
        .box_decoration(round.background_color(theme.primary_color()))
        .child(
            Align::new().alignment(Alignment::MidCenter).child(
                Text::new(initial).text_style(
                    TextStyle::new()
                        .font_size((size * 0.45) as u32)
                        .color(theme.on_primary_color()),
                ),
            ),
        )
        .boxed()
}
