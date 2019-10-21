use super::graphics::Point;
use super::Color;
use std::f32;

#[derive(Debug)]
pub struct Text<'a> {
    /// The content of the text.
    pub content: &'a str,
    /// The position of the text.
    pub position: Point,
    /// Size of the text.
    pub size: f32,
    /// Color of the text.
    pub color: Color,
    /// Background of the text.
    pub background: Color,
}
