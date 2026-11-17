//! Renderer por software en CPU (pipeline propio).
//!
//! Cada cuadro:
//!   1. DESCARTE: los objetos fuera del campo de vision (frustum) ni se procesan.
//!   2. TRANSFORMACION: vertices -> espacio de recorte con  proyeccion * vista * modelo.
//!   3. RASTERIZACION: back-face culling, recorte contra el plano cercano,
//!      coordenadas baricentricas y z-buffer compartido por todos los objetos.
//!   4. SOMBREADO: SOLO para los pixeles visibles se construye un `Fragment` y se llama
//!      al FRAGMENT SHADER del objeto (el color final lo decide cada shader).
use crate::math::*;
use crate::mesh::Mesh;
use crate::shaders::{solid, ShaderFn};
use crate::util::smoothstep;
use std::thread;

/// Datos de un pixel visible que reciben los shaders.
pub struct Fragment {
    pub pos: Vec3,       // posicion en el espacio del objeto
    pub normal: Vec3,    // normal (planetas: esfera suave; objetos: normal plana de la cara)
    pub view: Vec3,      // direccion pixel -> camara (unitaria)
    pub light: Vec3,     // direccion pixel -> sol (unitaria)
    pub intensity: f32,  // luz difusa basica = max(0, N . L)
    pub color: Vec3,     // color del material (solo objetos solidos)
    pub emissive: f32,   // brillo propio del material
    pub spec: f32,       // brillo especular del material
}

/// Valores globales que leen los shaders.
#[derive(Clone)]
pub struct Uniforms {
    pub time: f32,           // tiempo de simulacion en segundos
    pub animate: bool,       // material animado
    pub clouds: bool,
    pub cloud_shadows: bool,
    pub cloud_cover: f32,
    pub bump: bool,
    pub parallax: bool,
    pub night_lights: bool,
    pub specular: bool,
    pub storm: bool,
    pub quality: u8,         // 0 = rapido, 1 = completo
}

impl Default for Uniforms {
    fn default() -> Self {
        Uniforms {
            time: 0.0,
            animate: true,
            clouds: true,
            cloud_shadows: true,
            cloud_cover: 0.40,
            bump: true,
            parallax: true,
            night_lights: true,
            specular: true,
            storm: true,
            quality: 1,
        }
    }
}

/// Como se sombrea un objeto.
#[derive(Clone, Copy)]
pub enum Kind {
    /// Esfera procedural: el shader recibe todo en espacio del objeto (el patron gira con el planeta).
    Shader(ShaderFn),
    /// Modelo cargado de un .obj: sombreado plano con los colores de su .mtl.
    Solid,
}

pub struct DrawItem<'a> {
    pub mesh: &'a Mesh,
    pub pos: Vec3,
    pub axes: Axes,
    pub scale: f32,
    pub kind: Kind,
}

pub struct Camera {
    pub pos: Vec3,
    pub axes: Axes, // z = hacia donde mira
    pub fov_deg: f32,
    pub near: f32,
}

/// Contadores para demostrar el descarte y los niveles de detalle.
#[derive(Clone, Copy, Default, Debug)]
pub struct Stats {
    pub objects_total: usize,
    pub objects_drawn: usize,
    pub tris_submitted: usize,     // triangulos de los objetos que SI se procesan
    pub tris_backface: usize,      // descartados por dar la espalda a la camara
    pub tris_rasterized: usize,    // triangulos que llegaron al rasterizador
}

const NO_ITEM: u16 = u16::MAX;

pub struct Renderer {
    pub w: usize,
    pub h: usize,
    depth: Vec<f32>,
    item: Vec<u16>,
    tri: Vec<u32>,
    bc: Vec<[f32; 3]>,
    clip: Vec<[f32; 4]>,
    pub stats: Stats,
}

#[derive(Clone, Copy)]
struct ClipVert {
    c: [f32; 4],
    bc: [f32; 3],
}

impl Renderer {
    pub fn new(w: usize, h: usize) -> Renderer {
        Renderer {
            w,
            h,
            depth: vec![f32::INFINITY; w * h],
            item: vec![NO_ITEM; w * h],
            tri: vec![0; w * h],
            bc: vec![[0.0; 3]; w * h],
            clip: Vec::new(),
            stats: Stats::default(),
        }
    }

    /// Dibuja la escena en `out` (un u32 0x00RRGGBB por pixel).
    pub fn render(&mut self, items: &[DrawItem], cam: &Camera, sun_pos: Vec3, u: &Uniforms, culling: bool, out: &mut Vec<u32>) {
        let (w, h) = (self.w, self.h);
        self.depth.fill(f32::INFINITY);
        self.item.fill(NO_ITEM);
        self.stats = Stats { objects_total: items.len(), ..Stats::default() };

        let aspect = w as f32 / h as f32;
        let view = view_matrix(cam.pos, &cam.axes);
        let proj = perspective(cam.fov_deg, aspect, cam.near, 2000.0);
        let vp = proj.mul(&view);
        let tan_y = (cam.fov_deg.to_radians() / 2.0).tan();
        let tan_x = tan_y * aspect;

        for (idx, it) in items.iter().enumerate() {
            // ---- 1. DESCARTE por frustum (esfera envolvente contra el campo de vision) ----
            let radius = it.mesh.radius * it.scale;
            if culling {
                let v = view.point(it.pos);
                let d = -v.z; // distancia a lo largo de la direccion de vision
                let sx = (1.0 + tan_x * tan_x).sqrt();
                let sy = (1.0 + tan_y * tan_y).sqrt();
                if d + radius < cam.near
                    || v.x.abs() > d * tan_x + radius * sx
                    || v.y.abs() > d * tan_y + radius * sy
                {
                    continue;
                }
            }
            self.stats.objects_drawn += 1;
            self.stats.tris_submitted += it.mesh.tris.len();

            // ---- 2. TRANSFORMACION de vertices: proyeccion * vista * modelo ----
            let mvp = vp.mul(&Mat4::model(it.pos, &it.axes, it.scale));
            self.clip.clear();
            self.clip.extend(it.mesh.verts.iter().map(|&p| mvp.transform(p)));

            // ---- 3. RASTERIZACION ----
            for (ti, t) in it.mesh.tris.iter().enumerate() {
                let c = [self.clip[t[0] as usize], self.clip[t[1] as usize], self.clip[t[2] as usize]];
                self.raster_triangle(c, idx as u16, ti as u32);
            }
        }

        // ---- 4. SOMBREADO de los pixeles visibles (repartido entre hilos) ----
        let right = -cam.axes.x; // X apunta a la izquierda (ver math::view_matrix)
        let up = cam.axes.y;
        let fwd = cam.axes.z;
        let sun_dir = (sun_pos - cam.pos).normalize();
        let sun_dist = (sun_pos - cam.pos).length();

        out.clear();
        out.resize(w * h, 0);
        let n_threads = thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
        let rows_per = (h + n_threads - 1) / n_threads;
        let (item_buf, tri_buf, bc_buf) = (&self.item, &self.tri, &self.bc);

        thread::scope(|scope| {
            for (ci, chunk) in out.chunks_mut(rows_per * w).enumerate() {
                scope.spawn(move || {
                    for (k, pixel) in chunk.iter_mut().enumerate() {
                        let idx = ci * rows_per * w + k;
                        let item_id = item_buf[idx];
                        if item_id == NO_ITEM {
                            // Fondo: estrellas y resplandor del sol segun la direccion del rayo
                            let (px, py) = ((idx % w) as f32, (idx / w) as f32);
                            let nx = (px + 0.5) / w as f32 * 2.0 - 1.0;
                            let ny = 1.0 - (py + 0.5) / h as f32 * 2.0;
                            let dir = (fwd + right * (nx * tan_x) + up * (ny * tan_y)).normalize();
                            *pixel = to_u32(space_color(dir, sun_dir, sun_dist));
                            continue;
                        }
                        let it = &items[item_id as usize];
                        let ti = tri_buf[idx] as usize;
                        let b = bc_buf[idx];
                        let t = it.mesh.tris[ti];
                        let (v0, v1, v2) = (it.mesh.verts[t[0] as usize], it.mesh.verts[t[1] as usize], it.mesh.verts[t[2] as usize]);
                        let local = v0 * b[0] + v1 * b[1] + v2 * b[2];
                        let world = it.pos + it.axes.apply(local) * it.scale;
                        let to_cam = (cam.pos - world).normalize();
                        let to_sun = (sun_pos - world).normalize();
                        let color = match it.kind {
                            Kind::Shader(shader) => {
                                // Todo en el espacio del objeto: el patron gira junto con el planeta
                                let n = local.normalize();
                                let light = it.axes.apply_inverse(to_sun);
                                let frag = Fragment {
                                    pos: local,
                                    normal: n,
                                    view: it.axes.apply_inverse(to_cam),
                                    light,
                                    intensity: n.dot(light).max(0.0),
                                    color: Vec3::ZERO,
                                    emissive: 0.0,
                                    spec: 0.0,
                                };
                                shader(&frag, u)
                            }
                            Kind::Solid => {
                                let m = it.mesh.materials[it.mesh.tri_mat[ti] as usize];
                                let n = it.axes.apply(it.mesh.face_normals[ti]);
                                let frag = Fragment {
                                    pos: local,
                                    normal: n,
                                    view: to_cam,
                                    light: to_sun,
                                    intensity: n.dot(to_sun).max(0.0),
                                    color: m.color,
                                    emissive: m.emissive,
                                    spec: m.spec,
                                };
                                solid::shade(&frag, u)
                            }
                        };
                        *pixel = to_u32(color);
                    }
                });
            }
        });
    }

    // ------------------------------------------------------------------
    /// Back-face culling, recorte contra el plano cercano y rasterizado de un triangulo.
    fn raster_triangle(&mut self, c: [[f32; 4]; 3], item: u16, tri: u32) {
        // Rechazo rapido: todo el triangulo fuera del mismo lado del frustum
        if (c[0][0] > c[0][3] && c[1][0] > c[1][3] && c[2][0] > c[2][3])
            || (c[0][0] < -c[0][3] && c[1][0] < -c[1][3] && c[2][0] < -c[2][3])
            || (c[0][1] > c[0][3] && c[1][1] > c[1][3] && c[2][1] > c[2][3])
            || (c[0][1] < -c[0][3] && c[1][1] < -c[1][3] && c[2][1] < -c[2][3])
        {
            return;
        }
        // Orientacion (determinante en coordenadas homogeneas): >0 = mira a la camara
        let det = c[0][0] * (c[1][1] * c[2][3] - c[1][3] * c[2][1])
            - c[0][1] * (c[1][0] * c[2][3] - c[1][3] * c[2][0])
            + c[0][3] * (c[1][0] * c[2][1] - c[1][1] * c[2][0]);
        if det <= 0.0 {
            self.stats.tris_backface += 1;
            return;
        }
        // Recorte contra el plano cercano (z + w >= 0)
        let d = [c[0][2] + c[0][3], c[1][2] + c[1][3], c[2][2] + c[2][3]];
        if d[0] < 0.0 && d[1] < 0.0 && d[2] < 0.0 {
            return;
        }
        self.stats.tris_rasterized += 1;
        let bcs = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        if d[0] >= 0.0 && d[1] >= 0.0 && d[2] >= 0.0 {
            self.fill([ClipVert { c: c[0], bc: bcs[0] }, ClipVert { c: c[1], bc: bcs[1] }, ClipVert { c: c[2], bc: bcs[2] }], item, tri);
            return;
        }
        // Sutherland-Hodgman contra un solo plano -> poligono de 3 o 4 vertices
        let mut poly: Vec<ClipVert> = Vec::with_capacity(4);
        for i in 0..3 {
            let j = (i + 1) % 3;
            let (a, b) = (ClipVert { c: c[i], bc: bcs[i] }, ClipVert { c: c[j], bc: bcs[j] });
            if d[i] >= 0.0 {
                poly.push(a);
            }
            if (d[i] >= 0.0) != (d[j] >= 0.0) {
                let t = d[i] / (d[i] - d[j]);
                let mut m = a;
                for k in 0..4 {
                    m.c[k] = a.c[k] + (b.c[k] - a.c[k]) * t;
                }
                for k in 0..3 {
                    m.bc[k] = a.bc[k] + (b.bc[k] - a.bc[k]) * t;
                }
                poly.push(m);
            }
        }
        for i in 1..poly.len().saturating_sub(1) {
            self.fill([poly[0], poly[i], poly[i + 1]], item, tri);
        }
    }

    /// Rellena un triangulo ya recortado (coordenadas baricentricas + z-buffer).
    fn fill(&mut self, v: [ClipVert; 3], item: u16, tri: u32) {
        let (w, h) = (self.w, self.h);
        let (mut sx, mut sy, mut sz, mut sw) = ([0.0f32; 3], [0.0f32; 3], [0.0f32; 3], [0.0f32; 3]);
        for i in 0..3 {
            let inv_w = 1.0 / v[i].c[3];
            sx[i] = (v[i].c[0] * inv_w * 0.5 + 0.5) * w as f32;
            sy[i] = (1.0 - (v[i].c[1] * inv_w * 0.5 + 0.5)) * h as f32;
            sz[i] = v[i].c[2] * inv_w;
            sw[i] = v[i].c[3];
        }
        let area = (sx[1] - sx[0]) * (sy[2] - sy[0]) - (sx[2] - sx[0]) * (sy[1] - sy[0]);
        if area.abs() < 1e-8 {
            return;
        }
        let min_x = sx.iter().cloned().fold(f32::INFINITY, f32::min).floor().max(0.0) as usize;
        let max_x = (sx.iter().cloned().fold(f32::NEG_INFINITY, f32::max).ceil() as i64).clamp(0, w as i64 - 1) as usize;
        let min_y = sy.iter().cloned().fold(f32::INFINITY, f32::min).floor().max(0.0) as usize;
        let max_y = (sy.iter().cloned().fold(f32::NEG_INFINITY, f32::max).ceil() as i64).clamp(0, h as i64 - 1) as usize;
        if min_x > max_x || min_y > max_y {
            return;
        }
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let (cx, cy) = (x as f32 + 0.5, y as f32 + 0.5);
                let b0 = ((sx[1] - cx) * (sy[2] - cy) - (sx[2] - cx) * (sy[1] - cy)) / area;
                let b1 = ((sx[2] - cx) * (sy[0] - cy) - (sx[0] - cx) * (sy[2] - cy)) / area;
                let b2 = 1.0 - b0 - b1;
                let eps = -1e-4;
                if b0 < eps || b1 < eps || b2 < eps {
                    continue;
                }
                let z = b0 * sz[0] + b1 * sz[1] + b2 * sz[2];
                let idx = y * w + x;
                if z >= self.depth[idx] {
                    continue;
                }
                self.depth[idx] = z;
                // Pesos corregidos por perspectiva y convertidos a baricentricas del triangulo ORIGINAL
                let (q0, q1, q2) = (b0 / sw[0], b1 / sw[1], b2 / sw[2]);
                let s = q0 + q1 + q2;
                let (q0, q1, q2) = (q0 / s, q1 / s, q2 / s);
                self.item[idx] = item;
                self.tri[idx] = tri;
                self.bc[idx] = [
                    q0 * v[0].bc[0] + q1 * v[1].bc[0] + q2 * v[2].bc[0],
                    q0 * v[0].bc[1] + q1 * v[1].bc[1] + q2 * v[2].bc[1],
                    q0 * v[0].bc[2] + q1 * v[1].bc[2] + q2 * v[2].bc[2],
                ];
            }
        }
    }
}

fn to_u32(c: Vec3) -> u32 {
    let f = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u32;
    (f(c.x) << 16) | (f(c.y) << 8) | f(c.z)
}

// ---------------------------------------------------------------------------
// Fondo del espacio (se calcula con la direccion del rayo de cada pixel)
// ---------------------------------------------------------------------------
fn hash3(x: i32, y: i32, z: i32, s: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374761393)
        ^ (y as u32).wrapping_mul(668265263)
        ^ (z as u32).wrapping_mul(2147483647)
        ^ s.wrapping_mul(1274126177);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^= h >> 16;
    (h & 0xFFFFFF) as f32 / 16777216.0
}

fn space_color(dir: Vec3, sun_dir: Vec3, sun_dist: f32) -> Vec3 {
    let mut c = v3(0.008, 0.008, 0.022);
    // Estrellas en dos capas (puntos pequenos en celdas 3D)
    for (scale, thr, size) in [(70.0f32, 0.985f32, 0.17f32), (150.0, 0.990, 0.20)] {
        let p = dir * scale;
        let cell = (p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
        let f = v3(p.x - cell.0 as f32, p.y - cell.1 as f32, p.z - cell.2 as f32);
        let h = hash3(cell.0, cell.1, cell.2, 1);
        if h > thr {
            let center = v3(
                0.3 + 0.4 * hash3(cell.0, cell.1, cell.2, 2),
                0.3 + 0.4 * hash3(cell.0, cell.1, cell.2, 3),
                0.3 + 0.4 * hash3(cell.0, cell.1, cell.2, 4),
            );
            let d = (f - center).length();
            let b = smoothstep(size, 0.0, d) * (0.5 + 0.5 * hash3(cell.0, cell.1, cell.2, 5));
            let tint = v3(1.0, 0.85 + 0.15 * hash3(cell.0, cell.1, cell.2, 6), 0.75 + 0.25 * hash3(cell.0, cell.1, cell.2, 7));
            c += tint * b;
        }
    }
    // Resplandor del sol
    let cos = sun_dir.dot(dir).max(0.0);
    let strength = (120.0 / sun_dist.max(20.0)).min(1.5);
    let glow = (cos.powf(600.0) * 0.9 + cos.powf(60.0) * 0.18 + cos.powf(8.0) * 0.03) * strength;
    c += v3(1.0, 0.65, 0.25) * glow;
    c
}
