pub mod components;
pub mod net;
pub mod plugins;

use bevy::math::Vec3;
pub use plugins::*;

pub fn is_nearly_equal(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.0001
}

pub fn is_nearly_equal_f32(a: Option<f32>, b: Option<f32>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => is_nearly_equal(a, b),
        (None, None) => true,
        _ => false,
    }
}

pub fn is_nearly_equal_vec3(a: Option<Vec3>, b: Option<Vec3>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => {
            is_nearly_equal(a.x, b.x) && is_nearly_equal(a.y, b.y) && is_nearly_equal(a.z, b.z)
        }
        (None, None) => true,
        _ => false,
    }
}

/// Takes in two vec3s and returns true if they are nearly equal in the XZ plane
pub fn is_nearly_equal_vec3_but_2d(a: Option<Vec3>, b: Option<Vec3>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => is_nearly_equal(a.x, b.x) && is_nearly_equal(a.z, b.z),
        _ => false,
    }
}
