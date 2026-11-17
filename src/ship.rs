//! La nave: fisica de vuelo y controles.
//!
//! * Los giros y la aceleracion son PROGRESIVOS (suavizados) y usan `h` (tiempo en segundos),
//!   nunca el numero de cuadros -> se siente igual en computadoras rapidas y lentas.
//! * Gravedad: el sol y los planetas atraen la nave y curvan su trayectoria.
//! * Colision: la nave no puede atravesar el sol ni los planetas.
//! * Modo de control local: cerca de un planeta la nave viaja con el, su "arriba" se alinea con
//!   la superficie y la velocidad maxima baja con la altura (para poder rodearlo con calma).
use crate::math::*;

pub const SHIP_RADIUS: f32 = 0.45;
const ACCEL: f32 = 24.0;        // aceleracion hacia adelante (unidades/s^2)
const ACCEL_STRAFE: f32 = 14.0; // aceleracion lateral
const MAX_SPEED: f32 = 42.0;
const MAX_SPEED_BOOST: f32 = 110.0;

/// Lo que el jugador (o el piloto automatico) le pide a la nave. Todo entre -1 y 1.
#[derive(Clone, Copy, Default)]
pub struct Input {
    pub pitch: f32,    // + = nariz hacia arriba
    pub yaw: f32,      // + = girar a la derecha
    pub roll: f32,     // + = inclinar a la derecha
    pub thrust: f32,   // + = acelerar, - = retroceder
    pub strafe_x: f32, // + = moverse a la derecha
    pub strafe_y: f32, // + = moverse hacia arriba
    pub boost: bool,
    pub brake: bool,
}

/// Cuerpo celeste que atrae y colisiona (sol o planeta).
#[derive(Clone, Copy)]
pub struct Body {
    pub pos: Vec3,
    pub radius: f32, // radio de colision
    pub gm: f32,     // intensidad de la gravedad
}

pub struct Ship {
    pub pos: Vec3,
    pub vel: Vec3,
    pub axes: Axes, // z = adelante, y = arriba, x = izquierda
    ang_vel: Vec3,  // velocidades de giro actuales (pitch, yaw, roll)
}

impl Ship {
    pub fn new(pos: Vec3, look_at: Vec3) -> Ship {
        Ship { pos, vel: Vec3::ZERO, axes: Axes::look(look_at - pos, v3(0.0, 1.0, 0.0)), ang_vel: Vec3::ZERO }
    }

    pub fn speed(&self) -> f32 {
        self.vel.length()
    }

    /// Avanza la simulacion `h` segundos (h pequeno: el juego llama varias veces por cuadro).
    /// `local` = (centro, radio) del planeta cercano si el modo de control local esta activo.
    pub fn step(&mut self, h: f32, inp: &Input, bodies: &[Body], gravity: bool, local: Option<(Vec3, f32)>) {
        // ---- giro progresivo: la velocidad angular se acerca poco a poco a la pedida ----
        let target = v3(inp.pitch * 1.7, inp.yaw * 1.5, inp.roll * 2.2);
        let k = 1.0 - (-7.0 * h).exp();
        self.ang_vel = self.ang_vel + (target - self.ang_vel) * k;

        let (mut ax, mut ay, mut az) = (self.axes.x, self.axes.y, self.axes.z);
        let a = -self.ang_vel.x * h; // cabeceo (nariz arriba = giro negativo alrededor de X)
        ay = Axes::rotate_vec(ay, ax, a);
        az = Axes::rotate_vec(az, ax, a);
        let a = -self.ang_vel.y * h; // viraje a la derecha
        ax = Axes::rotate_vec(ax, ay, a);
        az = Axes::rotate_vec(az, ay, a);
        let a = self.ang_vel.z * h; // alabeo a la derecha
        ax = Axes::rotate_vec(ax, az, a);
        ay = Axes::rotate_vec(ay, az, a);
        self.axes = Axes { x: ax, y: ay, z: az }.orthonormalize();

        // ---- empuje ----
        let boost = if inp.boost { 3.5 } else { 1.0 };
        let right = -self.axes.x;
        let mut acc = self.axes.z * (inp.thrust * ACCEL * boost)
            + right * (inp.strafe_x * ACCEL_STRAFE)
            + self.axes.y * (inp.strafe_y * ACCEL_STRAFE);

        // ---- gravedad de sol y planetas ----
        if gravity {
            for b in bodies {
                let d = b.pos - self.pos;
                let a = (b.gm / (d.dot(d) + 1.0)).min(14.0);
                acc += d.normalize() * a;
            }
        }
        self.vel += acc * h;

        // ---- frenado / resistencia suave (asistencia de vuelo) ----
        let drag = if inp.brake { 3.2 } else { 0.35 };
        self.vel = self.vel * (-drag * h).exp();

        // ---- velocidad maxima (baja cerca de un planeta) ----
        let mut vmax = if inp.boost { MAX_SPEED_BOOST } else { MAX_SPEED };
        if let Some((center, radius)) = local {
            let altitude = ((self.pos - center).length() - radius).max(0.0);
            vmax = vmax.min(2.5 + altitude * 5.0);
        }
        let sp = self.vel.length();
        if sp > vmax {
            self.vel = self.vel * (vmax / sp);
        }
        self.pos += self.vel * h;

        // ---- modo local: el "arriba" de la nave se alinea con la superficie ----
        if let Some((center, _)) = local {
            if inp.roll.abs() < 0.1 {
                let want = (self.pos - center).normalize();
                let axis = self.axes.y.cross(want);
                let s = axis.length();
                if s > 1e-4 {
                    let angle = s.min(1.0).asin() * (1.0 - (-1.8 * h).exp());
                    let n = axis / s;
                    self.axes = Axes {
                        x: Axes::rotate_vec(self.axes.x, n, angle),
                        y: Axes::rotate_vec(self.axes.y, n, angle),
                        z: Axes::rotate_vec(self.axes.z, n, angle),
                    }
                    .orthonormalize();
                }
            }
        }

        // ---- colision: no se puede atravesar el sol ni los planetas ----
        for b in bodies {
            let d = self.pos - b.pos;
            let dist = d.length();
            let min = b.radius + SHIP_RADIUS;
            if dist < min && dist > 1e-4 {
                let n = d / dist;
                self.pos = b.pos + n * min;
                let vn = self.vel.dot(n);
                if vn < 0.0 {
                    self.vel -= n * (vn * 1.2); // pequeno rebote
                }
            }
        }
    }
}
