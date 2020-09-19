const RGBA_MAX_F32: f32 = 255.0;

/// A color in the sRGB color space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    /// The red color component.
    pub r: f32,
    /// The green color component.
    pub g: f32,
    /// The blue color component.
    pub b: f32,
    /// The alpha color component.
    pub a: f32,
}

impl From<wgpu::Color> for Color {
    fn from(color: wgpu::Color) -> Self {
        Self {
            r: color.r as f32,
            g: color.g as f32,
            b: color.b as f32,
            a: color.a as f32,
        }
    }
}

impl From<Color> for wgpu::Color {
    fn from(color: Color) -> Self {
        Self {
            r: color.r as f64,
            g: color.g as f64,
            b: color.b as f64,
            a: color.a as f64,
        }
    }
}

impl Color {
    /// The black color.
    pub const BLACK: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };

    /// The White color.
    pub const WHITE: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };

    /// Converts an array with the raw color components [red, green, blue, alpha]
    /// into a [Color].
    pub const fn from_raw_components(components: [f32; 4]) -> Self {
        Self {
            r: components[0],
            g: components[1],
            b: components[2],
            a: components[3],
        }
    }

    /// Converts a [Color] into its raw components.
    pub const fn into_raw_components(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

/// RGB color space transformations.
impl Color {
    /// Create a [Color] from the raw RGBA channels.
    pub const fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / RGBA_MAX_F32,
            g: g as f32 / RGBA_MAX_F32,
            b: b as f32 / RGBA_MAX_F32,
            a: a as f32 / RGBA_MAX_F32,
        }
    }

    /// Create a [Color] from a RGBA representation (0xRRGGBBAA).
    pub const fn from_rgba_u64(color: u64) -> Self {
        let r = ((color & 0xFF_00_00_00) >> 24) as u8;
        let g = ((color & 0x00_FF_00_00) >> 16) as u8;
        let b = ((color & 0x00_00_FF_00) >> 8) as u8;
        let a = (color & 0x00_00_00_FF) as u8;

        Self::from_rgba(r, g, b, a)
    }

    /// Converts a color to its RGBA channels.
    pub fn to_rgba(self) -> [u8; 4] {
        [
            (self.r * RGBA_MAX_F32).round() as u8,
            (self.g * RGBA_MAX_F32).round() as u8,
            (self.b * RGBA_MAX_F32).round() as u8,
            (self.a * RGBA_MAX_F32).round() as u8,
        ]
    }

    /// Create a [Color] from the raw RGB channels.
    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self::from_rgba(r, g, b, RGBA_MAX_F32 as u8)
    }

    /// Create a [Color] from the raw RGB representation (0xRRGGBB).
    pub const fn from_rgb_u64(color: u64) -> Self {
        Self::from_rgba_u64(color << 8 | 0xFF)
    }
}
