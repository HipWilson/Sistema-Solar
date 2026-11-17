//! Fragment shaders del proyecto.
//!
//! Cada shader es una funcion  `fn(&Fragment, &Uniforms) -> Vec3`  que recibe los datos de UN
//! pixel y devuelve su color (r, g, b entre 0 y 1).
//!
//!   star.rs        Sol (emite luz propia, animado)
//!   rocky.rs       Terra Vitae: planeta rocoso (nubes, relieve, oceano brillante...)
//!   gas_giant.rs   Nebulosa Magna: gigante gaseoso con vortice
//!   football.rs    Futbolandia: planeta-balon (pentagonos y hexagonos)
//!   chocolate.rs   Cacaolandia: planeta de chocolate que se derrite del lado del sol
//!   moto_planet.rs Motoplaneta: carreteras con motos de luz
//!   solid.rs       Objetos cargados de .obj (nave, motos, arboles...)
use crate::math::Vec3;
use crate::renderer::{Fragment, Uniforms};

pub mod chocolate;
pub mod football;
pub mod gas_giant;
pub mod moto_planet;
pub mod rocky;
pub mod solid;
pub mod star;

pub type ShaderFn = fn(&Fragment, &Uniforms) -> Vec3;
