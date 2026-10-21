//! The frosted-glass surface the floating mobile bar sits on.

use aimer::canvas::{Canvas, Rect};
use aimer::decoration::{BackdropFilter, BoxDecoration, BoxShadow, Decoration, LightEdge, Magnification};
use aimer::{Color, ResolvedSize, Vec2d};

/// A pill of frosted glass: the page behind it blurred, a translucent tint over
/// that, a soft shadow around it, and a light edge along its rim.
///
/// The backdrop filter only knows rectangles, so the blur is painted inside a
/// clip with the pill's corners; without it the blurred square would show past
/// the rounded tint. The corners are always half the shorter side, so the same
/// decoration shapes the wide tab bar and the round avatar button.
///
/// The layers are painted in the order they are read: blur, tint with shadow,
/// then the light edge, which lies over the tint and so keeps its contrast
/// against it.
pub struct GlassDecoration {
    tint: Color,
    shadow: Option<BoxShadow>,
    filter: BackdropFilter,
    edge: LightEdge,
    lens: Magnification
}

impl GlassDecoration {
    /// Glass tinted with `tint`, whose alpha says how much of the blurred page
    /// shows through.
    pub fn new(tint: Color) -> Self {
        Self {
            tint,
            shadow: None,
            filter: BackdropFilter::new().blur_radius(18.0).saturation(1.5),
            edge: LightEdge::new().width(6.0).strength(0.7),
            lens: Magnification::new()
        }
    }

    /// Casts `shadow` around the glass.
    pub fn shadow(mut self, shadow: BoxShadow) -> Self {
        self.shadow = Some(shadow);
        self
    }

    pub fn lens(mut self, lens: Magnification ) -> Self {
        self.lens = lens;
        self
    }

    pub fn backdrop(mut self, backdrop: BackdropFilter) -> Self {
        self.filter = backdrop;
        self
    }

    pub fn light_edge(mut self, edge: LightEdge) -> Self {
        self.edge = edge;
        self
    }

    /// The tint and shadow layer, shaped as a pill of `radius`.
    fn body(&self, radius: f32) -> BoxDecoration {
        let body = BoxDecoration::new()
            .background_color(self.tint)
            .border_radius(radius);
        match self.shadow {
            Some(shadow) => body.add_shadow(shadow),
            None => body,
        }
    }
}

impl Decoration for GlassDecoration {
    fn paint(&self, canvas: &Canvas, origin: Vec2d, size: ResolvedSize, scale: f32) {
        if size.width <= 0.0 || size.height <= 0.0 {
            return;
        }
        let radius = size.width.min(size.height) * 0.5;

        canvas.push_clip(
            Rect::new(origin.x, origin.y, size.width, size.height),
            [radius; 4],
        );
         self.body(radius).paint_layer(canvas, origin, size, scale);
        self.filter.paint(canvas, origin, size, scale);
        self.lens.paint(canvas, origin, size, scale);
        canvas.pop_clip();

       
        self.edge
            .paint_layer(canvas, origin, size, scale);
    }

    fn paint_outsets(&self, size: ResolvedSize, scale: f32) -> [f32; 4] {
        self.body(size.width.min(size.height) * 0.5)
            .paint_outsets(size, scale)
    }

    /// The tint, so the glass stops scroll events as the opaque bar did.
    fn background_color(&self) -> Option<Color> {
        Some(self.tint)
    }
}
