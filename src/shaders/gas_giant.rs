//! SHADER 3: GIGANTE GASEOSO "Nebulosa Magna"
//! ==========================================
//! Bandas crema, rosa y violeta con una gran tormenta turquesa.
//!   1. VORTICE: antes de dibujar las bandas, las coordenadas (longitud, latitud)
//!      cerca de la tormenta se GIRAN un angulo mayor en el centro. Asi las bandas
//!      se enroscan (no es un circulo pintado encima, es una distorsion).
//!   2. EFECTO LIQUIDO: las coordenadas se deforman con ruido dos veces
//!      ("domain warping") y cada latitud se desplaza distinto con el tiempo.
//!   3. BANDAS: ondas seno en la latitud (dos frecuencias) + ruido.
//!   4. COLOR: rampa de 6 colores + polos con bruma + color propio de la tormenta.
//!   5. ILUMINACION: difusa suave + brillo atmosferico (fresnel) en el borde.
//!
//! EFECTO ADICIONAL: Tormenta o vortice localizado.
use crate::col;
use crate::math::{v3, Vec3};
use crate::noise::{fbm, noise_vec3};
use crate::renderer::{Fragment, Uniforms};
use crate::util::{mix, ramp, rotate_y, smoothstep};
use std::f32::consts::FRAC_PI_2;

// ---------------- PARAMETROS QUE SE PUEDEN MODIFICAR ----------------
const BAND_FREQ: f32 = 13.0;   // cantidad de bandas (mas alto = mas bandas)
const FLOW_AMOUNT: f32 = 0.30; // cuanto fluyen/ondulan las bandas (radianes)
const LIQUID_WARP: f32 = 0.22; // deformacion tipo liquido
const STORM_LON: f32 = 0.9;    // longitud de la tormenta (radianes)
const STORM_LAT: f32 = -0.32;  // latitud de la tormenta (radianes)
const STORM_SIZE: (f32, f32) = (0.55, 0.30); // tamano (ancho, alto)
const STORM_SWIRL: f32 = 5.5;  // cuanto se enroscan las bandas
const ANIM_SPEED: f32 = 1.0;

const BAND_COLORS: [Vec3; 6] = [
    col!(244, 226, 196), // crema
    col!(236, 176, 150), // salmon
    col!(196, 96, 128),  // rosa oscuro
    col!(112, 60, 128),  // violeta
    col!(222, 190, 204), // lila claro
    col!(250, 232, 214), // crema brillante
];
const POLE_COLOR: Vec3 = col!(90, 110, 170);   // bruma azul en los polos
const STORM_COLOR: Vec3 = col!(60, 200, 190);  // tormenta turquesa
const STORM_CORE: Vec3 = col!(240, 250, 240);  // ojo de la tormenta
// --------------------------------------------------------------------

/// Lleva un angulo al rango [-pi, pi] (para que la longitud no tenga salto).
fn wrap_angle(a: f32) -> f32 {
    a.sin().atan2(a.cos())
}

pub fn shade(f: &Fragment, u: &Uniforms) -> Vec3 {
    let p = f.normal;
    let ts = if u.animate { u.time * ANIM_SPEED } else { 0.0 };

    let lon = p.z.atan2(p.x);
    let lat = p.y.clamp(-1.0, 1.0).asin();

    // ---------- 1. VORTICE: giramos (lon, lat) alrededor de la tormenta ----------
    let mut storm_mask = 0.0;
    let mut ring = 0.0;
    let mut p2 = p;
    if u.storm {
        let cos_lat = lat.cos().max(0.25);
        let dx = wrap_angle(lon - STORM_LON) * cos_lat / STORM_SIZE.0; // distancia normalizada
        let dy = (lat - STORM_LAT) / STORM_SIZE.1;
        let r2 = dx * dx + dy * dy; // elipse
        let falloff = (-r2 * 1.1).exp(); // 1 en el centro, 0 lejos
        let breathe = 1.0 + 0.12 * (ts * 0.5).sin();
        let theta = STORM_SWIRL * falloff * breathe; // angulo de giro
        let (s, c) = theta.sin_cos();
        let (rx, ry) = (dx * c - dy * s, dx * s + dy * c); // rotacion 2D
        let new_lon = STORM_LON + rx * STORM_SIZE.0 / cos_lat;
        let new_lat = STORM_LAT + ry * STORM_SIZE.1;
        // Volvemos a convertir (lon, lat) a un punto 3D
        p2 = v3(new_lat.cos() * new_lon.cos(), new_lat.sin(), new_lat.cos() * new_lon.sin());
        storm_mask = (-r2 * 2.2).exp();
        let d = (r2.sqrt() - 1.0) / 0.22;
        ring = (-d * d).exp();
    }

    let lat2 = p2.y.clamp(-1.0, 1.0).asin();

    // ---------- 2. EFECTO LIQUIDO ----------
    // Cada latitud se desplaza distinto: las bandas "resbalan" unas sobre otras.
    let flow_angle = FLOW_AMOUNT * (ts * 0.35 + lat2 * 5.0).sin();
    let mut q = rotate_y(p2, flow_angle);
    // Doble deformacion de dominio (ruido que deforma al ruido)
    let drift = v3(ts * 0.06, ts * 0.02, -ts * 0.04);
    q = q + noise_vec3(q * 2.0 + drift, 14) * LIQUID_WARP;
    let turb = fbm(q * 2.4 + drift, 4, 2);
    let fine = fbm(q * 7.0 - drift * 2.0, 3, 6);

    // ---------- 3. BANDAS ----------
    let y = lat2 / FRAC_PI_2; // latitud de -1 a 1
    let wave1 = 0.5 + 0.5 * (y * BAND_FREQ + 4.0 * (turb - 0.5)).sin();
    let wave2 = 0.5 + 0.5 * (y * BAND_FREQ * 1.9 + 1.7 + 2.5 * (turb - 0.5)).sin();
    let value = 0.62 * wave1 + 0.20 * wave2 + 0.18 * fine;
    let mut color = ramp(value, &BAND_COLORS);

    // ---------- 4. POLOS y TORMENTA ----------
    let pole = smoothstep(0.68, 0.98, p.y.abs());
    color = mix(color, POLE_COLOR * (0.8 + 0.4 * fine), pole * 0.75);

    if u.storm {
        let swirl_tex = 0.55 + 0.45 * fine; // espirales (vienen del giro)
        color = mix(color, STORM_COLOR * swirl_tex, smoothstep(0.08, 0.75, storm_mask) * 0.9);
        color = mix(color, STORM_CORE, smoothstep(0.78, 1.0, storm_mask) * 0.7);
        color = color + col!(255, 240, 220) * (0.18 * ring);
    }

    // ---------- 5. ILUMINACION ----------
    let ndl = f.normal.dot(f.light);
    let diffuse = smoothstep(-0.12, 1.0, ndl); // terminador suave
    color = color * (0.07 + 0.93 * diffuse);

    let mu = f.normal.dot(f.view).clamp(0.0, 1.0);
    let rim = (1.0 - mu).powi(3) * (0.2 + 0.8 * diffuse); // brillo atmosferico del borde
    color = color + col!(255, 170, 200) * (rim * 0.55);
    color * (0.75 + 0.25 * mu.sqrt()) // leve oscurecimiento del borde
}
