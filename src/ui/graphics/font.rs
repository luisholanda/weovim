use super::transform::Transformation;
use super::Point;
use crate::ui::text::Text;
use wgpu_glyph::{
    BuiltInLineBreaker, GlyphBrush, GlyphCruncher, Layout, Scale, Section, SectionText,
    VariedSection,
};

pub struct Font<'f> {
    brusher: GlyphBrush<'f, ()>,
    layout: Layout<BuiltInLineBreaker>,
}

impl<'f> Font<'f> {
    /// Creates a new [Font] using the raw bytes of the desired font.
    pub fn from_bytes(device: &mut wgpu::Device, font_bytes: &'f [u8]) -> Self {
        Self {
            brusher: wgpu_glyph::GlyphBrushBuilder::using_font_bytes(font_bytes)
                .texture_filter_method(wgpu::FilterMode::Nearest)
                .build(device, wgpu::TextureFormat::Bgra8UnormSrgb),
            layout: Layout::default_single_line()
                .h_align(wgpu_glyph::HorizontalAlign::Center)
                .v_align(wgpu_glyph::VerticalAlign::Center),
        }
    }

    /// Enqueue many [Text]s to be drawn.
    pub fn add(&mut self, position: Point, text: &Text) -> Option<(Point, Point)> {
        let font_id = wgpu_glyph::FontId::default();

        let section = VariedSection {
            screen_position: (position.x, position.y),
            text: vec![SectionText {
                text: &text.content,
                scale: Scale::uniform(text.size),
                color: text.color.into_raw_components(),
                font_id,
            }],
            layout: self.layout,
            ..Default::default()
        };

        let bounds = self.brusher.glyph_bounds(&section);

        self.brusher.queue(section);

        bounds.map(|b| (Point::new(b.min.x, b.min.y), Point::new(b.max.x, b.max.y)))
    }

    /// Draw the enqueued texts into the target texture.
    pub fn draw(
        &mut self,
        device: &mut wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        transform: Transformation,
    ) {
        self.brusher
            .draw_queued_with_transform(device, encoder, target, transform.into())
            .expect("Failed to draw text");
    }
}
