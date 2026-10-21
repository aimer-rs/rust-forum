use super::widgets::{
    CONTENT_MARGIN, SettingsRow, Tile, Trailing, footnote, group, heading, inset, muted, page,
};
use crate::app_shell::is_mobile;
use crate::components::auth_button::text_button;
use crate::components::side_bar::{BAR_HEIGHT, bar_width, icon_of, preview_bar, tab_icon};
use crate::components::theme_selector::ThemeSelector;
use crate::preferences::{self, NavTab, NavbarTabs};
use crate::store::AppStore;
use crate::theme::ForumTheme;
use aimer::provider::media_query::MediaQuery;
use aimer::style::{BoxDecoration, LayoutSpacing, Theme};
use aimer::{
    Align, Alignment, AnyWidget, BuildContext, Column, Container, Dimension, ProviderContext,
    ProviderHandle, Scalable, SizedBox, Stack, StatelessWidget, Widget, ZeroSizedBox,
};

/// How tall the card that shows the bar is.
const PREVIEW_HEIGHT: f32 = 190.0;
/// The room between the bar and the bottom of the card.
const PREVIEW_BAR_MARGIN: u32 = 16;
/// The room kept at each side of the bar in the card.
const PREVIEW_SIDE_ROOM: f32 = 4.0;

/// The appearance of the app: the theme, and, in a narrow window, which tabs the
/// bar along the bottom has.
///
/// A preview of the bar sits over the list of tabs and follows each change as it
/// is made, so a tab can be tried in and out without leaving the page for the one
/// it is in.
#[derive(Clone, StatelessWidget)]
pub struct AppearanceScreen;

impl StatelessWidget for AppearanceScreen {
    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme = *ForumTheme::of(ctx);

        let mut body = vec![heading(theme, "Theme"), inset(ThemeSelector.boxed())];
        // The sidebar of a wide window has no tabs to choose.
        if is_mobile(ctx) {
            body.extend(navigation_bar(ctx, theme));
        }
        page(ctx, theme, "scrollable-appearance", "Appearance", body)
    }
}

/// The section for the tab bar: the preview, the list of tabs to switch, and the
/// way back to the default.
fn navigation_bar(ctx: &BuildContext, theme: ForumTheme) -> Vec<AnyWidget> {
    let store = ProviderHandle::<AppStore>::of(ctx);
    let bar = ctx.select::<AppStore, NavbarTabs>(|s| s.navbar.clone());

    let rows = NavTab::ALL
        .into_iter()
        .map(|tab| tab_row(theme, &store, &bar, tab))
        .collect();

    let mut section = vec![
        heading(theme, "Navigation bar"),
        inset(preview(theme, &bar, ctx)),
        SizedBox::new().height(16).boxed(),
        group(rows),
        footnote(
            theme,
            "The bar keeps at least two tabs. Account stays, as the settings are opened from it.",
        ),
    ];
    if bar != NavbarTabs::default() {
        section.push(inset(text_button("Restore the default tabs", theme.primary_color(), 15, {
            let store = store.clone();
            move || preferences::set_navbar(&store, NavbarTabs::default())
        })));
    }
    section
}

/// A row for one tab: its icon and name, and whether the bar has it.
fn tab_row(
    theme: ForumTheme,
    store: &ProviderHandle<AppStore>,
    bar: &NavbarTabs,
    tab: NavTab,
) -> AnyWidget {
    let on = bar.contains(tab);
    let row = SettingsRow::new(
        Tile::Custom(Box::new(move |tint| tab_icon(theme, icon_of(tab), tint))),
        tab.label(),
    );
    if tab.is_required() {
        return row.detail("Always on the bar").trailing(Trailing::Locked).build(theme);
    }
    let row = row.trailing(Trailing::Switch(on));
    if !bar.can_toggle(tab) {
        return row.detail("The bar needs at least two tabs").build(theme);
    }
    let store = store.clone();
    // The bar is read when the row is pressed, not when it was built, so that a
    // press never works from a bar the page has since moved on from.
    row.on_press(move || {
        let toggled = store.read().navbar.toggled(tab);
        preferences::set_navbar(&store, toggled);
    })
    .build(theme)
}

/// The bar as it will be, floating over a faint page the way it does in the app.
fn preview(theme: ForumTheme, bar: &NavbarTabs, ctx: &BuildContext) -> AnyWidget {
    let line = |width: f32| {
        Container::new()
            .width(Dimension::Px(width))
            .height(Dimension::Px(10.0))
            .box_decoration(
                BoxDecoration::new().background_color(muted(theme).with_alpha(0.25)).border_radius(5),
            )
            .child(ZeroSizedBox)
            .boxed()
    };
    // A page of cards for the glass to blur.
    let page = Container::new().padding(LayoutSpacing::all(16)).child(
        Column::new().gaps(LayoutSpacing::all(10)).children([
            line(180.0),
            line(260.0),
            line(120.0),
            line(220.0),
        ]),
    );

    let tabs = bar.tabs().to_vec();
    // A bar of every tab is nearly as wide as the window, so a narrow one is shown
    // smaller than it is, to be seen whole.
    let room = MediaQuery::select(ctx, |media| media.size.width) - 2.0 * CONTENT_MARGIN - 2.0 * PREVIEW_SIDE_ROOM;
    let scale = (room / bar_width(tabs.len())).min(1.0);
    let floating = Align::new().alignment(Alignment::BotCenter).child(
        Container::new().padding(LayoutSpacing::new().bottom(PREVIEW_BAR_MARGIN)).child(
            SizedBox::new()
                .width(bar_width(tabs.len()))
                .height(BAR_HEIGHT)
                .child(Scalable::new().scale(scale).child(preview_bar(theme, &tabs, Some(0)))),
        ),
    );

    Container::new()
        .height(Dimension::Px(PREVIEW_HEIGHT))
        .box_decoration(
            BoxDecoration::new()
                .background_color(theme.primary_color().with_alpha(0.1))
                .border_radius(18),
        )
        .child(Stack::new().children([page.boxed(), floating.boxed()]))
        .boxed()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::router::AppRouter;
    use aimer::quiver::winit::dpi::PhysicalSize;
    use aimer::router::Navigator;
    use aimer::{AimerApp, HeadlessOptions, Provider};
    use std::time::Duration;
    use aimer::BuildContext;
    use crate::screen::settings::widgets::testing;

    /// How many nodes the page paints in a window `width` across, with the bar of
    /// `tabs` chosen.
    fn painted(width: u32, tabs: NavbarTabs) -> usize {
        let mut store = AppStore::default();
        store.navbar = tabs;
        crate::testing::hold();
        let mut app = AimerApp::start_headless_with(
            Provider::new()
                .create(move || store.clone())
                .child(Navigator::new(AppRouter::SettingsAppearance, |r| r.boxed())),
            HeadlessOptions { size: PhysicalSize::new(width, 900), scale_factor: 1.0 },
        );
        for _ in 0..10 {
            std::thread::sleep(Duration::from_millis(20));
            app.pump_frames(3);
        }
        app.paint_source_census().expect("a mounted page has a root").local_v2
    }

    #[test]
    fn the_preview_and_the_list_follow_the_tabs_chosen() {
        let all = painted(390, NavbarTabs::default());
        let fewer = painted(390, NavbarTabs::new([NavTab::Latest]));
        assert!(fewer < all, "{fewer} nodes with two tabs, {all} with five");
    }

    #[test]
    fn a_wide_window_has_no_bar_to_arrange() {
        // Its sidebar has no tabs to choose, so the choice changes nothing on the page.
        let all = painted(900, NavbarTabs::default());
        let fewer = painted(900, NavbarTabs::new([NavTab::Latest]));
        assert_eq!(all, fewer);
    }

    thread_local! {
        static STORE: std::cell::RefCell<Option<ProviderHandle<AppStore>>> = const { std::cell::RefCell::new(None) };
    }

    /// The app's page: it keeps hold of the store and shows the route.
    #[derive(Clone, StatelessWidget)]
    struct Probe(AppRouter);

    impl StatelessWidget for Probe {
        fn build(&self, ctx: &BuildContext) -> impl Widget {
            STORE.with(|store| *store.borrow_mut() = Some(ProviderHandle::<AppStore>::of(ctx)));
            self.0.clone().boxed()
        }
    }

    fn tabs_of_store() -> Vec<NavTab> {
        STORE.with(|store| store.borrow().as_ref().expect("the page was built").read().navbar.tabs().to_vec())
    }

    #[test]
    fn pressing_a_tab_takes_it_off_the_bar_and_pressing_it_again_puts_it_back() {
        crate::testing::hold();
        let mut app = AimerApp::start_headless_with(
            Provider::new()
                .create(AppStore::default)
                .child(Navigator::new(AppRouter::SettingsAppearance, |r| Probe(r).boxed())),
            HeadlessOptions { size: PhysicalSize::new(390, 900), scale_factor: 1.0 },
        );

        testing::settle(&mut app);
        assert_eq!(tabs_of_store().len(), 5);

        // The "Hot" row of the list.
        testing::tap(&mut app, 1, (200.0, 650.0));
        testing::settle(&mut app);
        assert_eq!(tabs_of_store(), [NavTab::Latest, NavTab::New, NavTab::Search, NavTab::Account]);

        // The list sits where it did, so the same place is the same row.
        testing::tap(&mut app, 2, (200.0, 650.0));
        testing::settle(&mut app);
        assert_eq!(tabs_of_store(), NavTab::ALL);

        // The "Account" row cannot be pressed off.
        testing::tap(&mut app, 3, (200.0, 758.0));
        testing::settle(&mut app);
        assert_eq!(tabs_of_store(), NavTab::ALL);
    }
}
