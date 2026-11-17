//! Pequenas funciones de ayuda para los shaders.
use crate::math::Vec3;

/// Crea un color a partir de valores 0..255 (se usa en constantes).
#[macro_export]
macro_rules! col {
    ($r:expr, $g:expr, $b:expr) => {
        $crate::math::Vec3 {
            x: $r as f32 / 255.0,
            y: $g as f32 / 255.0,
            z: $b as f32 / 255.0,
        }
    };
}

/// 0 si x<e0, 1 si x>e1 y una transicion suave en medio.
pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0 + 1e-8)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Mezcla de dos colores.
pub fn mix(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    a + (b - a) * t
}

/// Rampa de colores: t en [0,1] recorre la lista interpolando entre colores.
pub fn ramp(t: f32, colors: &[Vec3]) -> Vec3 {
    let n = colors.len() - 1;
    let t = t.clamp(0.0, 1.0) * n as f32;
    let i = (t as usize).min(n - 1);
    mix(colors[i], colors[i + 1], t - i as f32)
}

/// Rota un punto alrededor del eje Y.
pub fn rotate_y(p: Vec3, angle: f32) -> Vec3 {
    let (s, c) = angle.sin_cos();
    Vec3 {
        x: p.x * c + p.z * s,
        y: p.y,
        z: -p.x * s + p.z * c,
    }
}
