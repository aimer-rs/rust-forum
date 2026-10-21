use crate::components::button_reveal::ButtonReveal;
use crate::components::side_bar::SIDE_BAR_ICONS;
use crate::components::side_bar::glass::GlassDecoration;
use crate::components::side_bar::user_menu::sized_avatar;
use crate::preferences::{NavTab, NavbarTabs};
use crate::router::AppRouter;
use crate::store::AppStore;
use crate::theme::ForumTheme;
use aimer::animation::AnimationController;
use aimer::decoration::*;
use aimer::indicator::{TabBar, TabItem, indicator_styles};
use aimer::router::Navigator;
use aimer::style::{BoxShadow, LayoutSpacing, Theme};
use aimer::{
    Align, Alignment, AnyWidget, BoxAlignment, BuildContext, Color, Container, Icons,
    JustifyContent, ProviderContext, Row, StatelessWidget, StaticSvg, Svg, SvgAsset, SvgStyle,
    Widget,
};

/// Windows narrower than this (logical pixels) get the floating tab bar.
pub const MOBILE_BREAKPOINT: f32 = 600.0;

/// Where a tab's icon comes from: the app's own SVG files, or the Lucide set.
#[derive(Clone, Copy)]
pub enum TabIcon {
    Asset(&'static str),
    Lucide(&'static StaticSvg),
    /// The user's avatar, drawn as it is rather than tinted.
    Avatar,
}

/// The icon of a tab.
pub fn icon_of(tab: NavTab) -> TabIcon {
    match tab {
        NavTab::Latest => TabIcon::Asset(SIDE_BAR_ICONS[0]),
        NavTab::New => TabIcon::Asset(SIDE_BAR_ICONS[1]),
        NavTab::Hot => TabIcon::Asset(SIDE_BAR_ICONS[2]),
        NavTab::Search => TabIcon::Lucide(Icons::SEARCH),
        NavTab::Account => TabIcon::Avatar,
    }
}

/// The height of the strip along the bottom of the window the tab bar floats in.
pub const NAVBAR_HEIGHT: f32 = 120.0;
const TAB_WIDTH: f32 = 60.0;
const TAB_HEIGHT: f32 = 52.0;
/// The room the bar keeps round its tabs.
const BAR_PADDING: u32 = 6;
/// How big the bar is: the tabs side by side, and the padding round them.
pub fn bar_width(tabs: usize) -> f32 {
    TAB_WIDTH * 1.1 * tabs as f32 + BAR_PADDING as f32 * 2.0
}
pub const BAR_HEIGHT: f32 = TAB_HEIGHT + BAR_PADDING as f32 * 2.0;
/// How large the bar is when it starts to appear: as little over its size as the
/// room it has in a narrow window allows, as it is nearly as wide as one.
const REVEAL_SCALE_FROM: f32 = 1.1;
const ICON_SIZE: u32 = 24;

/// The sidebar on a narrow window: a floating tab bar, with the account page as
/// its last tab.
///     
/// The tab bar sends the app to Latest, New, Hot, Search or the account page. Its indicator slides to the
/// current tab and lifts into a lens while it moves; it can also be dragged
/// along the bar. On a page that is none of the three it shows no selection. The
/// bar floats above the page, centred along the bottom edge, so the page keeps
/// the whole window instead of losing a column to the sidebar.
///
/// The bar comes in and goes out the way the buttons of the bar over a page do,
/// on the controller it is given (`0.0` out, `1.0` in): see [`ButtonReveal`].
#[derive(Clone, StatelessWidget)]
pub struct MobileNavbar {
    reveal: AnimationController,
}

impl MobileNavbar {
    pub fn new(reveal: AnimationController) -> Self {
        Self { reveal }
    }
}

impl StatelessWidget for MobileNavbar {
    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme = *ForumTheme::of(ctx);
        // let bottom_inset = aimer::safe_area_insets().bottom;
        let navigator = Navigator::<AppRouter>::of(ctx);

        // The tabs the member chose for the bar.
        let tabs = ctx.select::<AppStore, NavbarTabs>(|store| store.navbar.clone());

        // The tab the app is on, or none: the bar is controlled, so it follows
        // the navigator and only proposes where the user would like to go.
        let selection = selected_tab(tabs.tabs(), |route| navigator.contains_route(route));
        let width = bar_width(tabs.len());

        // The bar is given as a function, as it is built afresh on every frame of
        // its coming in and going out.
        let tab_bar = ButtonReveal::new(
            self.reveal.clone(),
            width,
            BAR_HEIGHT,
            BAR_HEIGHT / 2.0,
            move || {
                let navigator = navigator.clone();
                let tabs = tabs.clone();
                let chosen = tabs.tabs().to_vec();
                tab_bar(theme, &chosen, selection, move |index| {
                    let route = tabs.tabs()[index].route();
                    if navigator.current_route() != route {
                        navigator.set_route(route);
                    }
                })
            },
        )
        .scale_from(REVEAL_SCALE_FROM)
        .boxed();

        Align::new().alignment(Alignment::BotCenter).box_child(
            Container::new().height(NAVBAR_HEIGHT).child(
                Row::new()
                    // The container fills the window's width, so the row has to
                    // centre its own contents.
                    .justify_content(JustifyContent::Center)
                    .vertical_alignment(BoxAlignment::Center)
                    .children([tab_bar]),
            ),
        )
    }
}

impl MobileNavbar {
    pub fn style(theme: ForumTheme) -> impl Decoration {
        glass(theme)
    }
}

/// The index of the tab the app is on, given whether each route is showing, or
/// `None` on a page that is none of them.
fn selected_tab(tabs: &[NavTab], is_showing: impl Fn(&AppRouter) -> bool) -> Option<usize> {
    tabs.iter().position(|tab| is_showing(&tab.route()))
}

/// The bar as it looks with `tabs` and `selected` highlighted, for showing what
/// a choice of tabs comes to. It is not for pressing: nothing it shows goes
/// anywhere.
pub fn preview_bar(theme: ForumTheme, tabs: &[NavTab], selected: Option<usize>) -> AnyWidget {
    tab_bar(theme, tabs, selected, |_| {})
}

/// The floating shadow both the tab bar and the avatar circle sit on.
fn floating_shadow() -> BoxShadow {
    BoxShadow::new()
        .color(Color::BLACK.with_opacity(40))
        .blur(18.0)
        .offset_y(6.0)
}

/// How much of the blurred page shows through the glass. The tint is the theme's
/// surface color, so the bar still reads as the same surface in light and dark.
const GLASS_TINT_ALPHA: f32 = 0.45;

/// The frosted glass the tab bar and the avatar circle are both made of: the
/// page behind them blurred, tinted with the surface color, with a light edge.
pub fn glass(theme: ForumTheme) -> impl Decoration {
    GlassDecoration::new(theme.surface_color().with_alpha(0.3))
        .light_edge(LightEdge::new().width(3.0).strength(0.6))
        .lens(
            Magnification::new()
                .zoom(1.4)
                .thickness(32.0)
                .bulge(0.2)
                .style(
                    MagnificationStyle::REFRACTION | MagnificationStyle::BULGE
                ),
        )
        .backdrop(
            BackdropFilter::new()
                .blur_radius(6.0)
                .saturation(0.65)
                .brightness(1.0)
                .contrast(0.9),
        )
}

/// The tabs, themed: frosted glass behind them, the accent color on the
/// selected one, and a soft accent wash for the pill the lens rests as.
fn tab_bar(
    theme: ForumTheme,
    tabs: &[NavTab],
    selection: Option<usize>,
    on_select: impl Fn(usize) + 'static,
) -> AnyWidget {
    TabBar::new()
        .selection(selection)
        .on_select(on_select)
        .item_size(TAB_WIDTH * 1.1, TAB_HEIGHT)
        .bar_padding(LayoutSpacing::all(BAR_PADDING))
        .bar_decoration(glass(theme))
        .inactive_color(theme.on_surface_color())
        .selected_color(theme.primary_color())
        .style(indicator_styles::LIQUID_GLASS.rest_fill(theme.primary_color().with_alpha(0.2)))
        .items(tabs.iter().map(|tab| {
            let icon = icon_of(*tab);
            TabItem::new()
                .label(tab.label())
                .icon(move |tint| tab_icon(theme, icon, tint))
        }))
        .boxed()
}

/// A tab's icon, drawn in `tint`.
pub fn tab_icon(theme: ForumTheme, icon: TabIcon, tint: Color) -> AnyWidget {
    match icon {
        TabIcon::Avatar => sized_avatar(theme, ICON_SIZE as f32).boxed(),
        TabIcon::Asset(svg_path) => SvgAsset::new(svg_path)
            .width(ICON_SIZE)
            .height(ICON_SIZE)
            .style(".icon-path", SvgStyle::new().stroke(tint))
            .style(".icon-path-fill", SvgStyle::new().fill(tint))
            .boxed(),
        TabIcon::Lucide(icon) => Svg::new(icon)
            .style("path", SvgStyle::new().stroke(tint))
            .width(ICON_SIZE)
            .height(ICON_SIZE)
            .boxed(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aimer::{router::Route, share::ShareRef};

    #[test]
    fn each_tab_route_selects_its_own_tab() {
        let tabs = NavTab::ALL;
        for (index, tab) in tabs.iter().enumerate() {
            let route = tab.route();
            assert_eq!(selected_tab(&tabs, |candidate| *candidate == route), Some(index));
        }
    }

    #[test]
    fn a_tab_taken_off_the_bar_selects_nothing_and_the_others_keep_their_place() {
        let tabs = NavbarTabs::default().toggled(NavTab::New);
        assert_eq!(selected_tab(tabs.tabs(), |route| *route == AppRouter::New), None);
        assert_eq!(selected_tab(tabs.tabs(), |route| *route == AppRouter::Hot), Some(1));
    }

    #[test]
    fn a_page_that_is_none_of_the_tabs_selects_nothing() {
        let other = [
            AppRouter::Settings,
            AppRouter::CategoryTopics {
                slug: ShareRef::from_static("rust"),
                id: 12,
            },
            AppRouter::PostDetail {
                slug: ShareRef::from_static("hello"),
                id: 345,
            },
        ];
        for route in &other {
            assert_eq!(
                selected_tab(&NavTab::ALL, |candidate| candidate == route),
                None,
                "{}",
                route.format()
            );
        }
    }

    #[test]
    fn the_tabs_are_the_places_the_sidebar_lists_with_one_icon_each() {
        let labels: Vec<_> = NavTab::ALL.iter().map(|tab| tab.label()).collect();
        assert_eq!(labels, ["Latest", "New", "Hot", "Search", "Account"]);
        for tab in NavTab::ALL {
            if let TabIcon::Asset(icon) = icon_of(tab) {
                assert!(SIDE_BAR_ICONS.contains(&icon), "{icon}");
            }
        }
    }

    #[test]
    fn the_bar_is_as_wide_as_its_tabs() {
        assert!(bar_width(3) < bar_width(5));
        assert_eq!(bar_width(0), BAR_PADDING as f32 * 2.0);
    }
}
