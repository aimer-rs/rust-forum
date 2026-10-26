use crate::router::AppRouter;
use crate::theme::ForumTheme;
use aimer::router::Navigator;
use aimer::style::{BoxDecoration, LayoutSpacing, TextStyle, Theme};
use aimer::{
    BoxAlignment, BuildContext, Color, Container, Dimension, Icons, Row, StatelessWidget, Svg, SvgAsset, SvgStyle, Text, Widget, case,
};
use aimer::input::button::Button;

#[derive(StatelessWidget)]
pub struct NavigationRow {
    label: &'static str,
    route: AppRouter,
    svg_path: &'static str,
}

impl NavigationRow {
    pub fn new(label: &'static str, router: AppRouter, svg_path: &'static str) -> Self {
        Self {
            label,
            route: router,
            svg_path,
        }
    }
}

impl StatelessWidget for NavigationRow {
    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let navigator = Navigator::<AppRouter>::of(ctx);
        let route = self.route.clone();
        let theme = ForumTheme::of(ctx);
        navigator.contains_route(&self.route);

        let should_activate = navigator.contains_route(&route);
        let can_route = navigator.current_route() != route;
        let box_decoration = BoxDecoration::new()
            .background_color(case!(
                should_activate,
                theme.primary_color().with_alpha(0.2),
                Color::Transparent
            ))
            .border_radius(8);

        let hover_decor = box_decoration.clone().background_color(case!(
            should_activate,
            theme.primary_color().with_alpha(0.2),
            theme.primary_color().with_alpha(0.1)
        ));

        Button::new()
            .hover_decoration(hover_decor)
            .decoration(box_decoration)
            .on_press(move || {
                if can_route {
                    navigator.set_route(route.clone());
                }
            })
            .child(
                Container::new()
                    .width(Dimension::Percent(100f32))
                    .padding(LayoutSpacing::vertical(6).left(18))
                    .child(
                        Row::new()
                            .vertical_alignment(BoxAlignment::Center)
                            .gaps(LayoutSpacing::all(10))
                            .children([
                                SvgAsset::new(self.svg_path)
                                    .width(24)
                                    .height(24)
                                    .style(
                                        ".icon-path",
                                        SvgStyle::new().stroke(theme.primary_color()),
                                    )
                                    .style(
                                        ".icon-path-fill",
                                        SvgStyle::new().fill(theme.primary_color()),
                                    )
                                    .boxed(),
                                Text::new(self.label)
                                    .text_style(TextStyle::new().font_size(16).color(
                                        if should_activate {
                                            theme.font_color
                                        } else {
                                            theme.on_surface_color()
                                        },
                                    ))
                                    .boxed(),
                            ]),
                    ),
            )
    }
}

/// The row that opens the search page: a magnifier and "Search", lit while the
/// search page is the one showing.
#[derive(StatelessWidget)]
pub struct SearchRow;

impl StatelessWidget for SearchRow {
    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let navigator = Navigator::<AppRouter>::of(ctx);
        let theme = ForumTheme::of(ctx);
        let active = navigator.current_route() == AppRouter::Search;
        let decoration = BoxDecoration::new()
            .background_color(case!(
                active,
                theme.primary_color().with_alpha(0.2),
                Color::Transparent
            ))
            .border_radius(8);
        let hover = decoration.clone().background_color(case!(
            active,
            theme.primary_color().with_alpha(0.2),
            theme.primary_color().with_alpha(0.1)
        ));

        Button::new()
            .hover_decoration(hover)
            .decoration(decoration)
            .on_press(move || {
                if !active {
                    navigator.set_route(AppRouter::Search);
                }
            })
            .child(
                Container::new()
                    .width(Dimension::Percent(100f32))
                    .padding(LayoutSpacing::vertical(6).left(18))
                    .child(
                        Row::new()
                            .vertical_alignment(BoxAlignment::Center)
                            .gaps(LayoutSpacing::all(10))
                            .children([
                                Svg::new(Icons::SEARCH)
                                    .style("path", SvgStyle::new().stroke(theme.primary_color()))
                                    .width(24)
                                    .height(24)
                                    .boxed(),
                                Text::new("Search")
                                    .text_style(TextStyle::new().font_size(16).color(
                                        if active { theme.font_color } else { theme.on_surface_color() },
                                    ))
                                    .boxed(),
                            ]),
                    ),
            )
    }
}
