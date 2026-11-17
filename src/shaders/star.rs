//! SHADER 1: ESTRELLA "Helios"
//! ===========================
//! La estrella emite su propia luz, por eso NO usa la iluminacion externa.
//! Su color depende de una "temperatura" (`heat`) calculada con ruido:
//!   1. Deformacion (warp): ruido que cambia con el tiempo -> la superficie "hierve".
//!   2. Zonas grandes de temperatura (fbm de baja frecuencia).
//!   3. Granulacion: abs(ruido) vale ~0 donde el ruido cruza el cero; eso dibuja
//!      un panal de bordes oscuros alrededor de celdas brillantes.
//!   4. Manchas solares: donde otro ruido supera un umbral, baja la temperatura.
//!   5. Erupciones: zonas muy calientes que se mueven con el tiempo.
//! Al final `heat` se convierte en color con una rampa y se oscurece el borde.
//!
//! EFECTO ADICIONAL: Material animado.
use crate::col;
use crate::math::{v3, Vec3};
use crate::noise::{fbm, noise3, noise_vec3};
use crate::renderer::{Fragment, Uniforms};
use crate::util::{mix, ramp, smoothstep};

// ---------------- PARAMETROS QUE SE PUEDEN MODIFICAR ----------------
const WARP_AMOUNT: f32 = 0.35;     // cuanto se deforma la superficie
const GRAN_FREQ: f32 = 11.0;       // tamano de las celdas (mas alto = mas chicas)
const SPOT_THRESHOLD: f32 = 0.67;  // umbral de manchas (mas bajo = mas manchas)
const FLARE_THRESHOLD: f32 = 0.68; // umbral de erupciones (mas bajo = mas erupciones)
const ANIM_SPEED: f32 = 1.0;       // velocidad de la animacion
const PULSE: f32 = 0.04;           // cuanto "late" la estrella

// Rampa de colores segun la temperatura (de fria a caliente)
const STAR_COLORS: [Vec3; 6] = [
    col!(40, 2, 0),      // mancha oscura
    col!(160, 20, 5),    // rojo
    col!(240, 90, 10),   // naranja
    col!(255, 170, 30),  // amarillo-naranja
    col!(255, 225, 120), // amarillo claro
    col!(255, 250, 230), // blanco caliente
];
// --------------------------------------------------------------------

pub fn shade(f: &Fragment, u: &Uniforms) -> Vec3 {
    let p = f.normal; // punto sobre la esfera unitaria
    let ts = if u.animate { u.time * ANIM_SPEED } else { 0.0 };

    // 1. Deformacion que cambia con el tiempo
    let warp = noise_vec3(p * 1.8 + v3(0.0, ts * 0.12, ts * 0.05), 3);
    let q = p + warp * WARP_AMOUNT;

    // 2. Zonas grandes de temperatura
    let large = fbm(q * 2.5 + v3(ts * 0.05, 0.0, 0.0), 4, 1);

    // 3. Granulacion (celdas de conveccion que se mueven)
    let g = noise3(q * GRAN_FREQ + v3(0.0, -ts * 0.35, ts * 0.2), 5);
    let cells = smoothstep(0.0, 0.55, g.abs()); // 0 en los bordes, 1 en el centro
    let detail = fbm(q * 16.0 + v3(ts * 0.3, 0.0, -ts * 0.2), 2, 9);

    let mut heat = 0.50 * large + 0.20 * cells + 0.16 * detail + 0.08;

    // 4. Manchas solares (casi fijas, derivan muy lento)
    let spots = fbm(p * 2.0 + v3(5.0, 1.0 + ts * 0.01, 3.0), 3, 21);
    let penumbra = smoothstep(SPOT_THRESHOLD - 0.05, SPOT_THRESHOLD + 0.03, spots);
    let umbra = smoothstep(SPOT_THRESHOLD + 0.01, SPOT_THRESHOLD + 0.07, spots);
    heat = heat - 0.28 * penumbra - 0.35 * umbra;

    // 5. Erupciones brillantes que aparecen y se mueven
    let flares = fbm(q * 3.0 + v3(ts * 0.15, ts * 0.1, 7.0), 3, 33);
    heat += 0.45 * smoothstep(FLARE_THRESHOLD, FLARE_THRESHOLD + 0.08, flares);

    // Pulso suave de toda la estrella
    heat *= 1.0 + PULSE * (ts * 1.3).sin();

    // Temperatura -> color
    let color = ramp(heat * 1.25, &STAR_COLORS);

    // Oscurecimiento del borde (limb darkening): mas rojizo y oscuro
    let mu = f.normal.dot(f.view).clamp(0.0, 1.0);
    let limb = 0.30 + 0.70 * mu.powf(0.55);
    let edge_tint = v3(1.0, 0.55, 0.25);
    let color = mix(color * edge_tint, color, smoothstep(0.0, 0.6, mu));
    color * limb * 1.15
}
