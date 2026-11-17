//! Mallas de triangulos: carga de .obj/.mtl y generacion procedural de esferas.
//!
//! Convencion: los triangulos que miran hacia AFUERA tienen orden antihorario (CCW),
//! que es lo que exporta Blender por defecto.
use crate::math::{v3, Vec3};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Material de un modelo (viene del archivo .mtl).
#[derive(Clone, Copy, Debug)]
pub struct Material {
    pub color: Vec3,
    pub emissive: f32, // 0 = se ilumina normal, 1 = brilla por si mismo
    pub spec: f32,     // brillo especular
}

#[derive(Clone)]
pub struct Mesh {
    pub verts: Vec<Vec3>,
    pub tris: Vec<[u32; 3]>,
    pub tri_mat: Vec<u16>,       // indice de material de cada triangulo
    pub face_normals: Vec<Vec3>, // normal plana de cada triangulo
    pub materials: Vec<Material>,
    pub radius: f32, // radio de la esfera que contiene toda la malla (para el descarte)
}

impl Mesh {
    pub fn new(verts: Vec<Vec3>, tris: Vec<[u32; 3]>, tri_mat: Vec<u16>, materials: Vec<Material>) -> Mesh {
        let face_normals = tris
            .iter()
            .map(|t| {
                let (a, b, c) = (verts[t[0] as usize], verts[t[1] as usize], verts[t[2] as usize]);
                (b - a).cross(c - a).normalize()
            })
            .collect();
        let radius = verts.iter().map(|v| v.length()).fold(0.0, f32::max);
        Mesh { verts, tris, tri_mat, face_normals, materials, radius }
    }
}

// ---------------------------------------------------------------------------
// Carga de modelos .obj (+ .mtl)
// ---------------------------------------------------------------------------
fn parse_mtl(path: &Path) -> HashMap<String, Material> {
    let mut mats = HashMap::new();
    let Ok(text) = fs::read_to_string(path) else { return mats };
    let mut name = String::new();
    let mut cur = Material { color: v3(0.8, 0.8, 0.8), emissive: 0.0, spec: 0.3 };
    let mut have = false;
    let nums = |it: std::str::SplitWhitespace| -> Vec<f32> { it.filter_map(|s| s.parse().ok()).collect() };
    for line in text.lines() {
        let mut p = line.split_whitespace();
        match p.next() {
            Some("newmtl") => {
                if have {
                    mats.insert(name.clone(), cur);
                }
                name = p.next().unwrap_or("").to_string();
                cur = Material { color: v3(0.8, 0.8, 0.8), emissive: 0.0, spec: 0.3 };
                have = true;
            }
            Some("Kd") => {
                let n = nums(p);
                if n.len() >= 3 {
                    cur.color = v3(n[0], n[1], n[2]);
                }
            }
            Some("Ke") => {
                let n = nums(p);
                if n.len() >= 3 {
                    cur.emissive = n[0].max(n[1]).max(n[2]);
                }
            }
            Some("Ks") => {
                let n = nums(p);
                if !n.is_empty() {
                    cur.spec = n[0];
                }
            }
            _ => {}
        }
    }
    if have {
        mats.insert(name, cur);
    }
    mats
}

/// Carga un .obj (con su .mtl si existe). Triangula caras de mas de 3 vertices.
pub fn load_obj(path: &str) -> Result<Mesh, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("No pude leer {}: {}", path, e))?;
    let dir = Path::new(path).parent().unwrap_or(Path::new("."));
    let mut verts = Vec::new();
    let mut tris = Vec::new();
    let mut tri_mat = Vec::new();
    let mut mat_lib: HashMap<String, Material> = HashMap::new();
    let mut mat_index: HashMap<String, u16> = HashMap::new();
    let mut materials: Vec<Material> = vec![Material { color: v3(0.8, 0.8, 0.8), emissive: 0.0, spec: 0.3 }];
    let mut current: u16 = 0;

    for line in text.lines() {
        let mut p = line.split_whitespace();
        match p.next() {
            Some("mtllib") => {
                if let Some(f) = p.next() {
                    mat_lib = parse_mtl(&dir.join(f));
                }
            }
            Some("usemtl") => {
                let name = p.next().unwrap_or("").to_string();
                current = *mat_index.entry(name.clone()).or_insert_with(|| {
                    materials.push(mat_lib.get(&name).copied().unwrap_or(materials[0]));
                    (materials.len() - 1) as u16
                });
            }
            Some("v") => {
                let n: Vec<f32> = p.take(3).filter_map(|s| s.parse().ok()).collect();
                if n.len() == 3 {
                    verts.push(v3(n[0], n[1], n[2]));
                }
            }
            Some("f") => {
                let idx: Vec<u32> = p
                    .filter_map(|c| c.split('/').next()?.parse::<i64>().ok())
                    .map(|i| if i < 0 { (verts.len() as i64 + i) as u32 } else { (i - 1) as u32 })
                    .collect();
                for i in 1..idx.len().saturating_sub(1) {
                    tris.push([idx[0], idx[i], idx[i + 1]]);
                    tri_mat.push(current);
                }
            }
            _ => {}
        }
    }
    if tris.is_empty() {
        return Err(format!("{} no tiene triangulos", path));
    }
    Ok(Mesh::new(verts, tris, tri_mat, materials))
}

// ---------------------------------------------------------------------------
// Generacion procedural: icosfera con subdivision (ETAPA PROPIA DE GEOMETRIA)
// ---------------------------------------------------------------------------
/// Crea una esfera de radio 1 subdividiendo un icosaedro `level` veces.
/// Triangulos: 20 * 4^level  (nivel 0 = 20, nivel 3 = 1280, nivel 5 = 20480).
/// Si se da `displace`, cada vertice se mueve en direccion radial segun esa funcion
/// (DEFORMACION de la geometria con ruido).
pub fn icosphere(level: u32, displace: Option<&dyn Fn(Vec3) -> f32>) -> Mesh {
    let t = (1.0 + 5.0f32.sqrt()) / 2.0;
    let mut verts: Vec<Vec3> = [
        (-1.0, t, 0.0), (1.0, t, 0.0), (-1.0, -t, 0.0), (1.0, -t, 0.0),
        (0.0, -1.0, t), (0.0, 1.0, t), (0.0, -1.0, -t), (0.0, 1.0, -t),
        (t, 0.0, -1.0), (t, 0.0, 1.0), (-t, 0.0, -1.0), (-t, 0.0, 1.0),
    ]
    .iter()
    .map(|&(x, y, z)| v3(x, y, z).normalize())
    .collect();
    let mut faces: Vec<[u32; 3]> = vec![
        [0, 11, 5], [0, 5, 1], [0, 1, 7], [0, 7, 10], [0, 10, 11],
        [1, 5, 9], [5, 11, 4], [11, 10, 2], [10, 7, 6], [7, 1, 8],
        [3, 9, 4], [3, 4, 2], [3, 2, 6], [3, 6, 8], [3, 8, 9],
        [4, 9, 5], [2, 4, 11], [6, 2, 10], [8, 6, 7], [9, 8, 1],
    ];

    for _ in 0..level {
        let mut cache: HashMap<(u32, u32), u32> = HashMap::new();
        let mut midpoint = |a: u32, b: u32, verts: &mut Vec<Vec3>| -> u32 {
            let key = if a < b { (a, b) } else { (b, a) };
            *cache.entry(key).or_insert_with(|| {
                let m = ((verts[a as usize] + verts[b as usize]) * 0.5).normalize();
                verts.push(m);
                (verts.len() - 1) as u32
            })
        };
        let mut next = Vec::with_capacity(faces.len() * 4);
        for f in &faces {
            let ab = midpoint(f[0], f[1], &mut verts);
            let bc = midpoint(f[1], f[2], &mut verts);
            let ca = midpoint(f[2], f[0], &mut verts);
            next.push([f[0], ab, ca]);
            next.push([f[1], bc, ab]);
            next.push([f[2], ca, bc]);
            next.push([ab, bc, ca]);
        }
        faces = next;
    }

    if let Some(f) = displace {
        for v in verts.iter_mut() {
            let dir = v.normalize();
            *v = dir * (1.0 + f(dir));
        }
    }
    let n = faces.len();
    Mesh::new(verts, faces, vec![0; n], vec![Material { color: v3(1.0, 1.0, 1.0), emissive: 0.0, spec: 0.0 }])
}
