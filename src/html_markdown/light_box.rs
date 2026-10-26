use aimer::input::mouse_region::MouseRegion;
use aimer::screen::MediaQuery;
use aimer::style::{LayoutSpacing, TextAlign, TextStyle};
use aimer::{
    Align, Alignment, AnyWidget, BuildContext, Color, Container, MarkdownCustomBlock,
    MarkdownCustomBlockInput, MarkdownError, NetworkImage, Opacity, Stack, State,
    StateUpdater, StatefulWidget, Text, Widget,
};
use serde::{Deserialize, Serialize};
use std::rc::Rc;

#[derive(StatefulWidget, Deserialize, Serialize)]
pub struct LightBox {
    image_url: Rc<str>,
    alt: Rc<str>,
    width: u32,
    height: u32,
}

impl LightBox {
    pub fn new(image_url: Rc<str>, alt: Rc<str>, width: u32, height: u32) -> LightBox {
        Self {
            image_url,
            alt,
            width,
            height,
        }
    }

    // pub fn from_md_data(item: &str)
}

pub struct LightBoxState {
    image: Rc<str>,
    alt: Rc<str>,
    is_hover: bool,
    width: u32,
    height: u32,
    updater: StateUpdater<Self>,
}

impl StatefulWidget for LightBox {
    type State = LightBoxState;

    fn create_state(self) -> Self::State {
        LightBoxState {
            image: self.image_url,
            alt: self.alt,
            is_hover: false,
            width: self.width,
            height: self.height,
            updater: StateUpdater::new(),
        }
    }
}

impl State<LightBox> for LightBoxState {
    fn init_state(&mut self, updater: StateUpdater<Self>)
    where
        Self: Sized,
    {
        self.updater = updater;
    }

    fn adopt_config_from(&mut self, new: Self)
    where
        Self: Sized,
    {
        self.image = new.image;
        self.alt = new.alt;
    }

    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let updater = self.updater.clone();
        let window_size = MediaQuery::of(ctx);
        let image_width_ratio =
            window_size.size.width / self.width as f32 / window_size.scale_factor * 1.3;
        MouseRegion::new()
            .on_hover_enter(move || updater.set_state(|state| state.is_hover = true))
            .on_hover_exit(move || updater.set_state(|state| state.is_hover = false))
            .child(
                Container::new()
                    .height(self.height as f32 * image_width_ratio)
                    .width(self.width as f32 * image_width_ratio)
                    .child(
                        Stack::new().children([
                            NetworkImage::new(self.image.to_string()).boxed(),
                            Align::new().alignment(Alignment::BotLeft).box_child(
                                Opacity::new()
                                    .opacity(if self.is_hover { 1.0 } else { 0.0 })
                                    .child(
                                        Container::new()
                                            .padding(LayoutSpacing::new().left(12))
                                            .color(Color::BLACK.with_alpha(0.8))
                                            .height(40)
                                            .child(
                                                Text::new(self.alt.clone())
                                                    .text_align(TextAlign::MidLeft)
                                                    .text_style(
                                                        TextStyle::new()
                                                            .font_size(16)
                                                            .color(Color::WHITE),
                                                    ),
                                            ),
                                    ),
                            ),
                        ]),
                    ),
            )
    }
}

impl MarkdownCustomBlock for LightBox {
    const NAME: &'static str = "forum-lightbox-wrapper";
    const OPENING: &'static str = ":::forum-lightbox-wrapper";
    const CLOSING: &'static str = ":::";
    type Props = Self;

    fn parse(input: MarkdownCustomBlockInput<'_>) -> Result<Self::Props, MarkdownError> {
        let data: Self::Props =
            serde_json::from_str(input.raw).map_err(|e| MarkdownError::new(e.to_string()))?;
        Ok(data)
    }

    fn build(props: &Self::Props, ctx: &BuildContext) -> AnyWidget {
        LightBox::new(
            props.image_url.clone(),
            props.alt.clone(),
            props.width,
            props.height,
        )
        .boxed()
    }
}
