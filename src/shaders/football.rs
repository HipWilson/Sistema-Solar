//! SHADER: PLANETA DE FUTBOL "Futbolandia"
//! =======================================
//! Un balon gigante: un icosaedro truncado (12 pentagonos negros + 20 hexagonos blancos).
//! No es una textura: cada pixel calcula a que cara del poliedro pertenece.
//!   * Las 32 caras se describen con su normal `n` y la distancia `d` de su plano al centro.
//!   * Un rayo desde el centro hacia el punto `p` pega primero en la cara con mayor  p.n / d.
//!   * La diferencia entre la mejor y la segunda mejor cara dice que tan cerca esta de una
//!     costura: ahi se pinta la costura oscura y se oscurece un poco (oclusion).
//!   * Ruido: manchas de pasto y tierra, y rayones en el cuero.
use crate::col;
use crate::math::{v3, Vec3};
use crate::noise::fbm;
use crate::renderer::{Fragment, Uniforms};
use crate::util::{mix, smoothstep};
use std::sync::OnceLock;

// ---------------- PARAMETROS QUE SE PUEDEN MODIFICAR ----------------
const SEAM_WIDTH: f32 = 0.022;    // grosor de las costuras
const DIRT_AMOUNT: f32 = 0.55;    // cuanta suciedad / pasto (0 = balon nuevo)
const GLOSS: f32 = 0.45;          // brillo del cuero
const PENTAGON: Vec3 = col!(18, 18, 24);
const HEXAGON: Vec3 = col!(244, 242, 236);
const SEAM: Vec3 = col!(70, 72, 78);
const GRASS: Vec3 = col!(60, 130, 55);
const DIRT: Vec3 = col!(110, 85, 55);
// --------------------------------------------------------------------

struct Faces {
    normals: [Vec3; 32], // 12 pentagonos + 20 hexagonos
    dist: [f32; 32],
}

/// Calcula una sola vez las 32 caras del icosaedro truncado.
fn faces() -> &'static Faces {
    static DATA: OnceLock<Faces> = OnceLock::new();
    DATA.get_or_init(|| {
        let t = (1.0 + 5.0f32.sqrt()) / 2.0;
        let v: Vec<Vec3> = [
            (-1.0, t, 0.0), (1.0, t, 0.0), (-1.0, -t, 0.0), (1.0, -t, 0.0),
            (0.0, -1.0, t), (0.0, 1.0, t), (0.0, -1.0, -t), (0.0, 1.0, -t),
            (t, 0.0, -1.0), (t, 0.0, 1.0), (-t, 0.0, -1.0), (-t, 0.0, 1.0),
        ]
        .iter()
        .map(|&(x, y, z)| v3(x, y, z).normalize())
        .collect();
        let adjacent = |a: usize, b: usize| v[a].dot(v[b]) > 0.3 && a != b;

        let mut normals = [Vec3::ZERO; 32];
        let mut dist = [0.0f32; 32];
        // Pentagonos: uno por cada vertice del icosaedro
        for i in 0..12 {
            let w = (0..12).find(|&j| adjacent(i, j)).unwrap();
            normals[i] = v[i];
            dist[i] = v[i].dot(v[i] + (v[w] - v[i]) / 3.0); // el corte esta a 1/3 de la arista
        }
        // Hexagonos: uno por cada cara del icosaedro
        let mut k = 12;
        for a in 0..12 {
            for b in (a + 1)..12 {
                for c in (b + 1)..12 {
                    if adjacent(a, b) && adjacent(b, c) && adjacent(a, c) {
                        let n = (v[a] + v[b] + v[c]).normalize();
                        normals[k] = n;
                        dist[k] = n.dot(v[a] + (v[b] - v[a]) / 3.0);
                        k += 1;
                    }
                }
            }
        }
        Faces { normals, dist }
    })
}

pub fn shade(f: &Fragment, _u: &Uniforms) -> Vec3 {
    let p = f.normal; // punto de la esfera en el espacio del planeta (gira con el)
    let faces = faces();

    // Cara mas cercana y segunda mas cercana
    let (mut best, mut second, mut best_i) = (-10.0f32, -10.0f32, 0usize);
    for i in 0..32 {
        let s = p.dot(faces.normals[i]) / faces.dist[i];
        if s > best {
            second = best;
            best = s;
            best_i = i;
        } else if s > second {
            second = s;
        }
    }
    let edge = best - second; // 0 en la costura
    let is_pentagon = best_i < 12;
    let seam = 1.0 - smoothstep(0.0, SEAM_WIDTH, edge);
    let puff = smoothstep(0.0, 0.10, edge); // 0 junto a la costura, 1 en el centro del panel

    // Color base de cada panel + detalle
    let grain = fbm(p * 40.0, 2, 3);
    let mut color = if is_pentagon { PENTAGON * (0.8 + 0.5 * grain) } else { HEXAGON * (0.92 + 0.12 * grain) };

    // Suciedad: pasto y tierra, sobre todo en los hexagonos blancos
    let dirt = smoothstep(0.58, 0.78, fbm(p * 3.2 + Vec3::splat(9.0), 4, 8)) * DIRT_AMOUNT;
    let dirt_color = mix(DIRT, GRASS, smoothstep(0.4, 0.7, fbm(p * 6.0 + Vec3::splat(2.0), 3, 5)));
    color = mix(color, dirt_color, dirt * if is_pentagon { 0.35 } else { 0.85 });
    color = mix(color, SEAM, seam);
    color = color * (0.62 + 0.38 * puff); // los paneles se ven "inflados"

    // Iluminacion: difusa + brillo del cuero
    let half = (f.light + f.view).normalize();
    let spec = f.normal.dot(half).max(0.0).powf(36.0) * GLOSS * puff * (1.0 - dirt);
    let lit = 0.05 + 0.95 * smoothstep(-0.05, 0.9, f.intensity);
    color * lit + Vec3::splat(spec * f.intensity)
}
