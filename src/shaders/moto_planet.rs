//! SHADER: PLANETA DE MOTOCICLETAS "Motoplaneta"
//! =============================================
//! Un planeta cubierto de autopistas: meridianos y paralelos forman una red de carreteras.
//!   1. Carreteras: se mide la distancia angular a la linea de cada meridiano / paralelo
//!      (corrigiendo con cos(latitud) para que el ancho sea constante) y se pinta asfalto
//!      con grano, linea central amarilla discontinua y lineas blancas en los bordes.
//!   2. Bloques: lo que queda entre carreteras alterna dos tonos tipo ciudad/taller.
//!   3. Neon: los bordes de las carreteras emiten luz que se ve sobre todo del lado nocturno.
//!   4. Motos de luz: pulsos brillantes que recorren las carreteras con el tiempo (animado).
//!   5. Polos: plazas circulares naranjas.
//! EFECTO: shader animado (las luces recorren las carreteras).
use crate::col;
use crate::math::{v3, Vec3};
use crate::noise::fbm;
use crate::renderer::{Fragment, Uniforms};
use crate::util::{mix, smoothstep};
use std::f32::consts::{FRAC_PI_2, PI};

// ---------------- PARAMETROS QUE SE PUEDEN MODIFICAR ----------------
const MERIDIANS: f32 = 12.0;     // numero de carreteras de norte a sur
const PARALLELS: f32 = 7.0;      // numero de carreteras alrededor del planeta
const ROAD_HALF_WIDTH: f32 = 0.055; // medio ancho de la carretera (radianes)
const PULSE_SPEED: f32 = 1.6;    // velocidad de las motos de luz
const ASPHALT: Vec3 = col!(40, 40, 46);
const YELLOW: Vec3 = col!(236, 192, 40);
const BLOCK_A: Vec3 = col!(118, 62, 28);
const BLOCK_B: Vec3 = col!(66, 72, 84);
const NEON: Vec3 = col!(255, 70, 180);
const NEON2: Vec3 = col!(40, 220, 255);
const PLAZA: Vec3 = col!(230, 110, 25);
// --------------------------------------------------------------------

pub fn shade(f: &Fragment, u: &Uniforms) -> Vec3 {
    let p = f.normal;
    let ts = if u.animate { u.time } else { 0.0 };
    let lon = p.z.atan2(p.x); // -pi..pi
    let lat = p.y.clamp(-1.0, 1.0).asin(); // -pi/2..pi/2
    let cos_lat = lat.cos().max(0.05);

    // ---------- 1. Distancia a las carreteras ----------
    let mx = lon / (2.0 * PI) * MERIDIANS;
    let dm = (mx - mx.round()).abs() * (2.0 * PI / MERIDIANS) * cos_lat; // radianes al meridiano mas cercano
    let py = (lat / PI + 0.5) * PARALLELS;
    let dp = (py - py.round()).abs() * (PI / PARALLELS); // radianes al paralelo mas cercano
    let meridian_on = smoothstep(1.2, 1.1, lat.abs()) * (1.0 - smoothstep(ROAD_HALF_WIDTH - 0.006, ROAD_HALF_WIDTH, dm));
    let parallel_on = 1.0 - smoothstep(ROAD_HALF_WIDTH - 0.006, ROAD_HALF_WIDTH, dp);
    let road = meridian_on.max(parallel_on);
    let d_road = if meridian_on > parallel_on { dm } else { dp };

    // ---------- Asfalto con lineas ----------
    let grain = fbm(p * 70.0, 2, 4);
    let mut asphalt = ASPHALT * (0.75 + 0.5 * grain);
    // marcas de llantas: vetas oscuras a lo largo de la carretera
    let skid = smoothstep(0.55, 0.8, fbm(p * 22.0, 2, 12));
    asphalt = asphalt * (1.0 - 0.25 * skid);
    // linea central amarilla discontinua
    let along_m = (lat * 9.0).fract().abs();
    let along_p = (lon * 14.0 * cos_lat.max(0.3)).fract().abs();
    let dash = if meridian_on > parallel_on { along_m < 0.55 } else { along_p < 0.55 };
    if dash && d_road < 0.004 {
        asphalt = YELLOW;
    }
    // lineas blancas de borde
    if d_road > ROAD_HALF_WIDTH * 0.80 && d_road < ROAD_HALF_WIDTH * 0.92 {
        asphalt = col!(210, 210, 215);
    }

    // ---------- 2. Bloques entre carreteras ----------
    let parity = ((mx.round() + py.round()) as i32).rem_euclid(2);
    let block = mix(BLOCK_A, BLOCK_B, parity as f32) * (0.8 + 0.4 * fbm(p * 14.0, 3, 21));
    let mut color = mix(block, asphalt, road);

    // ---------- 5. Polos ----------
    let pole = smoothstep(1.28, 1.34, lat.abs());
    let ring = 0.5 + 0.5 * ((FRAC_PI_2 - lat.abs()) * 38.0).sin();
    color = mix(color, PLAZA * (0.75 + 0.25 * ring), pole);

    // ---------- Iluminacion del sol ----------
    let wet = road * 0.5 + pole * 0.2; // el asfalto brilla un poco
    let half = (f.light + f.view).normalize();
    let spec = f.normal.dot(half).max(0.0).powf(30.0) * wet * 0.6;
    let lit = 0.05 + 0.95 * smoothstep(-0.05, 0.9, f.intensity);
    let mut out = color * lit + Vec3::splat(spec * f.intensity);

    // ---------- 3. Neon (se nota mas de noche) ----------
    let night = 1.0 - smoothstep(0.0, 0.3, f.intensity);
    let edge_glow = (1.0 - smoothstep(ROAD_HALF_WIDTH, ROAD_HALF_WIDTH + 0.045, d_road)) * (1.0 - road);
    let neon_color = mix(NEON, NEON2, parity as f32);
    out += neon_color * (edge_glow * (0.12 + 0.7 * night));

    // ---------- 4. Motos de luz que recorren las carreteras ----------
    let pm = (lat * 7.0 - ts * PULSE_SPEED + mx.round() * 1.7).fract().abs();
    let pp = (lon * 9.0 * cos_lat.max(0.3) + ts * PULSE_SPEED * 0.8 + py.round() * 2.3).fract().abs();
    let on_center_m = meridian_on * (1.0 - smoothstep(0.012, 0.03, dm));
    let on_center_p = parallel_on * (1.0 - smoothstep(0.012, 0.03, dp));
    let pulse = on_center_m * smoothstep(0.93, 1.0, pm) + on_center_p * smoothstep(0.93, 1.0, pp);
    out += v3(1.0, 0.85, 0.5) * (pulse * 1.4);
    out
}

/// Si el punto de la esfera esta sobre una carretera, devuelve la direccion en que corre
/// (sirve para colocar y orientar las motocicletas sobre el asfalto).
pub fn road_direction(p: Vec3) -> Option<Vec3> {
    let lon = p.z.atan2(p.x);
    let lat = p.y.clamp(-1.0, 1.0).asin();
    if lat.abs() > 1.1 {
        return None;
    }
    let cos_lat = lat.cos().max(0.05);
    let mx = lon / (2.0 * PI) * MERIDIANS;
    let dm = (mx - mx.round()).abs() * (2.0 * PI / MERIDIANS) * cos_lat;
    let py = (lat / PI + 0.5) * PARALLELS;
    let dp = (py - py.round()).abs() * (PI / PARALLELS);
    let limit = ROAD_HALF_WIDTH * 0.35;
    if dm < limit {
        // meridiano: corre de norte a sur
        let north = v3(-lat.sin() * lon.cos(), lat.cos(), -lat.sin() * lon.sin());
        Some(north.normalize())
    } else if dp < limit {
        // paralelo: corre de oeste a este
        Some(v3(-lon.sin(), 0.0, lon.cos()).normalize())
    } else {
        None
    }
}
