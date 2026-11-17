//! El sistema solar: sol, planetas que orbitan y rotan, objetos de superficie, nave y camara.
//!
//! TODO el movimiento depende del TIEMPO TRANSCURRIDO (`dt` en segundos), no de los cuadros:
//! las posiciones de los planetas son funciones del tiempo de simulacion `self.time`.
use crate::math::*;
use crate::mesh::{icosphere, load_obj, Mesh};
use crate::noise::noise3;
use crate::renderer::{Camera, DrawItem, Kind};
use crate::shaders::{chocolate, football, gas_giant, moto_planet, rocky, star, ShaderFn};
use crate::ship::{Body, Input, Ship};
use crate::util::smoothstep;
use std::f32::consts::PI;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Datos de los planetas (aqui se cambian radios, orbitas, velocidades...)
// ---------------------------------------------------------------------------
pub const SUN_RADIUS: f32 = 6.0;
const SUN_GM: f32 = 6.0 * SUN_RADIUS * SUN_RADIUS;
const SHIP_SCALE: f32 = 0.6;

/// Objetos de superficie: indice del modelo, cantidad, escala y separacion minima (radianes).
#[derive(Clone, Copy)]
pub struct PropDef {
    pub model: usize,
    pub count: usize,
    pub scale: f32,
    pub min_angle: f32,
}

#[derive(Clone, Copy)]
pub struct PlanetDef {
    pub name: &'static str,
    pub radius: f32,        // radio del planeta
    pub orbit_radius: f32,  // distancia al sol
    pub orbit_period: f32,  // segundos que tarda en dar una vuelta al sol
    pub phase: f32,         // posicion inicial en la orbita (radianes)
    pub inclination: f32,   // inclinacion de la orbita (grados)
    pub spin_period: f32,   // segundos que tarda en girar sobre su eje
    pub tilt: f32,          // inclinacion del eje (grados)
    pub shader: ShaderFn,
    pub deform: bool,       // deformar la geometria con ruido (montanas reales)
    pub props: Option<PropDef>,
    pub map_color: [u8; 3], // color en el radar
}

pub const MODEL_FILES: [&str; 5] = ["ship", "moto", "goal", "kiss", "tree"];
pub const M_SHIP: usize = 0;
pub const M_MOTO: usize = 1;
pub const M_GOAL: usize = 2;
pub const M_KISS: usize = 3;
pub const M_TREE: usize = 4;

pub fn planet_defs() -> Vec<PlanetDef> {
    vec![
        PlanetDef {
            name: "Terra Vitae (rocoso)", radius: 2.6, orbit_radius: 26.0, orbit_period: 120.0, phase: 0.6,
            inclination: 2.0, spin_period: 45.0, tilt: 18.0, shader: rocky::shade, deform: true,
            props: Some(PropDef { model: M_TREE, count: 70, scale: 0.22, min_angle: 0.10 }),
            map_color: [70, 140, 220],
        },
        PlanetDef {
            name: "Futbolandia (futbol)", radius: 2.3, orbit_radius: 38.0, orbit_period: 170.0, phase: 2.4,
            inclination: -3.0, spin_period: 22.0, tilt: 25.0, shader: football::shade, deform: false,
            props: Some(PropDef { model: M_GOAL, count: 12, scale: 0.38, min_angle: 0.50 }),
            map_color: [240, 240, 240],
        },
        PlanetDef {
            name: "Cacaolandia (chocolate)", radius: 2.7, orbit_radius: 50.0, orbit_period: 230.0, phase: 4.0,
            inclination: 4.0, spin_period: 38.0, tilt: -12.0, shader: chocolate::shade, deform: false,
            props: Some(PropDef { model: M_KISS, count: 40, scale: 0.24, min_angle: 0.20 }),
            map_color: [140, 80, 40],
        },
        PlanetDef {
            name: "Motoplaneta (motocicletas)", radius: 2.5, orbit_radius: 62.0, orbit_period: 300.0, phase: 1.2,
            inclination: 1.0, spin_period: 30.0, tilt: 8.0, shader: moto_planet::shade, deform: false,
            props: Some(PropDef { model: M_MOTO, count: 30, scale: 0.34, min_angle: 0.30 }),
            map_color: [255, 110, 30],
        },
        PlanetDef {
            name: "Nebulosa Magna (gaseoso)", radius: 5.5, orbit_radius: 82.0, orbit_period: 420.0, phase: 5.2,
            inclination: -2.0, spin_period: 24.0, tilt: 4.0, shader: gas_giant::shade, deform: false,
            props: None,
            map_color: [200, 110, 170],
        },
    ]
}

/// Planetas que visita el piloto automatico (indices de `planet_defs`) para grabar el video.
pub const DEMO_TOUR: [usize; 2] = [0, 3];

// ---------------------------------------------------------------------------
pub struct Prop {
    pub dir: Vec3,     // direccion desde el centro del planeta (espacio del planeta)
    pub forward: Vec3, // hacia donde mira el objeto (tangente a la superficie)
    pub height: f32,   // factor de radio (1 + deformacion)
}

pub struct Planet {
    pub def: PlanetDef,
    pub lods: Vec<Mesh>, // niveles de detalle (de menos a mas triangulos)
    pub pos: Vec3,
    pub axes: Axes,
    pub props: Vec<Prop>,
}

const LOD_MIN_LEVEL: u32 = 2;
const LOD_MAX_LEVEL: u32 = 5;

/// Posicion de un planeta en su orbita en el tiempo `t` (funcion pura del tiempo).
pub fn planet_pos_at(d: &PlanetDef, t: f32) -> Vec3 {
    let a = d.phase + 2.0 * PI * t / d.orbit_period;
    let incl = d.inclination.to_radians();
    let (x, z) = (a.cos() * d.orbit_radius, a.sin() * d.orbit_radius);
    v3(x, z * incl.sin(), z * incl.cos())
}

fn planet_axes_at(d: &PlanetDef, t: f32) -> Axes {
    Axes::rot_z(d.tilt.to_radians()).mul(&Axes::rot_y(2.0 * PI * t / d.spin_period))
}

// ---------------------------------------------------------------------------
// Piloto automatico para grabar el recorrido
// ---------------------------------------------------------------------------
const T_APPROACH: f32 = 14.0;
const T_DESCEND: f32 = 7.0;
const T_ORBIT: f32 = 15.0;
const T_DEPART: f32 = 7.0;
const T_VISIT: f32 = T_APPROACH + T_DESCEND + T_ORBIT + T_DEPART;

pub struct Demo {
    t0: f32,           // tiempo de simulacion cuando empezo
    starts: Vec<Vec3>, // posicion inicial de cada visita
    dirs: Vec<Vec3>,   // direccion de aproximacion de cada visita
}

pub fn demo_duration() -> f32 {
    DEMO_TOUR.len() as f32 * T_VISIT
}

pub struct Game {
    pub planets: Vec<Planet>,
    sun_lods: Vec<Mesh>,
    models: Vec<Mesh>,
    pub ship: Ship,
    pub time: f32,
    pub time_scale: f32,
    pub paused: bool,
    pub seed: u32,
    cam_pos: Vec3,
    cam_axes: Axes, // ejes con los que se dibuja
    cam_base: Axes, // ejes suavizados que siguen a la nave
    pub first_person: bool,
    pub gravity: bool,
    pub assist: bool,
    pub culling: bool,
    pub lod_enabled: bool,
    pub local_planet: Option<usize>,
    pub demo: Option<Demo>,
    pub uniforms_quality: u8,
}

fn assets_dir() -> PathBuf {
    let local = PathBuf::from("assets/models");
    if local.exists() {
        return local;
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/models")
}

/// DISTRIBUCION PROCEDURAL REPRODUCIBLE: con la misma semilla salen las mismas posiciones.
/// Se muestrean puntos de la esfera con un generador pseudoaleatorio, se agrupan con ruido
/// (clusters), se filtran con `accept` y se exige una separacion minima.
fn distribute(seed: u32, count: usize, min_angle: f32, cluster: bool, accept: &dyn Fn(Vec3) -> bool) -> Vec<Vec3> {
    let mut state = seed.wrapping_mul(2654435761).wrapping_add(12345) | 1;
    let mut rnd = || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        (state & 0xFFFFFF) as f32 / 16777216.0
    };
    let cos_min = min_angle.cos();
    let mut out: Vec<Vec3> = Vec::new();
    for _ in 0..count * 600 {
        if out.len() >= count {
            break;
        }
        let z = 2.0 * rnd() - 1.0;
        let a = 2.0 * PI * rnd();
        let r = (1.0 - z * z).sqrt();
        let p = v3(r * a.cos(), z, r * a.sin());
        if cluster && noise3(p * 2.2, seed as i32) < -0.15 {
            continue; // el ruido deja "claros" sin objetos
        }
        if !accept(p) || out.iter().any(|q| q.dot(p) > cos_min) {
            continue;
        }
        out.push(p);
    }
    out
}

impl Game {
    pub fn new(seed: u32) -> Result<Game, String> {
        let dir = assets_dir();
        let mut models = Vec::new();
        for name in MODEL_FILES {
            models.push(load_obj(dir.join(format!("{}.obj", name)).to_str().unwrap())?);
        }

        let sun_lods = (LOD_MIN_LEVEL..=LOD_MAX_LEVEL).map(|l| icosphere(l, None)).collect();
        let defs = planet_defs();
        let mut planets = Vec::new();
        for d in defs {
            // ETAPA PROPIA DE GEOMETRIA: se genera la esfera por subdivision, a varios niveles
            let lods: Vec<Mesh> = (LOD_MIN_LEVEL..=LOD_MAX_LEVEL)
                .map(|l| {
                    if d.deform {
                        icosphere(l, Some(&|p| rocky::displacement(p)))
                    } else {
                        icosphere(l, None)
                    }
                })
                .collect();
            planets.push(Planet { def: d, lods, pos: Vec3::ZERO, axes: Axes::IDENTITY, props: Vec::new() });
        }

        let start = v3(0.0, 16.0, 100.0);
        let mut g = Game {
            planets,
            sun_lods,
            models,
            ship: Ship::new(start, Vec3::ZERO),
            time: 0.0,
            time_scale: 1.0,
            paused: false,
            seed,
            cam_pos: start,
            cam_axes: Axes::IDENTITY,
            cam_base: Axes::IDENTITY,
            first_person: false,
            gravity: true,
            assist: true,
            culling: true,
            lod_enabled: true,
            local_planet: None,
            demo: None,
            uniforms_quality: 1,
        };
        g.regenerate_props();
        g.update_planets();
        g.snap_camera();
        Ok(g)
    }

    /// Vuelve a distribuir los objetos de superficie (otra semilla = otra distribucion).
    pub fn regenerate_props(&mut self) {
        for (i, pl) in self.planets.iter_mut().enumerate() {
            pl.props.clear();
            let Some(pd) = pl.def.props else { continue };
            let seed = self.seed.wrapping_add(1000 * (i as u32 + 1));
            let is_rocky = pl.def.deform;
            let is_moto = pd.model == M_MOTO;
            let accept = |p: Vec3| -> bool {
                if is_rocky {
                    rocky::is_forest(p)
                } else if is_moto {
                    moto_planet::road_direction(p).is_some()
                } else {
                    true
                }
            };
            let dirs = distribute(seed, pd.count, pd.min_angle, !is_moto, &accept);
            for dir in dirs {
                let forward = if is_moto {
                    moto_planet::road_direction(dir).unwrap()
                } else {
                    let k = noise3(dir * 9.0, 5) * PI * 2.0; // orientacion variada
                    let helper = if dir.y.abs() < 0.9 { v3(0.0, 1.0, 0.0) } else { v3(1.0, 0.0, 0.0) };
                    let t1 = helper.cross(dir).normalize();
                    let t2 = dir.cross(t1);
                    t1 * k.cos() + t2 * k.sin()
                };
                let height = if is_rocky { 1.0 + rocky::displacement(dir) } else { 1.0 };
                pl.props.push(Prop { dir, forward, height });
            }
        }
    }

    // ------------------------------------------------------------------
    fn update_planets(&mut self) {
        let t = self.time;
        for p in self.planets.iter_mut() {
            p.pos = planet_pos_at(&p.def, t);
            p.axes = planet_axes_at(&p.def, t);
        }
    }

    pub fn bodies(&self) -> Vec<Body> {
        let mut v = vec![Body { pos: Vec3::ZERO, radius: SUN_RADIUS * 1.02, gm: SUN_GM }];
        for p in &self.planets {
            let r = p.def.radius * if p.def.deform { 1.08 } else { 1.02 };
            v.push(Body { pos: p.pos, radius: r, gm: 6.0 * p.def.radius * p.def.radius });
        }
        v
    }

    /// Planeta mas cercano (indice, altura sobre la superficie).
    pub fn nearest_planet(&self) -> (usize, f32) {
        let mut best = (0, f32::MAX);
        for (i, p) in self.planets.iter().enumerate() {
            let alt = (self.ship.pos - p.pos).length() - p.def.radius;
            if alt < best.1 {
                best = (i, alt);
            }
        }
        best
    }

    // ------------------------------------------------------------------
    /// Avanza la simulacion `dt` segundos reales.
    pub fn update(&mut self, dt: f32, input: &Input) {
        let dt = dt.clamp(0.0, 0.1);
        let sim_dt = if self.paused { 0.0 } else { dt * self.time_scale };

        if self.demo.is_some() {
            self.time += sim_dt;
            self.update_planets();
            self.update_demo();
        } else {
            // Subpasos pequenos: la fisica da lo mismo en computadoras rapidas y lentas
            let steps = ((dt / 0.008).ceil() as usize).max(1);
            let h = dt / steps as f32;
            let mut local_info: Option<(usize, Vec3)> = None;
            for _ in 0..steps {
                let old: Vec<Vec3> = self.planets.iter().map(|p| p.pos).collect();
                self.time += if self.paused { 0.0 } else { h * self.time_scale };
                self.update_planets();

                // Modo de control local: cerca de un planeta, la nave viaja con el
                let (idx, alt) = self.nearest_planet();
                let pl = &self.planets[idx];
                let near = self.assist && alt < pl.def.radius * 2.8 + 2.0;
                self.local_planet = if near { Some(idx) } else { None };
                if near {
                    self.ship.pos += pl.pos - old[idx];
                    local_info = Some((idx, pl.pos));
                }
                let local = local_info.filter(|_| near).map(|(i, c)| (c, self.planets[i].def.radius));
                let bodies = self.bodies();
                self.ship.step(h, input, &bodies, self.gravity, local);
            }
        }
        self.update_camera(dt);
    }

    // ------------------------------------------------------------------
    // Camara: persigue a la nave con suavizado (o vista en primera persona)
    fn snap_camera(&mut self) {
        self.cam_base = self.ship.axes;
        self.update_camera(1.0);
    }

    fn update_camera(&mut self, dt: f32) {
        if self.first_person {
            self.cam_base = self.ship.axes;
            self.cam_axes = self.ship.axes;
            self.cam_pos = self.ship.pos + self.ship.axes.z * 0.4 + self.ship.axes.y * 0.12;
            return;
        }
        // La camara sigue a la nave con retraso suave (por eso se "siente" el giro)
        let k = 1.0 - (-9.0 * dt).exp();
        let fwd = self.cam_base.z.lerp(self.ship.axes.z, k).normalize();
        let up = self.cam_base.y.lerp(self.ship.axes.y, k).normalize();
        self.cam_base = Axes::look(fwd, up);
        let base = self.cam_base;
        let mut pos = self.ship.pos - base.z * 4.2 + base.y * 1.5;
        // La camara tampoco atraviesa los planetas
        for b in self.bodies() {
            let d = pos - b.pos;
            let min = b.radius + 0.35;
            if d.length() < min {
                pos = b.pos + d.normalize() * min;
            }
        }
        // Mira un poco por delante de la nave
        let target = self.ship.pos + base.z * 8.0;
        self.cam_axes = Axes::look((target - pos).normalize(), base.y);
        self.cam_pos = pos;
    }

    pub fn camera(&self) -> Camera {
        Camera { pos: self.cam_pos, axes: self.cam_axes, fov_deg: 62.0, near: 0.05 }
    }

    // ------------------------------------------------------------------
    // Piloto automatico
    pub fn start_demo(&mut self) {
        let mut starts = Vec::new();
        let mut dirs = Vec::new();
        let t0 = self.time;
        let mut s = self.ship.pos;
        for (k, &pi) in DEMO_TOUR.iter().enumerate() {
            let d = &self.planets[pi].def;
            let c0 = planet_pos_at(d, t0 + k as f32 * T_VISIT);
            starts.push(s);
            let mut dir = (s - c0).normalize();
            if dir.length() < 0.5 {
                dir = v3(0.0, 0.3, 1.0).normalize();
            }
            dirs.push(dir);
            // fin de esta visita = inicio de la siguiente
            let demo = Demo { t0, starts: starts.clone(), dirs: dirs.clone() };
            s = self.demo_visit_pos(&demo, k, T_VISIT);
        }
        self.demo = Some(Demo { t0, starts, dirs });
    }

    pub fn stop_demo(&mut self) {
        self.demo = None;
        self.ship.vel = Vec3::ZERO;
    }

    /// Posicion de la nave durante la visita `k`, `s` segundos despues de que empezo.
    fn demo_visit_pos(&self, demo: &Demo, k: usize, s: f32) -> Vec3 {
        let d = &self.planets[DEMO_TOUR[k]].def;
        let pl = planet_pos_at(d, demo.t0 + k as f32 * T_VISIT + s);
        let r_far = d.radius * 3.4 + 3.0;
        let r_orb = d.radius * 1.45 + 0.4;
        let r_out = r_far * 1.6;
        let e1 = demo.dirs[k];
        let tilt_n = v3(0.25, 1.0, 0.15).normalize();
        let mut e2 = tilt_n.cross(e1);
        if e2.length() < 0.2 {
            e2 = v3(0.0, 0.0, 1.0).cross(e1);
        }
        let e2 = e2.normalize();
        let omega = 2.0 * PI * 1.3 / (T_DESCEND + T_ORBIT);
        let ease = |u: f32| smoothstep(0.0, 1.0, u.clamp(0.0, 1.0));

        if s < T_APPROACH {
            let a = pl + e1 * r_far;
            demo.starts[k].lerp(a, ease(s / T_APPROACH))
        } else {
            let s2 = s - T_APPROACH;
            let r = if s2 < T_DESCEND {
                r_far + (r_orb - r_far) * ease(s2 / T_DESCEND)
            } else if s2 < T_DESCEND + T_ORBIT {
                r_orb
            } else {
                r_orb + (r_out - r_orb) * ease((s2 - T_DESCEND - T_ORBIT) / T_DEPART)
            };
            let th = omega * s2;
            pl + (e1 * th.cos() + e2 * th.sin()) * r
        }
    }

    fn update_demo(&mut self) {
        let Some(demo) = self.demo.take() else { return };
        let el = self.time - demo.t0;
        if el >= demo_duration() {
            self.stop_demo();
            return;
        }
        let k = ((el / T_VISIT) as usize).min(DEMO_TOUR.len() - 1);
        let s = el - k as f32 * T_VISIT;
        let pos = self.demo_visit_pos(&demo, k, s);
        let ahead = self.demo_visit_pos(&demo, k, (s + 0.15).min(T_VISIT));
        let back = self.demo_visit_pos(&demo, k, (s - 0.15).max(0.0));
        let fwd = (ahead - back).normalize();
        let pl = self.planets[DEMO_TOUR[k]].pos;
        let w = smoothstep(10.0, 16.0, s) * (1.0 - smoothstep(T_VISIT - 5.0, T_VISIT, s));
        let radial = (pos - pl).normalize();
        let up = v3(0.0, 1.0, 0.0).lerp(radial, w).normalize();
        self.ship.vel = (ahead - back) / 0.3;
        self.ship.pos = pos;
        if fwd.length() > 0.5 {
            let target = Axes::look(fwd, up);
            let kf = 0.25;
            self.ship.axes = Axes::look(
                self.ship.axes.z.lerp(target.z, kf).normalize(),
                self.ship.axes.y.lerp(target.y, kf).normalize(),
            );
        }
        self.local_planet = if (T_APPROACH - 2.0..T_VISIT - 3.0).contains(&s) { Some(DEMO_TOUR[k]) } else { None };
        self.demo = Some(demo);
    }

    pub fn demo_progress(&self) -> Option<f32> {
        self.demo.as_ref().map(|d| (self.time - d.t0) / demo_duration())
    }

    // ------------------------------------------------------------------
    /// Nivel de detalle segun la distancia: lejos = pocos triangulos, cerca = muchos.
    fn lod_index(&self, dist: f32, radius: f32) -> usize {
        if !self.lod_enabled {
            return (LOD_MAX_LEVEL - LOD_MIN_LEVEL) as usize;
        }
        let ratio = dist / radius;
        if ratio > 16.0 {
            0
        } else if ratio > 8.0 {
            1
        } else if ratio > 4.0 {
            2
        } else {
            3
        }
    }

    /// Construye la lista de objetos a dibujar en este cuadro.
    pub fn draw_items(&self) -> Vec<DrawItem<'_>> {
        let cam = self.cam_pos;
        let mut items: Vec<DrawItem> = Vec::new();

        // Sol
        let sun_lod = self.lod_index((cam - Vec3::ZERO).length(), SUN_RADIUS);
        items.push(DrawItem { mesh: &self.sun_lods[sun_lod], pos: Vec3::ZERO, axes: Axes::rot_y(self.time * 0.05), scale: SUN_RADIUS, kind: Kind::Shader(star::shade) });

        for p in &self.planets {
            let dist = (cam - p.pos).length();
            let lod = self.lod_index(dist, p.def.radius);
            items.push(DrawItem { mesh: &p.lods[lod], pos: p.pos, axes: p.axes, scale: p.def.radius, kind: Kind::Shader(p.def.shader) });

            // Objetos de superficie: solo si la camara esta cerca del planeta (LOD de objetos)
            if let Some(pd) = p.def.props {
                if !self.lod_enabled || dist < p.def.radius * 10.0 + 12.0 {
                    for pr in &p.props {
                        let up_axes = Axes::from_up_forward(pr.dir, pr.forward);
                        // La orientacion se compone con la del planeta: el objeto gira con el
                        let axes = p.axes.mul(&up_axes);
                        let pos = p.pos + p.axes.apply(pr.dir * (p.def.radius * pr.height));
                        items.push(DrawItem { mesh: &self.models[pd.model], pos, axes, scale: pd.scale, kind: Kind::Solid });
                    }
                }
            }
        }

        // La nave
        if !self.first_person {
            items.push(DrawItem { mesh: &self.models[M_SHIP], pos: self.ship.pos, axes: self.ship.axes, scale: SHIP_SCALE, kind: Kind::Solid });
        }
        items
    }
}

impl Game {
    pub fn set_time(&mut self, t: f32) {
        self.time = t;
        self.update_planets();
    }

    /// Atajo de desarrollo / capturas: coloca la nave cerca de un planeta (0 = sol) mirandolo.
    pub fn warp_near(&mut self, which: usize, distance: f32) {
        self.demo = None;
        let (center, radius) = if which == 0 {
            (Vec3::ZERO, SUN_RADIUS)
        } else {
            let p = &self.planets[(which - 1).min(self.planets.len() - 1)];
            (p.pos, p.def.radius)
        };
        let to_sun = if which == 0 { v3(0.3, 0.15, 1.0).normalize() } else { (-center).normalize() };
        let side = to_sun.cross(v3(0.0, 1.0, 0.0)).normalize();
        let d = radius * distance;
        let pos = center + (to_sun * 0.75 + side * 0.45 + v3(0.0, 0.3, 0.0)).normalize() * d;
        self.ship = Ship::new(pos, center);
        self.snap_camera();
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn new_game() -> Game {
        Game::new(2026).expect("assets")
    }

    /// La misma semilla da exactamente las mismas posiciones; otra semilla da otras.
    #[test]
    fn distribucion_reproducible() {
        let a = distribute(7, 30, 0.2, true, &|_| true);
        let b = distribute(7, 30, 0.2, true, &|_| true);
        let c = distribute(8, 30, 0.2, true, &|_| true);
        assert_eq!(a.len(), b.len());
        assert!(a.iter().zip(&b).all(|(x, y)| x == y));
        assert!(a.iter().zip(&c).any(|(x, y)| x != y));
    }

    /// Los planetas se mueven segun el TIEMPO, no segun los cuadros.
    #[test]
    fn orbita_depende_del_tiempo() {
        let d = planet_defs()[0];
        let p0 = planet_pos_at(&d, 0.0);
        let p1 = planet_pos_at(&d, d.orbit_period); // una vuelta completa
        assert!((p0 - p1).length() < 1e-2);
        assert!((planet_pos_at(&d, 10.0) - p0).length() > 1.0);
    }

    /// La nave no puede atravesar un planeta aunque acelere contra el.
    #[test]
    fn colision_con_planeta() {
        let mut g = new_game();
        g.warp_near(1, 3.0);
        let c = g.planets[0].pos;
        let r = g.planets[0].def.radius;
        // apuntar al centro y acelerar con impulso
        let inp = Input { thrust: 1.0, boost: true, ..Input::default() };
        let mut min_dist = f32::MAX;
        for _ in 0..600 {
            g.update(1.0 / 60.0, &inp);
            min_dist = min_dist.min((g.ship.pos - g.planets[0].pos).length());
        }
        assert!(min_dist >= r * 0.99, "la nave entro al planeta: {} < {}", min_dist, r);
        let _ = c;
    }

    /// El mismo vuelo con 30 y con 144 cuadros por segundo da casi el mismo resultado.
    #[test]
    fn fisica_independiente_de_los_fps() {
        let run = |fps: f32| {
            let mut g = new_game();
            g.paused = true; // planetas quietos para comparar solo la nave
            let inp = Input { thrust: 1.0, yaw: 0.5, ..Input::default() };
            let n = (4.0 * fps) as usize;
            for _ in 0..n {
                g.update(1.0 / fps, &inp);
            }
            g.ship.pos
        };
        let (a, b) = (run(30.0), run(144.0));
        let start = new_game().ship.pos;
        let travelled = (b - start).length();
        assert!((a - b).length() < 0.03 * travelled, "difieren demasiado: {:?} vs {:?}", a, b);
    }

    /// La gravedad desvia la trayectoria: sin motores, la nave cae hacia el planeta cercano.
    #[test]
    fn gravedad_atrae_la_nave() {
        let mut g = new_game();
        g.warp_near(1, 3.0);
        g.assist = false;
        g.paused = true;
        let d0 = (g.ship.pos - g.planets[0].pos).length();
        for _ in 0..120 {
            g.update(1.0 / 60.0, &Input::default());
        }
        let d1 = (g.ship.pos - g.planets[0].pos).length();
        assert!(d1 < d0, "no hubo atraccion: {} -> {}", d0, d1);
    }
}
