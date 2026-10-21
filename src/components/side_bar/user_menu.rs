use std::time::Duration;

use crate::router::AppRouter;
use crate::theme::ForumTheme;
use aimer::animation::curves;
use aimer::input::button::Button;
use aimer::modal::{
    Anchor, AnchorHandle, Floating, FloatingAlign, FloatingSide, ModalAnimation, ModalController,
};
use aimer::router::NavigatorController;
use aimer::style::{BoxDecoration, BoxShadow, LayoutSpacing, TextAlign, TextStyle, Theme};
use aimer::{
    Align, AnyWidget, BoxAlignment, BuildContext, Color, Column, Container, Dimension, Padding,
    Row, StatelessWidget, Text, Widget,
};

const AVATAR_SIZE: f32 = 28.0;
const MENU_WIDTH: f32 = 220.0;
const MENU_ITEM_HEIGHT: f32 = 40.0;
const MENU_PADDING: f32 = 8.0;

/// The user chip pinned to the bottom of the sidebar: a circular avatar next to
/// the username. Pressing it opens an anchored dropdown of account actions.
#[derive(Clone, StatelessWidget)]
pub struct UserMenuButton;

impl StatelessWidget for UserMenuButton {
    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme = *ForumTheme::of(ctx);
        let navigator = NavigatorController::<AppRouter>::of(ctx);
        let anchor = AnchorHandle::new();

        let button = Button::new()
            .decoration(theme.box_decor())
            .hover_decoration(theme.hover_box_decor())
            .on_press({
                let anchor = anchor.clone();
                let navigator = navigator.clone();
                move || {
                    Floating::new()
                        .anchor(anchor.clone())
                        .side(FloatingSide::Bottom)
                        .align(FloatingAlign::Start)
                        .gap(6.0)
                        .animation(enter_animation())
                        .child(menu_panel(theme, navigator.clone()))
                        .show();
                }
            })
            .child(
                Container::new().padding(LayoutSpacing::all(8)).child(
                    Row::new()
                        .vertical_alignment(BoxAlignment::Center)
                        .gaps(LayoutSpacing::all(10))
                        .children([
                            circular_avatar(theme).boxed(),
                            Text::new("Username")
                                .text_style(
                                    TextStyle::new()
                                        .font_size(16)
                                        .color(theme.on_surface_color()),
                                )
                                .boxed(),
                        ]),
                ),
            );

        Anchor::new().handle(anchor).child(button)
    }
}

/// A circular avatar placeholder: the accent colour with the user's initial.
fn circular_avatar(theme: ForumTheme) -> impl Widget {
    sized_avatar(theme, AVATAR_SIZE)
}

/// The avatar placeholder `size` across, as the icon of the account tab.
pub fn sized_avatar(theme: ForumTheme, size: f32) -> impl Widget {
    Container::new()
        .width(Dimension::Px(size))
        .height(Dimension::Px(size))
        .box_decoration(
            BoxDecoration::new()
                .background_color(theme.primary_color())
                .border_radius(size / 2.0),
        )
        .child(
            Align::new().alignment(TextAlign::MidCenter).child(
                Text::new("U").text_style(
                    TextStyle::new()
                        .font_size((size * 0.5) as u32)
                        .color(theme.on_primary_color()),
                ),
            ),
        )
}

/// The dropdown panel: a rounded, shadowed card holding one row per action.
fn menu_panel(theme: ForumTheme, navigator: NavigatorController<AppRouter>) -> impl Widget {
    let settings = navigator.clone();
    let border = theme.box_decor().border;

    Container::new()
        .width(Dimension::Px(MENU_WIDTH))
        .height(Dimension::Px(3.0 * MENU_ITEM_HEIGHT + MENU_PADDING * 2.5))
        .padding(LayoutSpacing::all(MENU_PADDING as u32))
        .box_decoration(
            BoxDecoration::new()
                .background_color(theme.surface_color())
                .border(border)
                .border_radius(12)
                .box_shadow(vec![
                    BoxShadow::new()
                        .color(Color::BLACK.with_opacity(70))
                        .blur(18.0)
                        .offset_y(8.0),
                ]),
        )
        .child(Column::new().children([
            menu_item("Profile", theme, move || {}),
            menu_item("Settings", theme, move || {
                settings.set_route(AppRouter::Settings)
            }),
            menu_item("Sign out", theme, move || {}),
        ]))
}

/// A single selectable menu row. Selecting it runs `on_select` and dismisses
/// the panel.
fn menu_item(label: &'static str, theme: ForumTheme, on_select: impl Fn() + 'static) -> AnyWidget {
    Container::new()
        .height(Dimension::Px(MENU_ITEM_HEIGHT))
        .child(
            Button::new()
                .hover_decoration(
                    BoxDecoration::new()
                        .background_color(theme.primary_color().with_alpha(0.2))
                        .border_radius(8),
                )
                .press_decoration(
                    BoxDecoration::new()
                        .background_color(theme.primary_color().with_alpha(0.45))
                        .border_radius(8),
                )
                .on_press(move || {
                    on_select();
                    ModalController::dismiss_top();
                })
                .decoration(
                    BoxDecoration::new()
                        .background_color(Color::Transparent)
                        .border_radius(8),
                )
                .child(
                    Container::new()
                        .padding(LayoutSpacing::new().left(12))
                        .child(
                            Text::new(label).text_align(TextAlign::MidLeft).text_style(
                                TextStyle::new()
                                    .font_size(16)
                                    .color(theme.on_surface_color()),
                            ),
                        ),
                ),
        )
        .boxed()
}

/// The enter/exit transition shared by the dropdown.
fn enter_animation() -> ModalAnimation {
    ModalAnimation::new()
        .enter_duration(Duration::from_millis(160))
        .exit_duration(Duration::from_millis(120))
        .enter_curve(curves::EASE_OUT)
        .exit_curve(curves::EASE_IN)
        .content_scale_from(0.94)
}
