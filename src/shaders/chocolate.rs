//! SHADER: PLANETA DE CHOCOLATE "Cacaolandia"
//! ==========================================
//! Una tableta de chocolate gigante que se derrite del lado que mira al sol.
//!   1. Tabletas: se proyecta la esfera sobre las 6 caras de un cubo (cada cara es una
//!      cuadricula) y se dibuja una tableta cuadrada con ranuras biseladas. La normal se
//!      inclina en los bordes (bump) para que las ranuras reaccionen a la luz.
//!   2. Derretido: un ruido decide donde el chocolate esta liquido. La zona derretida crece
//!      con la luz del sol (`intensity`), asi que SIGUE al sol mientras el planeta rota.
//!   3. Liquido: franjas verticales (como goteo) que bajan con el tiempo + vetas de chocolate
//!      blanco; es liso, mas claro y muy brillante.
//!   4. Los trozos solidos tienen grano y las ranuras son mas oscuras.
//! EFECTO: shader animado (el chocolate liquido gotea con el tiempo).
use crate::col;
use crate::math::{v3, Vec3};
use crate::noise::{fbm, noise_vec3};
use crate::renderer::{Fragment, Uniforms};
use crate::util::{mix, smoothstep};

// ---------------- PARAMETROS QUE SE PUEDEN MODIFICAR ----------------
const TILES: f32 = 4.0;        // tabletas por lado de cada cara del cubo
const GROOVE: f32 = 0.11;      // ancho de las ranuras entre tabletas
const BEVEL: f32 = 0.9;        // que tan marcado es el biselado
const MELT_BIAS: f32 = 0.20;   // cuanto derrite el sol (0 = nada, 0.6 = casi todo)
const DRIP_SPEED: f32 = 0.07;  // velocidad del goteo
const DARK: Vec3 = col!(58, 30, 17);
const MILK: Vec3 = col!(104, 58, 32);
const CARAMEL: Vec3 = col!(176, 108, 48);
const WHITE: Vec3 = col!(238, 222, 190);
// --------------------------------------------------------------------

pub fn shade(f: &Fragment, u: &Uniforms) -> Vec3 {
    let p = f.normal;
    let ts = if u.animate { u.time } else { 0.0 };

    // ---------- 1. Cuadricula en las caras de un cubo ----------
    let (ax, ay, az) = (p.x.abs(), p.y.abs(), p.z.abs());
    let (uu, vv, tu, tv) = if ax >= ay && ax >= az {
        (p.y / ax, p.z / ax, v3(0.0, 1.0, 0.0), v3(0.0, 0.0, 1.0))
    } else if ay >= az {
        (p.x / ay, p.z / ay, v3(1.0, 0.0, 0.0), v3(0.0, 0.0, 1.0))
    } else {
        (p.x / az, p.y / az, v3(1.0, 0.0, 0.0), v3(0.0, 1.0, 0.0))
    };
    let (gx, gy) = ((uu * 0.5 + 0.5) * TILES, (vv * 0.5 + 0.5) * TILES);
    let (fx, fy) = (gx - gx.floor(), gy - gy.floor());
    let ex = fx.min(1.0 - fx);
    let ey = fy.min(1.0 - fy);
    let groove_x = 1.0 - smoothstep(0.0, GROOVE, ex); // 1 dentro de la ranura
    let groove_y = 1.0 - smoothstep(0.0, GROOVE, ey);
    let groove = groove_x.max(groove_y);

    // Normal con bisel: en los bordes de cada tableta se inclina hacia afuera
    let bevel_n = (p + (tu * ((fx - 0.5) * 2.0 * groove_x) + tv * ((fy - 0.5) * 2.0 * groove_y)) * BEVEL).normalize();

    // ---------- 2. Zona derretida (crece del lado del sol) ----------
    let warp = noise_vec3(p * 1.7 + v3(0.0, ts * 0.01, 0.0), 71) * 0.35;
    let melt_field = fbm((p + warp) * 2.3, 4, 61) + MELT_BIAS * f.intensity - 0.12;
    let melt = smoothstep(0.56, 0.68, melt_field);

    // ---------- 3. Chocolate liquido: goteo y vetas blancas ----------
    let lon = p.z.atan2(p.x);
    let streak = fbm(v3(lon.cos() * 5.0, p.y * 1.4 + ts * DRIP_SPEED, lon.sin() * 5.0), 3, 17);
    let melted_color = mix(mix(MILK, CARAMEL, smoothstep(0.35, 0.75, streak)), MILK, 0.25);
    let marble = 0.5 + 0.5 * (p.y * 7.0 + 2.2 * (streak - 0.5) + ts * 0.15).sin();
    let melted_color = mix(melted_color, WHITE, smoothstep(0.93, 0.99, marble) * 0.6);

    // ---------- 4. Chocolate solido ----------
    let grain = fbm(p * 28.0, 2, 5);
    let mut solid = DARK * (0.8 + 0.5 * grain);
    solid = mix(solid, MILK, smoothstep(0.55, 0.8, fbm(p * 5.0, 3, 9)) * 0.5); // vetas de chocolate con leche
    solid = solid * (1.0 - 0.45 * groove); // ranuras mas oscuras
    // linea interior de cada tableta (pequeno relieve)
    let inner = smoothstep(0.20, 0.23, ex.min(ey)) * (1.0 - smoothstep(0.23, 0.27, ex.min(ey)));
    solid = solid * (1.0 + 0.25 * inner);

    // ---------- Mezcla final e iluminacion ----------
    let base = mix(solid, melted_color, melt);
    let normal = mix(bevel_n, p, melt); // lo derretido es liso
    let ndl = normal.dot(f.light).max(0.0);
    let half = (f.light + f.view).normalize();
    let shininess = 18.0 + 60.0 * melt;
    let spec_strength = 0.12 + 0.85 * melt;
    let spec = normal.dot(half).max(0.0).powf(shininess) * spec_strength * smoothstep(0.0, 0.2, f.intensity);
    let lit = 0.05 + 0.95 * smoothstep(-0.05, 0.9, ndl.max(f.intensity * 0.35));
    let warm = v3(1.0, 0.95, 0.88);
    base * lit * warm + Vec3::splat(spec)
}
