use nalgebra::{Matrix3, Vector2};
use std::ops::Mul;

pub type Vector = Vector2<f32>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transformation(Matrix3<f32>);

impl Transformation {
    pub fn identity() -> Transformation {
        Transformation(Matrix3::identity())
    }

    #[rustfmt::skip]
    pub fn orthographic(width: f32, height: f32) -> Transformation {
        Transformation(Matrix3::new(
            2.0 / width, 0.0         , -1.0,
            0.0,         2.0 / height, -1.0,
            0.0,         0.0         , 1.0
        ))
    }

    pub fn translate(translation: Vector) -> Transformation {
        Transformation(Matrix3::new_translation(&translation))
    }

    pub fn scale(scale: f32) -> Transformation {
        Transformation(Matrix3::new_scaling(scale))
    }
}

impl Mul for Transformation {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Transformation(self.0 * rhs.0)
    }
}

impl From<Transformation> for [[f32; 4]; 4] {
    #[rustfmt::skip]
    fn from(t: Transformation) -> Self {
        [
            [t.0[0], t.0[1], 0.0, t.0[2]],
            [t.0[3], t.0[4], 0.0, t.0[5]],
            [   0.0,   -1.0, 0.0,    0.0],
            [t.0[6], t.0[7], 0.0, t.0[8]],
        ]
    }
}

impl From<Transformation> for [f32; 16] {
    #[rustfmt::skip]
    fn from(t: Transformation) -> Self {
        [
            t.0[0], t.0[1], 0.0, t.0[2],
            t.0[3], t.0[4], 0.0, t.0[5],
               0.0,   -1.0, 0.0,    0.0,
            t.0[6], t.0[7], 0.0, t.0[8],
        ]
    }
}
