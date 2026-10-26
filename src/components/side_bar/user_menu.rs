use std::time::Duration;

use crate::api::session;
use crate::components::avatar::round_avatar;
use crate::components::windows;
use crate::models::comments::avatar_url;
use crate::store::AppStore;
use crate::theme::ForumTheme;
use aimer::animation::curves;
use aimer::input::button::Button;
use aimer::modal::{
    Anchor, AnchorHandle, Floating, FloatingAlign, FloatingSide, ModalAnimation, ModalController,
};
use aimer::style::{BoxDecoration, BoxShadow, LayoutSpacing, TextAlign, TextStyle, Theme};
use aimer::{
    Align, AnyWidget, BoxAlignment, BuildContext, Color, Column, Container, Dimension, Icons,
    ProviderContext, ProviderHandle, Row, StatelessWidget, Svg, SvgStyle, Text, Widget,
};

const AVATAR_SIZE: f32 = 28.0;
const MENU_WIDTH: f32 = 220.0;
const MENU_ITEM_HEIGHT: f32 = 40.0;
const MENU_PADDING: f32 = 8.0;

/// Who the chip shows: the signed-in member and the picture the forum has for
/// them once their profile has been seen, or nobody.
#[derive(Clone, PartialEq)]
struct Member {
    username: String,
    /// The forum's template for the avatar; empty until the profile is loaded.
    avatar_template: String,
}

/// The user chip pinned to the bottom of the sidebar: a circular avatar next to
/// the username, or the way to sign in when nobody is. Pressing it opens an
/// anchored dropdown of account actions.
#[derive(Clone, StatelessWidget)]
pub struct UserMenuButton;

impl StatelessWidget for UserMenuButton {
    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme = *ForumTheme::of(ctx);
        let store = ProviderHandle::<AppStore>::of(ctx);
        let anchor = AnchorHandle::new();
        let member = ctx.select::<AppStore, Option<Member>>(|store| {
            store.current_user.as_ref().map(|username| Member {
                username: username.clone(),
                avatar_template: store
                    .cached_profile(username)
                    .map(|profile| profile.detail.avatar_template)
                    .unwrap_or_default(),
            })
        });

        let (picture, name) = match &member {
            Some(member) => {
                let url = match member.avatar_template.as_str() {
                    "" => String::new(),
                    template => avatar_url(template, 64),
                };
                (
                    round_avatar(theme, &url, &member.username, AVATAR_SIZE),
                    member.username.clone(),
                )
            }
            None => (guest_icon(theme), "Sign in".to_string()),
        };

        let button = Button::new()
            .decoration(theme.box_decor())
            .hover_decoration(theme.hover_box_decor())
            .on_press({
                let anchor = anchor.clone();
                let signed_in = member.is_some();
                move || {
                    Floating::new()
                        .anchor(anchor.clone())
                        .side(FloatingSide::Bottom)
                        .align(FloatingAlign::Start)
                        .gap(6.0)
                        .animation(enter_animation())
                        .child(menu_panel(theme, store.clone(), signed_in))
                        .show();
                }
            })
            .child(
                Container::new().padding(LayoutSpacing::all(8)).child(
                    Row::new()
                        .vertical_alignment(BoxAlignment::Center)
                        .gaps(LayoutSpacing::all(10))
                        .children([
                            picture,
                            Text::new(name)
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

/// What stands where the avatar would, with nobody signed in.
fn guest_icon(theme: ForumTheme) -> AnyWidget {
    Container::new()
        .width(Dimension::Px(AVATAR_SIZE))
        .height(Dimension::Px(AVATAR_SIZE))
        .child(
            Svg::new(Icons::USER)
                .style("path", SvgStyle::new().stroke(theme.on_surface_color()))
                .width(AVATAR_SIZE as u32)
                .height(AVATAR_SIZE as u32),
        )
        .boxed()
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
fn menu_panel(theme: ForumTheme, store: ProviderHandle<AppStore>, signed_in: bool) -> impl Widget {
    let border = theme.box_decor().border;

    // Each opens in a window of its own, not as a page of the app.
    let settings = {
        let store = store.clone();
        menu_item("Settings", theme, move || windows::open_settings(&store))
    };
    let items = if signed_in {
        let profile = {
            let store = store.clone();
            menu_item("Profile", theme, move || windows::open_profile(&store))
        };
        // Unlike the settings this does not ask twice: the menu was opened on
        // purpose, and a choice in it is one press.
        let sign_out = menu_item("Sign out", theme, move || session::end(&store));
        vec![profile, settings, sign_out]
    } else {
        let sign_in = {
            let store = store.clone();
            menu_item("Sign in", theme, move || windows::open_login(&store))
        };
        vec![sign_in, settings]
    };

    Container::new()
        .width(Dimension::Px(MENU_WIDTH))
        .height(Dimension::Px(
            items.len() as f32 * MENU_ITEM_HEIGHT + MENU_PADDING * 2.5,
        ))
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
        .child(Column::new().children(items))
}

/// A single selectable menu row. Selecting it runs `on_select` and dismisses
/// the panel.
fn menu_item(label: &'static str, theme: ForumTheme, on_select: impl Fn() + 'static) -> AnyWidget {
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
        .box_child(
            Container::new()
                .height(Dimension::Px(MENU_ITEM_HEIGHT))
                .width(Dimension::Percent(100f32))
                .padding(LayoutSpacing::new().left(12))
                .child(
                    Text::new(label).text_align(TextAlign::MidLeft).text_style(
                        TextStyle::new()
                            .font_size(16)
                            .color(theme.on_surface_color()),
                    ),
                ),
        )
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::router::AppRouter;
    use crate::screen::settings::widgets::testing;
    use crate::store::DialogPage;
    use aimer::quiver::winit::dpi::PhysicalSize;
    use aimer::router::{Navigator, NavigatorController};
    use aimer::style::AnimatedTheme;
    use aimer::{AimerApp, HeadlessOptions, Provider, StatelessWidget};

    thread_local! {
        static SEEN: std::cell::RefCell<Option<(ProviderHandle<AppStore>, NavigatorController<AppRouter>)>> =
            const { std::cell::RefCell::new(None) };
    }

    /// The panel on its own, at the top left of the window, keeping hold of the
    /// store and the navigator it acts on.
    #[derive(Clone, StatelessWidget)]
    struct MenuPanel;

    impl StatelessWidget for MenuPanel {
        fn build(&self, ctx: &BuildContext) -> impl Widget {
            let store = ProviderHandle::<AppStore>::of(ctx);
            let navigator = NavigatorController::<AppRouter>::of(ctx);
            let signed_in = store.read().current_user.is_some();
            SEEN.with(|seen| *seen.borrow_mut() = Some((store.clone(), navigator.clone())));
            AnimatedTheme::new()
                .data(ForumTheme::light())
                .child(Themed { store, signed_in })
        }
    }

    /// The panel, under the theme it needs.
    #[derive(Clone, StatelessWidget)]
    struct Themed {
        store: ProviderHandle<AppStore>,
        signed_in: bool,
    }

    impl StatelessWidget for Themed {
        fn build(&self, ctx: &BuildContext) -> impl Widget {
            Align::new()
                .alignment(aimer::Alignment::TopLeft)
                .child(menu_panel(
                    *ForumTheme::of(ctx),
                    self.store.clone(),
                    self.signed_in,
                ))
        }
    }

    fn open(signed_in: bool) -> aimer::HeadlessAimerApp<impl Widget + 'static> {
        let mut store = AppStore::default();
        if signed_in {
            store.sign_in("domma".to_owned());
        }
        crate::testing::hold();
        let mut app = AimerApp::start_headless_with(
            Provider::new()
                .create(move || store.clone())
                .child(Navigator::new(AppRouter::Latest, |_| MenuPanel.boxed())),
            HeadlessOptions {
                size: PhysicalSize::new(400, 300),
                scale_factor: 1.0,
            },
        );
        testing::settle(&mut app);
        app
    }

    fn user() -> Option<String> {
        SEEN.with(|seen| {
            seen.borrow()
                .as_ref()
                .unwrap()
                .0
                .read()
                .current_user
                .clone()
        })
    }

    fn route() -> AppRouter {
        SEEN.with(|seen| seen.borrow().as_ref().unwrap().1.current_route())
    }

    fn windows() -> crate::store::Windows {
        SEEN.with(|seen| seen.borrow().as_ref().unwrap().0.read().windows)
    }

    /// The rows are 40 tall under the panel's 8 of padding.
    fn row(index: usize) -> (f64, f64) {
        (100.0, 8.0 + 40.0 * index as f64 + 20.0)
    }

    #[test]
    fn profile_opens_the_profile_window() {
        let mut app = open(true);
        testing::tap(&mut app, 1, row(0));
        testing::settle(&mut app);
        assert!(windows().profile && windows().dialog.is_none());
        // The app stays on the page it was on.
        assert!(route() == AppRouter::Latest);
    }

    #[test]
    fn settings_opens_the_settings_window() {
        let mut app = open(true);
        testing::tap(&mut app, 1, row(1));
        testing::settle(&mut app);
        assert!(windows().dialog == Some(DialogPage::Settings) && !windows().profile);
        assert!(route() == AppRouter::Latest);
    }

    #[test]
    fn sign_out_signs_the_member_out() {
        let mut app = open(true);
        assert_eq!(user().as_deref(), Some("domma"));
        testing::tap(&mut app, 1, row(2));
        testing::settle(&mut app);
        assert_eq!(user(), None);
    }

    #[test]
    fn nobody_signed_in_is_offered_the_sign_in_first_in_a_window() {
        let mut app = open(false);
        testing::tap(&mut app, 1, row(0));
        testing::settle(&mut app);
        assert!(windows().dialog == Some(DialogPage::Login) && !windows().profile);
        assert!(route() == AppRouter::Latest);
    }
}
