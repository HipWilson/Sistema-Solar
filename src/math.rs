//! Matematicas 3D: vectores, matrices 4x4, camara y perspectiva.
use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub const fn v3(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3 { x, y, z }
}

impl Vec3 {
    pub const ZERO: Vec3 = v3(0.0, 0.0, 0.0);
    pub const fn splat(v: f32) -> Vec3 {
        Vec3 { x: v, y: v, z: v }
    }
    pub fn dot(self, o: Vec3) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    pub fn cross(self, o: Vec3) -> Vec3 {
        v3(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }
    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }
    pub fn normalize(self) -> Vec3 {
        self / (self.length() + 1e-8)
    }
    pub fn lerp(self, o: Vec3, t: f32) -> Vec3 {
        self + (o - self) * t
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, o: Vec3) -> Vec3 {
        v3(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}
impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, o: Vec3) -> Vec3 {
        v3(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}
impl Mul<f32> for Vec3 {
    type Output = Vec3;
    fn mul(self, s: f32) -> Vec3 {
        v3(self.x * s, self.y * s, self.z * s)
    }
}
impl Mul<Vec3> for f32 {
    type Output = Vec3;
    fn mul(self, v: Vec3) -> Vec3 {
        v * self
    }
}
/// Multiplicacion componente a componente (para multiplicar colores).
impl Mul<Vec3> for Vec3 {
    type Output = Vec3;
    fn mul(self, o: Vec3) -> Vec3 {
        v3(self.x * o.x, self.y * o.y, self.z * o.z)
    }
}
impl Div<f32> for Vec3 {
    type Output = Vec3;
    fn div(self, s: f32) -> Vec3 {
        v3(self.x / s, self.y / s, self.z / s)
    }
}
impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        v3(-self.x, -self.y, -self.z)
    }
}
impl AddAssign for Vec3 {
    fn add_assign(&mut self, o: Vec3) {
        *self = *self + o;
    }
}
impl SubAssign for Vec3 {
    fn sub_assign(&mut self, o: Vec3) {
        *self = *self - o;
    }
}

/// Tres ejes (columnas) de una rotacion: eje X local, eje Y local y eje Z local.
/// Sirve para orientar objetos (planetas, nave, objetos de superficie).
#[derive(Clone, Copy, Debug)]
pub struct Axes {
    pub x: Vec3,
    pub y: Vec3,
    pub z: Vec3,
}

impl Axes {
    pub const IDENTITY: Axes = Axes { x: v3(1.0, 0.0, 0.0), y: v3(0.0, 1.0, 0.0), z: v3(0.0, 0.0, 1.0) };

    /// Aplica la rotacion a un vector.
    pub fn apply(&self, v: Vec3) -> Vec3 {
        self.x * v.x + self.y * v.y + self.z * v.z
    }
    /// Aplica la rotacion INVERSA (la transpuesta): del mundo al espacio del objeto.
    pub fn apply_inverse(&self, v: Vec3) -> Vec3 {
        v3(self.x.dot(v), self.y.dot(v), self.z.dot(v))
    }
    /// Composicion: primero `other`, despues `self`.
    pub fn mul(&self, other: &Axes) -> Axes {
        Axes { x: self.apply(other.x), y: self.apply(other.y), z: self.apply(other.z) }
    }
    pub fn rot_y(a: f32) -> Axes {
        let (s, c) = a.sin_cos();
        Axes { x: v3(c, 0.0, -s), y: v3(0.0, 1.0, 0.0), z: v3(s, 0.0, c) }
    }
    pub fn rot_z(a: f32) -> Axes {
        let (s, c) = a.sin_cos();
        Axes { x: v3(c, s, 0.0), y: v3(-s, c, 0.0), z: v3(0.0, 0.0, 1.0) }
    }
    /// Rotacion de `angle` radianes alrededor de un eje cualquiera (formula de Rodrigues).
    pub fn rotate_vec(v: Vec3, axis: Vec3, angle: f32) -> Vec3 {
        let (s, c) = angle.sin_cos();
        v * c + axis.cross(v) * s + axis * (axis.dot(v) * (1.0 - c))
    }
    /// Corrige errores numericos para que los tres ejes sigan siendo perpendiculares.
    pub fn orthonormalize(&self) -> Axes {
        let z = self.z.normalize();
        let y = (self.y - z * self.y.dot(z)).normalize();
        Axes { x: y.cross(z), y, z }
    }
    /// Base con el eje Z EXACTAMENTE hacia `forward` y el eje Y lo mas cerca posible de `up_hint`.
    /// (Sirve para la nave y la camara, que si pueden apuntar hacia arriba o abajo.)
    pub fn look(forward: Vec3, up_hint: Vec3) -> Axes {
        let z = forward.normalize();
        let mut y = up_hint - z * up_hint.dot(z);
        if y.length() < 1e-4 {
            y = if z.y.abs() < 0.9 { v3(0.0, 1.0, 0.0) } else { v3(1.0, 0.0, 0.0) };
            y = y - z * y.dot(z);
        }
        let y = y.normalize();
        Axes { x: y.cross(z), y, z }
    }
    /// Base ortonormal con `up` como eje Y y `forward` (aprox.) como eje Z.
    pub fn from_up_forward(up: Vec3, forward: Vec3) -> Axes {
        let y = up.normalize();
        let mut z = forward - y * forward.dot(y);
        if z.length() < 1e-4 {
            z = if y.x.abs() < 0.9 { v3(1.0, 0.0, 0.0) } else { v3(0.0, 0.0, 1.0) };
            z = z - y * z.dot(y);
        }
        let z = z.normalize();
        Axes { x: y.cross(z), y, z }
    }
}

/// Matriz 4x4 (filas).
#[derive(Clone, Copy)]
pub struct Mat4(pub [[f32; 4]; 4]);

impl Mat4 {
    pub fn mul(&self, o: &Mat4) -> Mat4 {
        let mut r = [[0.0f32; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    r[i][j] += self.0[i][k] * o.0[k][j];
                }
            }
        }
        Mat4(r)
    }
    /// Matriz de modelo: posicion + rotacion (ejes) + escala uniforme.
    pub fn model(pos: Vec3, axes: &Axes, scale: f32) -> Mat4 {
        Mat4([
            [axes.x.x * scale, axes.y.x * scale, axes.z.x * scale, pos.x],
            [axes.x.y * scale, axes.y.y * scale, axes.z.y * scale, pos.y],
            [axes.x.z * scale, axes.y.z * scale, axes.z.z * scale, pos.z],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }
    /// Transforma un punto (x, y, z, 1) y devuelve (x, y, z, w).
    pub fn transform(&self, p: Vec3) -> [f32; 4] {
        let m = &self.0;
        [
            m[0][0] * p.x + m[0][1] * p.y + m[0][2] * p.z + m[0][3],
            m[1][0] * p.x + m[1][1] * p.y + m[1][2] * p.z + m[1][3],
            m[2][0] * p.x + m[2][1] * p.y + m[2][2] * p.z + m[2][3],
            m[3][0] * p.x + m[3][1] * p.y + m[3][2] * p.z + m[3][3],
        ]
    }
    /// Transforma un punto con una matriz afin (modelo) y devuelve un Vec3.
    pub fn point(&self, p: Vec3) -> Vec3 {
        let r = self.transform(p);
        v3(r[0], r[1], r[2])
    }
}

/// Matriz de vista a partir de la posicion y los ejes de la camara.
pub fn view_matrix(eye: Vec3, axes: &Axes) -> Mat4 {
    // En un sistema de mano derecha con Y arriba y Z adelante, el eje X apunta a la IZQUIERDA.
    // La matriz de vista necesita la DERECHA (-X) y mira hacia -Z de la vista.
    let (r, u, f) = (-axes.x, axes.y, axes.z);
    Mat4([
        [r.x, r.y, r.z, -r.dot(eye)],
        [u.x, u.y, u.z, -u.dot(eye)],
        [-f.x, -f.y, -f.z, f.dot(eye)],
        [0.0, 0.0, 0.0, 1.0],
    ])
}

/// Matriz de proyeccion en perspectiva.
pub fn perspective(fov_deg: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    let t = 1.0 / (fov_deg.to_radians() / 2.0).tan();
    Mat4([
        [t / aspect, 0.0, 0.0, 0.0],
        [0.0, t, 0.0, 0.0],
        [0.0, 0.0, (far + near) / (near - far), (2.0 * far * near) / (near - far)],
        [0.0, 0.0, -1.0, 0.0],
    ])
}
