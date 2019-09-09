#[derive(Debug, Copy, Clone)]
pub struct Color([f64; 3]);

impl Color {
    #[inline(always)]
    pub fn r(self) -> f64 {
        self.0[0]
    }

    #[inline(always)]
    pub fn g(self) -> f64 {
        self.0[1]
    }

    #[inline(always)]
    pub fn b(self) -> f64 {
        self.0[2]
    }

    pub fn from_u64(v: u64) -> Self {
        Color([
            ((v >> 16) & 255) as f64 / 255f64,
            ((v >> 8) & 255) as f64 / 255f64,
            (v & 255) as f64 / 255f64,
        ])
    }
}
