//! The pieces the settings pages are made of: a page with a title, headings that
//! start a group, and the rows a group holds.

use crate::app_shell::{MOBILE_TOP, is_mobile};
use crate::components::page_title::PageTitle;
use crate::theme::ForumTheme;
use aimer::input::button::Button;
use aimer::share::ShareRef;
use aimer::style::{BoxDecoration, LayoutSpacing, TextOverflow, TextStyle};
use aimer::{
    Align, Alignment, AnyWidget, BoxAlignment, BuildContext, Color, Column, Container, Dimension,
    Expanded, FontWeight, Row, Scrollable, SizedBox, StaticSvg, Svg, SvgStyle, Text, Widget, ZeroSizedBox, case,
};

/// The room at each side of a page.
pub const PAGE_MARGIN: f32 = 8.0;
const ICON_SIZE: u32 = 24;
/// The room between a row's edge and what is in it: a pressed row is lit a little
/// wider than its contents.
pub const ROW_INSET: f32 = 12.0;
/// Where everything but the rows starts from the edge of the window.
pub const CONTENT_MARGIN: f32 = PAGE_MARGIN + ROW_INSET;
const ROW_RADIUS: f32 = 14.0;
const SWITCH_WIDTH: f32 = 52.0;
const SWITCH_HEIGHT: f32 = 30.0;
const SWITCH_KNOB: f32 = 24.0;
/// The room under the last group: the tab bar floats over it on the pages that
/// have one.
const BOTTOM_MARGIN: f32 = 130.0;

/// A page of the settings: the title under the bar over the page, and the groups
/// under it, scrolling.
pub fn page(
    ctx: &BuildContext,
    theme: ForumTheme,
    key: &'static str,
    title: &'static str,
    body: Vec<AnyWidget>,
) -> AnyWidget {
    let mut content: Vec<AnyWidget> = vec![
        // Clear of the bar over the page, with its back button.
        SizedBox::new().height(case!(is_mobile(ctx), MOBILE_TOP) + 56).boxed(),
        inset(PageTitle::new(ShareRef::from_static(title)).padding(LayoutSpacing::new()).boxed()),
        SizedBox::new().height(8).boxed(),
    ];
    content.extend(body);
    content.push(SizedBox::new().height(BOTTOM_MARGIN).boxed());

    Container::new()
        .color(theme.background_color())
        .child(
            Scrollable::new().key(key).box_child(
                Container::new()
                    .padding(LayoutSpacing::horizontal((PAGE_MARGIN as u32).into()))
                    .child(Column::new().children(content)),
            ),
        )
        .boxed()
}

/// `widget` lined up with what is in the rows, rather than with their edge.
pub fn inset(widget: AnyWidget) -> AnyWidget {
    Container::new()
        .padding(LayoutSpacing::horizontal((ROW_INSET as u32).into()))
        .child(widget)
        .boxed()
}

/// The name of a group, above it: small capitals in the muted colour.
pub fn heading(theme: ForumTheme, label: &str) -> AnyWidget {
    Container::new()
        .padding(LayoutSpacing::new().top(30).bottom(6).left(ROW_INSET as u32))
        .child(
            Text::new(label.to_uppercase()).text_style(
                TextStyle::new()
                    .font_weight(FontWeight::Bold)
                    .font_size(14)
                    .color(muted(theme)),
            ),
        )
        .boxed()
}

/// A line of explanation under a group.
pub fn footnote(theme: ForumTheme, text: &'static str) -> AnyWidget {
    Container::new()
        .padding(LayoutSpacing::new().top(8).left(ROW_INSET as u32).right(ROW_INSET as u32))
        .child(
            Text::new(text).text_style(
                TextStyle::new().text_overflow(TextOverflow::Wrap).font_size(13).color(muted(theme)),
            ),
        )
        .boxed()
}

/// The rows of a group, one under the other: the page shows through between
/// them, with no card or lines to hold them.
pub fn group(rows: Vec<AnyWidget>) -> AnyWidget {
    Column::new().children(rows).boxed()
}

/// What a row ends in.
pub enum Trailing {
    Nothing,
    /// An arrow: the row opens a page.
    Chevron,
    /// A switch, on or off.
    Switch(bool),
    /// A lock: the choice is on and cannot be changed.
    Locked,
}

/// What the icon of a row is.
pub enum Tile {
    Lucide(&'static StaticSvg),
    /// Anything else, drawn in the tint it is given and the size of an icon.
    Custom(Box<dyn Fn(Color) -> AnyWidget>),
}

/// One row of a group: an icon, a title, a line under it when there is one to say,
/// the value it holds and how it ends. It can be pressed when `on_press` is given.
pub struct SettingsRow {
    pub tile: Tile,
    pub title: String,
    pub detail: Option<String>,
    /// What the row is set to, said at its end: the theme, say.
    pub value: Option<String>,
    pub trailing: Trailing,
    /// The title is drawn in the accent colour, as for an action.
    pub accent: bool,
    pub on_press: Option<Box<dyn Fn() + 'static>>,
}

impl SettingsRow {
    pub fn new(tile: Tile, title: impl Into<String>) -> Self {
        Self {
            tile,
            title: title.into(),
            detail: None,
            value: None,
            trailing: Trailing::Nothing,
            accent: false,
            on_press: None,
        }
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    pub fn trailing(mut self, trailing: Trailing) -> Self {
        self.trailing = trailing;
        self
    }

    pub fn accent(mut self) -> Self {
        self.accent = true;
        self
    }

    pub fn on_press(mut self, on_press: impl Fn() + 'static) -> Self {
        self.on_press = Some(Box::new(on_press));
        self
    }

    pub fn build(self, theme: ForumTheme) -> AnyWidget {
        let ink = if self.accent { theme.primary_color() } else { theme.on_background_color() };
        let icon_tint = if self.accent { theme.primary_color() } else { theme.on_background_color() };
        let icon = Container::new()
            .width(Dimension::Px(ICON_SIZE as f32))
            .height(Dimension::Px(ICON_SIZE as f32))
            .child(match self.tile {
                Tile::Lucide(icon) => lucide(icon, icon_tint),
                Tile::Custom(draw) => draw(icon_tint),
            })
            .boxed();

        let mut lines: Vec<AnyWidget> = vec![
            Text::new(self.title)
                .text_style(TextStyle::new().font_size(18).color(ink))
                .boxed(),
        ];
        if let Some(detail) = self.detail {
            lines.push(
                Text::new(detail)
                    .text_style(
                        TextStyle::new()
                            .text_overflow(TextOverflow::Wrap)
                            .font_size(14)
                            .color(muted(theme)),
                    )
                    .boxed(),
            );
        }

        let mut cells: Vec<AnyWidget> = vec![
            icon,
            Expanded::new().box_child(Column::new().gaps(LayoutSpacing::all(2)).children(lines)),
        ];
        if let Some(value) = self.value {
            cells.push(
                Text::new(value).text_style(TextStyle::new().font_size(16).color(muted(theme))).boxed(),
            );
        }
        if let Some(trailing) = trailing(theme, &self.trailing) {
            cells.push(trailing);
        }
        let content = Container::new()
            .padding(LayoutSpacing::new().top(12).bottom(12).left(ROW_INSET as u32).right(ROW_INSET as u32))
            .child(
                Row::new()
                    .vertical_alignment(BoxAlignment::Center)
                    .gaps(LayoutSpacing::horizontal(8.into()))
                    .children(cells),
            )
            .boxed();

        match self.on_press {
            None => content,
            Some(on_press) => Row::new()
                .children([Expanded::new().box_child(
                    Button::new()
                        .decoration(BoxDecoration::new().border_radius(ROW_RADIUS))
                        .hover_decoration(
                            BoxDecoration::new()
                                .background_color(theme.primary_color().with_opacity(8))
                                .border_radius(ROW_RADIUS),
                        )
                        .press_decoration(
                            BoxDecoration::new()
                                .background_color(theme.primary_color().with_opacity(16))
                                .border_radius(ROW_RADIUS),
                        )
                        .on_press(on_press)
                        .child(content),
                )])
                .boxed(),
        }
    }
}

fn trailing(theme: ForumTheme, trailing: &Trailing) -> Option<AnyWidget> {
    match trailing {
        Trailing::Nothing => None,
        Trailing::Chevron => Some(lucide(aimer::Icons::CHEVRON_RIGHT, muted(theme))),
        Trailing::Switch(on) => Some(switch(theme, *on)),
        Trailing::Locked => Some(lucide(aimer::Icons::LOCK, muted(theme))),
    }
}

/// A switch as the rows show it: a pill that fills with the accent colour when it
/// is on, with a round knob at the end it is on.
fn switch(theme: ForumTheme, on: bool) -> AnyWidget {
    let track = if on { theme.primary_color() } else { theme.on_background_color().with_alpha(0.25) };
    Container::new()
        .width(Dimension::Px(SWITCH_WIDTH))
        .height(Dimension::Px(SWITCH_HEIGHT))
        .box_decoration(BoxDecoration::new().background_color(track).border_radius(SWITCH_HEIGHT / 2.0))
        .padding(LayoutSpacing::all(((SWITCH_HEIGHT - SWITCH_KNOB) / 2.0) as u32))
        .child(
            Align::new()
                .alignment(if on { Alignment::MidRight } else { Alignment::MidLeft })
                .child(
                    Container::new()
                        .width(Dimension::Px(SWITCH_KNOB))
                        .height(Dimension::Px(SWITCH_KNOB))
                        .box_decoration(
                            BoxDecoration::new().background_color(Color::WHITE).border_radius(SWITCH_KNOB / 2.0),
                        )
                        .child(ZeroSizedBox),
                ),
        )
        .boxed()
}

fn lucide(icon: &'static StaticSvg, color: Color) -> AnyWidget {
    Svg::new(icon).style("path", SvgStyle::new().stroke(color)).width(ICON_SIZE).height(ICON_SIZE).boxed()
}

/// The colour of what is said quietly: details, headings and arrows.
pub fn muted(theme: ForumTheme) -> Color {
    theme.on_background_color().with_opacity(150)
}

/// What the tests of the pages share: pressing a page and waiting for it.
#[cfg(test)]
pub mod testing {
    use aimer::HeadlessAimerApp;
    use aimer::Widget;
    use aimer::quiver::winit::dpi::PhysicalPosition;
    use aimer::quiver::winit::event::{DeviceId, Touch, TouchPhase, WindowEvent};
    use std::time::Duration;

    /// Lets the page animate to rest and draws it, so that what the next press
    /// lands on is where it is now.
    pub fn settle(app: &mut HeadlessAimerApp<impl Widget + 'static>) {
        for _ in 0..30 {
            std::thread::sleep(Duration::from_millis(20));
            app.pump_frames(3);
        }
        app.render_frame();
    }

    /// A finger comes down at `at` and lifts again. Each press is a finger of its
    /// own, so none is taken for the next of one that never left.
    pub fn tap(app: &mut HeadlessAimerApp<impl Widget + 'static>, finger: u64, at: (f64, f64)) {
        for phase in [TouchPhase::Moved, TouchPhase::Started, TouchPhase::Ended] {
            app.send_window_event(WindowEvent::Touch(Touch {
                device_id: DeviceId::dummy(),
                phase,
                location: PhysicalPosition::new(at.0, at.1),
                force: None,
                id: finger,
            }));
            app.pump_frames(2);
        }
    }
}
