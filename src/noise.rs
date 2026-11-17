//! Ruido procedural 3D (ruido de gradiente tipo Perlin/Simplex) hecho a mano.
//! Se usa la posicion 3D sobre la esfera, asi el patron no tiene costuras.
use crate::math::{v3, Vec3};

// 16 direcciones de gradiente (las aristas de un cubo)
const GRADS: [[f32; 3]; 16] = [
    [1.0, 1.0, 0.0], [-1.0, 1.0, 0.0], [1.0, -1.0, 0.0], [-1.0, -1.0, 0.0],
    [1.0, 0.0, 1.0], [-1.0, 0.0, 1.0], [1.0, 0.0, -1.0], [-1.0, 0.0, -1.0],
    [0.0, 1.0, 1.0], [0.0, -1.0, 1.0], [0.0, 1.0, -1.0], [0.0, -1.0, -1.0],
    [1.0, 1.0, 0.0], [-1.0, 1.0, 0.0], [0.0, -1.0, 1.0], [0.0, -1.0, -1.0],
];

/// Convierte coordenadas enteras de la malla en un numero pseudoaleatorio 0..15.
fn hash(ix: i32, iy: i32, iz: i32, seed: i32) -> usize {
    let mut h = (ix as u32).wrapping_mul(73856093)
        ^ (iy as u32).wrapping_mul(19349663)
        ^ (iz as u32).wrapping_mul(83492791)
        ^ (seed as u32).wrapping_mul(668265263);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^= h >> 16;
    (h & 15) as usize
}

/// Curva suave 6t^5 - 15t^4 + 10t^3 (evita que se vea la cuadricula).
fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Ruido de gradiente 3D. Devuelve valores aproximadamente en [-1, 1].
pub fn noise3(p: Vec3, seed: i32) -> f32 {
    let (fx, fy, fz) = (p.x.floor(), p.y.floor(), p.z.floor());
    let (ix, iy, iz) = (fx as i32, fy as i32, fz as i32);
    let (x, y, z) = (p.x - fx, p.y - fy, p.z - fz);
    let (u, v, w) = (fade(x), fade(y), fade(z));

    let corner = |dx: i32, dy: i32, dz: i32| -> f32 {
        let g = GRADS[hash(ix + dx, iy + dy, iz + dz, seed)];
        g[0] * (x - dx as f32) + g[1] * (y - dy as f32) + g[2] * (z - dz as f32)
    };
    let lerp = |a: f32, b: f32, t: f32| a + t * (b - a);

    // Interpolacion trilineal de las 8 esquinas
    let x00 = lerp(corner(0, 0, 0), corner(1, 0, 0), u);
    let x10 = lerp(corner(0, 1, 0), corner(1, 1, 0), u);
    let x01 = lerp(corner(0, 0, 1), corner(1, 0, 1), u);
    let x11 = lerp(corner(0, 1, 1), corner(1, 1, 1), u);
    let y0 = lerp(x00, x10, v);
    let y1 = lerp(x01, x11, v);
    lerp(y0, y1, w)
}

/// Ruido fractal: suma de varias capas de ruido, cada una con mas frecuencia
/// y menos amplitud. Devuelve valores en [0, 1] (centrados en 0.5).
pub fn fbm(p: Vec3, octaves: u32, seed: i32) -> f32 {
    let (mut total, mut amp, mut freq, mut norm) = (0.0f32, 1.0f32, 1.0f32, 0.0f32);
    for i in 0..octaves {
        total += amp * noise3(p * freq, seed + i as i32 * 7);
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    (0.5 + 0.5 * (total / norm) * 1.6).clamp(0.0, 1.0)
}

/// Tres ruidos distintos juntos -> un vector. Sirve para deformar posiciones.
pub fn noise_vec3(p: Vec3, seed: i32) -> Vec3 {
    v3(noise3(p, seed + 11), noise3(p, seed + 23), noise3(p, seed + 37))
}
