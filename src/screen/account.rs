use crate::app_shell::{MOBILE_TOP, is_mobile};
use crate::components::auth_button::filled_button;
use crate::components::avatar::round_avatar;
use crate::router::AppRouter;
use crate::screen::user_profile::{UserProfileScreen, card_count};
use crate::store::AppStore;
use crate::theme::ForumTheme;
use aimer::router::NavigatorController;
use aimer::share::ShareRef;
use aimer::style::{LayoutSpacing, TextOverflow, TextStyle, Theme};
use aimer::{
    BoxAlignment, BuildContext, Column, Container, Expanded, FontWeight, ProviderContext, Row,
    Scrollable, SizedBox, StatelessWidget, Text, Widget, case,
};

const AVATAR_SIZE: f32 = 96.0;

/// The page of the account tab: the signed-in user's own profile.
///
/// Nobody can sign in yet, so it shows what the profile will look like, a guest
/// with nothing counted and an invitation to sign in; once there is a signed-in
/// user it is their profile page, the same as any other member's.
#[derive(Clone, StatelessWidget)]
pub struct AccountScreen;

impl StatelessWidget for AccountScreen {
    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme = *ForumTheme::of(ctx);
        let user = ctx.select::<AppStore, Option<String>>(|s| s.current_user.clone());

        if let Some(username) = user {
            return UserProfileScreen::new(ShareRef::from_str_slice(&username)).boxed();
        }

        Container::new()
            .color(theme.background_color())
            .child(Scrollable::new().key("scrollable-account").box_child(Column::new().children([
                SizedBox::new().height(case!(is_mobile(ctx), MOBILE_TOP) + 70).boxed(),
                Container::new()
                    .padding(LayoutSpacing::horizontal(24.into()))
                    .child(signed_out(theme, NavigatorController::<AppRouter>::of(ctx)))
                    .boxed(),
                SizedBox::new().height(90).boxed(),
            ])))
            .boxed()
    }
}

/// The profile of nobody: the card of a profile with its figures at nothing,
/// and what to do about it.
fn signed_out(theme: ForumTheme, navigator: NavigatorController<AppRouter>) -> impl Widget {
    let muted = theme.on_background_color().with_opacity(160);

    let card = Row::new()
        .vertical_alignment(BoxAlignment::Center)
        .gaps(LayoutSpacing::horizontal(24.into()))
        .children([
            round_avatar(theme, "", "Guest", AVATAR_SIZE),
            Expanded::new().box_child(Column::new().gaps(LayoutSpacing::all(10)).children([
                Text::new("Guest")
                    .text_style(
                        TextStyle::new()
                            .font_weight(FontWeight::Bold)
                            .font_size(18)
                            .color(theme.on_background_color()),
                    )
                    .boxed(),
                Row::new()
                    .gaps(LayoutSpacing::horizontal(8.into()))
                    .children([
                        card_count(theme, "0", "posts"),
                        card_count(theme, "0", "topics"),
                        card_count(theme, "0", "likes"),
                    ])
                    .boxed(),
            ])),
        ]);

    Column::new().gaps(LayoutSpacing::all(20)).children([
        card.boxed(),
        Column::new()
            .gaps(LayoutSpacing::all(6))
            .children([
                Text::new("You're not signed in")
                    .text_style(
                        TextStyle::new()
                            .font_weight(FontWeight::Bold)
                            .font_size(20)
                            .color(theme.on_background_color()),
                    )
                    .boxed(),
                Text::new(
                    "Your profile, topics, replies and likes will be here once you sign in.",
                )
                .text_style(
                    TextStyle::new().text_overflow(TextOverflow::Wrap).font_size(16).color(muted),
                )
                .boxed(),
            ])
            .boxed(),
        filled_button(theme, "Sign in", move || navigator.push(AppRouter::Login)),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::router::AppRouter;
    use aimer::quiver::winit::dpi::PhysicalSize;
    use aimer::router::Navigator;
    use aimer::{AimerApp, HeadlessOptions, Provider};
    use std::time::Duration;

    fn painted(route: AppRouter, seed: impl FnOnce(&mut AppStore)) -> usize {
        let mut store = AppStore::default();
        seed(&mut store);
        crate::testing::hold();
        let mut app = AimerApp::start_headless_with(
            Provider::new()
                .create(move || store.clone())
                .child(Navigator::new(route, |r| r.boxed())),
            HeadlessOptions { size: PhysicalSize::new(390, 900), scale_factor: 1.0 },
        );
        for _ in 0..10 {
            std::thread::sleep(Duration::from_millis(20));
            app.pump_frames(3);
        }
        app.paint_source_census().expect("a mounted page has a root").local_v2
    }

    #[test]
    fn nobody_signed_in_shows_a_guest_card_and_the_invitation() {
        // The shell around the page paints about 120 nodes; the card and the two
        // paragraphs add a few dozen (153 when this was measured).
        let nodes = painted(AppRouter::Account, |_| {});
        assert!((140..200).contains(&nodes), "{nodes} nodes");
    }

    #[test]
    fn a_signed_in_user_gets_their_profile_page() {
        // Their profile is not loaded, so it shows its skeleton, which paints well
        // over twice what the card of a guest does.
        let nodes = painted(AppRouter::Account, |s| s.current_user = Some("domma".into()));
        assert!(nodes >= 200, "{nodes} nodes");
    }

    #[test]
    fn the_settings_page_builds_under_its_bar() {
        // 156 nodes when this was measured.
        let nodes = painted(AppRouter::Settings, |_| {});
        assert!(nodes >= 130, "{nodes} nodes");
    }
}
