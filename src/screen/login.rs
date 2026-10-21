mod email_code;
mod flow;
mod password;

pub use email_code::EmailCodeScreen;
pub use password::PasswordScreen;

use crate::app_shell::{MOBILE_TOP, is_mobile};
use crate::components::auth_button::{Glyph, filled_icon_button, outlined_button, text_button};
use crate::components::page_title::PageTitle;
use crate::router::AppRouter;
use crate::theme::ForumTheme;
use aimer::input::button::Button;
use aimer::router::NavigatorController;
use aimer::share::ShareRef;
use aimer::style::{BoxDecoration, LayoutSpacing, TextAlign, TextOverflow, TextStyle, Theme};
use aimer::{
    Align, Alignment, AnyWidget, BoxAlignment, BuildContext, Color, Column, Container, Dimension,
    Expanded, FontWeight, Icons, Row, Scrollable, SizedBox, State, StateUpdater, StatefulWidget,
    Svg, SvgStyle, Text, Widget, case,
};

const GITHUB_ICON: &str = include_str!("../../assets/github.svg");
/// How the form shares a wide window: the margins at each side and the form.
const WIDE_MARGIN_FLEX: f32 = 1.0;
const WIDE_FORM_FLEX: f32 = 1.6;
const BADGE_SIZE: f32 = 76.0;
/// The room under the last line of the page: the tab bar gives way to the sign-in
/// pages, so there is none of it to clear.
const BOTTOM_MARGIN: f32 = 24.0;
const TERMS_URL: &str = "https://users.rust-lang.org/tos";
const GUIDELINES_URL: &str = "https://users.rust-lang.org/faq";

/// The page to sign in from: the forum's mark over the ways in (GitHub, a
/// passkey, a one-time code emailed to the member, or a username and password),
/// the last two of which have pages of their own; see [`EmailCodeScreen`] and
/// [`PasswordScreen`].
///
/// Only the pages are here for now. Nothing is sent to the forum, so every way in
/// ends in a note saying it isn't connected yet (the pages say so up front too).
#[derive(Clone, StatefulWidget)]
pub struct LoginScreen;

pub struct LoginState {
    /// What the last press came to, shown above the terms.
    message: Option<String>,
    updater: StateUpdater<Self>,
}

impl StatefulWidget for LoginScreen {
    type State = LoginState;

    fn create_state(self) -> Self::State {
        LoginState {
            message: None,
            updater: StateUpdater::new(),
        }
    }
}

impl LoginState {
    fn sign_in_with_github(&mut self) {
        self.message = Some("Signing in with GitHub isn't available yet.".into());
    }

    fn sign_in_with_passkey(&mut self) {
        self.message = Some("Signing in with a passkey isn't available yet.".into());
    }
}

impl State<LoginScreen> for LoginState {
    fn init_state(&mut self, updater: StateUpdater<Self>) {
        self.updater = updater;
    }

    fn adopt_config_from(&mut self, _new: Self) {}

    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme = *ForumTheme::of(ctx);
        let navigator = NavigatorController::<AppRouter>::of(ctx);
        let updater = self.updater;
        let (to_email, to_password, skip) = (navigator.clone(), navigator.clone(), navigator);

        // One column, so that the pills are given the width of the page: see
        // `form_page`.
        let mut page: Vec<AnyWidget> = vec![
            // Clear of the bar over the page, with its back button.
            SizedBox::new()
                .height(case!(is_mobile(ctx), MOBILE_TOP) + 56)
                .boxed(),
            Expanded::new().box_child(
                Align::new()
                    .alignment(Alignment::MidCenter)
                    .child(brand(theme)),
            ),
            filled_icon_button(
                theme,
                Glyph::Embedded(GITHUB_ICON),
                "Continue with GitHub",
                move || updater.set_state(|state| state.sign_in_with_github()),
            ),
            outlined_button(
                theme,
                Some(Glyph::Lucide(Icons::FINGERPRINT_PATTERN)),
                "Continue with a passkey",
                move || updater.set_state(|state| state.sign_in_with_passkey()),
            ),
            outlined_button(
                theme,
                Some(Glyph::Lucide(Icons::MAIL)),
                "Email me a one-time code",
                move || to_email.push(AppRouter::LoginEmail),
            ),
            outlined_button(theme, None, "Log in with a username", move || {
                to_password.push(AppRouter::LoginPassword)
            }),
            text_button("Skip", theme.on_background_color(), 17, move || {
                if skip.can_pop() {
                    skip.pop();
                } else {
                    skip.push(AppRouter::Latest);
                }
            }),
        ];
        if let Some(message) = &self.message {
            page.push(note(theme, message));
        }
        // page.extend(terms(theme));
        page.push(
            SizedBox::new()
                .height(BOTTOM_MARGIN)
                .boxed(),
        );

        Container::new()
            .color(theme.background_color())
            .padding(LayoutSpacing::horizontal(24.into()))
            .child(centred(
                ctx,
                Column::new()
                    .horizontal_alignment(BoxAlignment::Center)
                    .gaps(LayoutSpacing::all(12).top(6).bottom(6))
                    .children(page),
            ))
    }
}

/// The forum's mark: a badge in the accent colour over the name.
fn brand(theme: ForumTheme) -> AnyWidget {
    Column::new()
        .horizontal_alignment(BoxAlignment::Center)
        .gaps(LayoutSpacing::all(18))
        .children([
            Container::new()
                .width(Dimension::Px(BADGE_SIZE))
                .height(Dimension::Px(BADGE_SIZE))
                .box_decoration(
                    BoxDecoration::new()
                        .background_color(theme.primary_color())
                        .border_radius(BADGE_SIZE / 2.0),
                )
                .child(
                    Align::new().alignment(Alignment::MidCenter).child(
                        Svg::new(Icons::MESSAGES_SQUARE)
                            .style("path", SvgStyle::new().stroke(theme.on_primary_color()))
                            .width(36)
                            .height(36),
                    ),
                )
                .boxed(),
            Text::new("Rust Forum")
                .text_style(
                    TextStyle::new()
                        .font_weight(FontWeight::Bold)
                        .font_size(34)
                        .color(theme.on_background_color()),
                )
                .boxed(),
        ])
        .boxed()
}

/// The line saying what going on means, and the links to read it.
///
/// Two widgets for the page's own column, not a column of their own.
fn terms(theme: ForumTheme) -> [AnyWidget; 2] {
    let muted: Color = theme.on_background_color().with_opacity(160);
    let link = |label: &str, url: &'static str| {
        Button::new()
            .no_decor()
            .on_press(move || {
                if let Err(error) = webbrowser::open(url) {
                    aimer::console::error!("could not open {url}: {error}");
                }
            })
            .child(
                Text::new(label.to_string()).text_style(
                    TextStyle::new()
                        .font_weight(FontWeight::Bold)
                        .font_size(14)
                        .color(theme.on_background_color().with_opacity(200)),
                ),
            )
            .boxed()
    };
    [
        Text::new("By continuing, you agree to our")
            .text_align(TextAlign::MidCenter)
            .text_style(
                TextStyle::new()
                    .text_overflow(TextOverflow::Wrap)
                    .font_size(14)
                    .color(muted),
            )
            .boxed(),
        Row::new()
            .horizontal_alignment(BoxAlignment::Center)
            .vertical_alignment(BoxAlignment::Center)
            .gaps(LayoutSpacing::horizontal(6.into()))
            .children([
                link("Terms", TERMS_URL),
                Text::new("&")
                    .text_style(TextStyle::new().font_size(14).color(muted))
                    .boxed(),
                link("Guidelines", GUIDELINES_URL),
            ])
            .boxed(),
    ]
}

/// What the last press came to, in the accent colour and centred.
fn note(theme: ForumTheme, message: &str) -> AnyWidget {
    Text::new(message.to_string())
        .text_align(TextAlign::MidCenter)
        .text_style(
            TextStyle::new()
                .text_overflow(TextOverflow::Wrap)
                .font_size(14)
                .color(theme.primary_color()),
        )
        .boxed()
}

/// `page` as wide as a phone on a wide window, with margins at each side; the
/// whole width on a narrow one.
fn centred(ctx: &BuildContext, page: impl Widget + 'static) -> AnyWidget {
    if is_mobile(ctx) {
        page.boxed()
    } else {
        Row::new()
            .children([
                Expanded::new()
                    .flex(WIDE_MARGIN_FLEX)
                    .box_child(SizedBox::new()),
                Expanded::new().flex(WIDE_FORM_FLEX).box_child(page),
                Expanded::new()
                    .flex(WIDE_MARGIN_FLEX)
                    .box_child(SizedBox::new()),
            ])
            .boxed()
    }
}

/// A page of the sign-in flow with a form on it: the title and a line under it,
/// the form, a note on what the last press came to, and the line saying signing
/// in is a preview.
///
/// The widgets of the form are in one column with the title, not in a column of
/// their own: a column inside a column is given no width to fill, and its fields
/// and buttons come out as wide as what is in them.
fn form_page(
    ctx: &BuildContext,
    theme: ForumTheme,
    title: &str,
    lead: &str,
    body: Vec<AnyWidget>,
    message: Option<&str>,
) -> AnyWidget {
    let mut form = vec![
        PageTitle::new(ShareRef::from_str_slice(title))
            .padding(LayoutSpacing::new())
            .boxed(),
        paragraph(theme, lead, 16, 160),
        SizedBox::new().height(8).boxed(),
    ];
    form.extend(body);
    if let Some(message) = message {
        form.push(
            Text::new(message.to_string())
                .text_style(
                    TextStyle::new()
                        .text_overflow(TextOverflow::Wrap)
                        .font_size(14)
                        .color(theme.primary_color()),
                )
                .boxed(),
        );
    }
    form.push(SizedBox::new().height(4).boxed());
    form.push(paragraph(
        theme,
        "Preview: signing in isn't connected to the forum yet.",
        13,
        120,
    ));

    let centred = centred(
        ctx,
        Column::new()
            .gaps(LayoutSpacing::all(14).top(4).bottom(4))
            .children(form),
    );

    Container::new()
        .color(theme.background_color())
        .child(
            Scrollable::new()
                .key(format!("scrollable-{title}"))
                .box_child(
                    Column::new().children([
                        // Clear of the bar over the page, with its back button.
                        SizedBox::new()
                            .height(case!(is_mobile(ctx), MOBILE_TOP) + 70)
                            .boxed(),
                        Container::new()
                            .padding(LayoutSpacing::horizontal(24.into()))
                            .child(centred)
                            .boxed(),
                        SizedBox::new().height(90).boxed(),
                    ]),
                ),
        )
        .boxed()
}

/// A line of muted text that wraps.
fn paragraph(theme: ForumTheme, text: &str, size: u32, opacity: u8) -> AnyWidget {
    Text::new(text.to_string())
        .text_style(
            TextStyle::new()
                .text_overflow(TextOverflow::Wrap)
                .font_size(size)
                .color(theme.on_background_color().with_opacity(opacity)),
        )
        .boxed()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::AppStore;
    use aimer::quiver::winit::dpi::PhysicalSize;
    use aimer::router::Navigator;
    use aimer::{AimerApp, HeadlessOptions, Provider};
    use std::time::Duration;

    #[test]
    fn each_way_in_that_stays_on_the_page_ends_in_a_note_that_it_is_not_available_yet() {
        let mut state = LoginScreen.create_state();
        assert!(state.message.is_none());
        state.sign_in_with_github();
        assert!(
            state
                .message
                .as_deref()
                .is_some_and(|m| m.contains("GitHub"))
        );
        state.sign_in_with_passkey();
        assert!(
            state
                .message
                .as_deref()
                .is_some_and(|m| m.contains("passkey"))
        );
    }

    /// The app on `route` in a window `width` wide, settled, and the nodes it
    /// paints.
    fn painted(route: AppRouter, width: u32) -> usize {
        let store = AppStore::default();
        crate::testing::hold();
        let mut app = AimerApp::start_headless_with(
            Provider::new()
                .create(move || store.clone())
                .child(Navigator::new(route, |r| r.boxed())),
            HeadlessOptions {
                size: PhysicalSize::new(width, 900),
                scale_factor: 1.0,
            },
        );
        for _ in 0..10 {
            std::thread::sleep(Duration::from_millis(20));
            app.pump_frames(3);
        }
        let census = app
            .paint_source_census()
            .expect("a mounted page has a root");
        assert!(census.sync_error.is_none(), "{:?}", census.sync_error);
        assert!(
            census.invalid_bounds.is_empty(),
            "{:?}",
            census.invalid_bounds
        );
        census.local_v2
    }

    #[test]
    fn the_login_page_builds_and_paints_under_its_bar() {
        // The shell paints about 45 nodes with its tab bar gone from these pages;
        // the mark, the pills and the link add about ninety (137 when this was
        // measured).
        let nodes = painted(AppRouter::Login, 390);
        assert!(nodes >= 120, "{nodes} nodes");
    }

    #[test]
    fn the_login_page_builds_in_a_wide_window_too() {
        let nodes = painted(AppRouter::Login, 1400);
        assert!(nodes >= 150, "{nodes} nodes");
    }

    #[test]
    fn the_email_code_and_password_pages_build_and_paint_under_their_bar() {
        for route in [AppRouter::LoginEmail, AppRouter::LoginPassword] {
            // 120 nodes when this was measured, with the shell's 45.
            let nodes = painted(route, 390);
            assert!(nodes >= 100, "{nodes} nodes");
        }
    }
}
