use crate::components::side_bar::parse_hex_color;
use crate::models::categories::Category;
use crate::router::AppRouter;
use crate::theme::ForumTheme;
use aimer::input::button::Button;
use aimer::router::NavigatorController;
use aimer::share::{ShareRef, Shared};
use aimer::style::{BoxDecoration, LayoutSpacing, TextStyle, Theme};
use aimer::{
    BoxAlignment, BuildContext, Color, Container, Dimension, Row, SizedBox, StatelessWidget, Text,
    Widget, ZeroSizedBox,
};

#[derive(StatelessWidget)]
pub struct CategoryRow {
    pub(crate) category: ShareRef<Category>,
}

impl CategoryRow {
    pub fn new(category: &ShareRef<Category>) -> Self {
        Self {
            category: category.clone(),
        }
    }
}

impl StatelessWidget for CategoryRow {
    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let category = &self.category;
        let navigator = NavigatorController::of(ctx);
        let theme = ForumTheme::of(ctx);
        let slug = category.slug.clone();
        let id = category.id;
        let category_name = category.clone().project(|item| item.name.as_str());

        let target_route = AppRouter::CategoryTopics {
            slug: Shared::new(slug).project(|item| item.as_str()),
            id,
        };

        let should_activate = navigator.contains_route(&target_route);
        let can_route = navigator.current_route() != target_route;
        let box_decoration =
            BoxDecoration::new()
                .border_radius(8)
                .background_color(if should_activate {
                    theme.primary_color().with_alpha(0.2)
                } else {
                    Color::Transparent
                });

        let hover_decor = box_decoration.clone().background_color(if should_activate {
            theme.primary_color().with_alpha(0.2)
        } else {
            theme.primary_color().with_alpha(0.1)
        });
        Button::new()
            .hover_decoration(hover_decor)
            .decoration(box_decoration)
            .on_press(move || {
                if can_route {
                    navigator.set_route(target_route.clone());
                }
            })
            .child(
                Container::new()
                    .width(Dimension::Percent(100f32))
                    .padding(LayoutSpacing::vertical(8))
                    .child(
                    Row::new()
                        .vertical_alignment(BoxAlignment::Center)
                        .gaps(LayoutSpacing::all(10))
                        .children([
                            SizedBox::new().width(4).boxed(),
                            Container::new()
                                .width(Dimension::Px(12.0))
                                .height(Dimension::Px(12.0))
                                .box_decoration(
                                    BoxDecoration::new()
                                        .background_color(parse_hex_color(&category.color))
                                        .border_radius(6),
                                )
                                .child(ZeroSizedBox)
                                .boxed(),
                            Text::new(category_name)
                                .text_style(TextStyle::new().font_size(16).color(
                                    if should_activate {
                                        theme.font_color
                                    } else {
                                        theme.on_surface_color()
                                    },
                                ))
                                .boxed(),
                            SizedBox::new().width(4).boxed(),
                        ]),
                ),
            )
    }
}
