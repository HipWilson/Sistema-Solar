//! SHADER DE OBJETOS SOLIDOS (nave, motos, arboles, porterias, chocolates).
//! Iluminacion del sol (difusa + especular + borde) con el color de cada material del .mtl.
//! Los materiales "emisivos" (por ejemplo el fuego de los motores) brillan por si mismos
//! y parpadean un poco con el tiempo.
use crate::math::{v3, Vec3};
use crate::renderer::{Fragment, Uniforms};

pub fn shade(f: &Fragment, u: &Uniforms) -> Vec3 {
    let n = f.normal;
    let ambient = v3(0.07, 0.08, 0.12); // luz tenue del espacio
    let diffuse = f.intensity;
    let half = (f.light + f.view).normalize();
    let spec = n.dot(half).max(0.0).powf(28.0) * f.spec * (diffuse > 0.0) as u32 as f32;
    let rim = (1.0 - n.dot(f.view).abs().clamp(0.0, 1.0)).powi(3) * 0.10;

    let mut color = f.color * (ambient + Vec3::splat(0.95 * diffuse)) + Vec3::splat(spec * 0.6) + Vec3::splat(rim);
    if f.emissive > 0.0 {
        let flicker = 1.0 + 0.12 * (u.time * 22.0 + f.pos.x * 9.0).sin();
        color = color * (1.0 - f.emissive) + f.color * (f.emissive * 1.7 * flicker);
    }
    color
}
