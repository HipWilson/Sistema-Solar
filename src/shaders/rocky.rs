//! SHADER 2: PLANETA ROCOSO "Terra Vitae"
//! ======================================
//! Oceanos, playas, bosques, desiertos rojos, montanas, nieve y casquetes polares.
//!
//! Pasos del shader (en orden):
//!   A. Campo de altura `height()`: ruido fractal + montanas "ridged".
//!   B. PARALLAX: se desplaza el punto donde se muestrea el terreno segun la
//!      direccion de la camara y la altura (las montanas parecen mas altas).
//!   C. Clasificacion: altura, humedad y latitud deciden el material.
//!   D. BUMP MAPPING: gradiente de la altura -> se inclina la normal.
//!   E. NUBES: mascara de ruido que gira a distinta velocidad que el terreno.
//!   F. SOMBRAS DE NUBES: se busca la nube que esta "hacia la luz".
//!   G. ILUMINACION: oceano brillante (especular) vs. tierra mate.
//!   H. DIA Y NOCHE: luces de ciudades solo del lado oscuro.
//!
//! EFECTOS ADICIONALES: Nubes, Sombras de nubes, Bump mapping, Parallax,
//! Respuestas diferentes a la iluminacion, Dia y noche.
use crate::col;
use crate::math::{v3, Vec3};
use crate::noise::{fbm, noise3, noise_vec3};
use crate::renderer::{Fragment, Uniforms};
use crate::util::{mix, rotate_y, smoothstep};

// ---------------- PARAMETROS QUE SE PUEDEN MODIFICAR ----------------
const SEA_LEVEL: f32 = 0.50;       // nivel del mar (mas alto = mas oceano)
const TERRAIN_SCALE: f32 = 2.0;    // tamano de continentes (mas alto = mas chicos)
const BUMP_STRENGTH: f32 = 0.35;   // que tan marcado se ve el relieve
const PARALLAX_SCALE: f32 = 0.20;  // cuanto se desplazan las montanas con la camara
const CLOUD_SPEED: f32 = 0.12;     // velocidad de las nubes (radianes por segundo)
const CLOUD_HEIGHT: f32 = 0.16;    // altura de las nubes (afecta las sombras)
const CLOUD_SHADOW_DARKNESS: f32 = 0.75; // oscuridad de las sombras (0 a 1)
const OCEAN_SHININESS: f32 = 140.0; // que tan concentrado es el brillo del mar
const OCEAN_SPECULAR: f32 = 0.75;  // intensidad del brillo del mar
const CITY_DENSITY: f32 = 0.58;    // umbral de ciudades (mas bajo = mas ciudades)
const SUN_COLOR: Vec3 = v3(1.0, 0.97, 0.90);

const OCEAN_DEEP: Vec3 = col!(8, 24, 80);
const OCEAN_SHALLOW: Vec3 = col!(30, 120, 170);
const SAND: Vec3 = col!(222, 200, 140);
const FOREST: Vec3 = col!(40, 110, 50);
const GRASS: Vec3 = col!(110, 150, 60);
const DESERT: Vec3 = col!(190, 100, 55);
const ROCK: Vec3 = col!(110, 95, 90);
const SNOW: Vec3 = col!(245, 248, 255);
const ICE: Vec3 = col!(205, 230, 245);
// --------------------------------------------------------------------

// ===================== A. CAMPO DE ALTURA =====================
/// Altura del terreno en [0, ~1]. El nivel del mar es SEA_LEVEL.
fn height(p: Vec3, octaves: u32) -> f32 {
    let base = fbm(p * TERRAIN_SCALE + Vec3::splat(3.1), octaves, 4);
    // Montanas: ruido "ridged" (1 - |ruido|) solo sobre tierra firme
    let ridge = 1.0 - noise3(p * 6.0 + Vec3::splat(1.7), 8).abs();
    let mountains = ridge * ridge * smoothstep(SEA_LEVEL + 0.02, SEA_LEVEL + 0.18, base);
    base + 0.16 * mountains
}

/// DEFORMACION DE LA GEOMETRIA: cuanto se desplaza cada vertice de la esfera (radio 1 = 0).
/// Las montanas realmente sobresalen de la silueta; el mar queda plano.
pub fn displacement(p: Vec3) -> f32 {
    0.14 * (height(p, 4) - SEA_LEVEL).max(0.0)
}

/// Sirve para colocar arboles: tierra firme de baja altura.
pub fn is_forest(p: Vec3) -> bool {
    let h = height(p, 5);
    h > SEA_LEVEL + 0.03 && h < SEA_LEVEL + 0.20 && p.y.abs() < 0.8 && fbm(p * 3.0 + Vec3::splat(20.0), 4, 15) > 0.45
}

// ===================== E. NUBES =====================
/// Mascara de nubes (0 a 1). Las nubes giran alrededor del eje Y con su propia
/// velocidad (CLOUD_SPEED) y su forma cambia un poco con el tiempo.
fn cloud_alpha(p: Vec3, t: f32, cover: f32) -> f32 {
    let mut q = rotate_y(p, -CLOUD_SPEED * t); // movimiento independiente del terreno
    q = q + noise_vec3(q * 1.8 + v3(0.0, t * 0.01, 0.0), 40) * 0.16;
    let d = fbm(q * 3.0 + Vec3::splat(11.0), 5, 12);
    let threshold = 0.80 - cover * 0.45; // cobertura -> umbral
    smoothstep(threshold, threshold + 0.14, d)
}

pub fn shade(f: &Fragment, u: &Uniforms) -> Vec3 {
    let n = f.normal; // normal geometrica (suave)
    let p = n; // punto en la esfera unitaria
    let (l, v) = (f.light, f.view);
    let n_dot_l = n.dot(l);
    let ts = if u.animate { u.time } else { 0.0 };

    // Parte de la direccion hacia la camara que es paralela a la superficie
    let view_tangent = v - n * n.dot(v);

    // ===================== B. PARALLAX =====================
    let mut p_s = p;
    let high_quality = u.quality >= 1;
    if u.parallax && high_quality {
        let h0 = height(p, 3); // altura aproximada
        let raised = (h0 - SEA_LEVEL).max(0.0); // solo la tierra "sube"
        p_s = (p + view_tangent * (raised * PARALLAX_SCALE)).normalize();
    }

    // ===================== C. CLASIFICACION DEL TERRENO =====================
    let h = height(p_s, 5);
    let is_land = h > SEA_LEVEL;
    let moisture = fbm(p_s * 3.0 + Vec3::splat(20.0), 4, 15);
    let lat = p_s.y.abs();
    let rise = (h - SEA_LEVEL).max(0.0); // altura sobre el mar

    // Oceano: mas oscuro mientras mas profundo
    let depth = ((SEA_LEVEL - h) / 0.25).clamp(0.0, 1.0);
    let ocean = mix(OCEAN_SHALLOW, OCEAN_DEEP, depth.powf(0.6));

    // Tierra: desierto (seco, cerca del ecuador) vs. bosque / pasto (humedo)
    let dryness = smoothstep(0.42, 0.58, 1.0 - moisture) * (1.0 - smoothstep(0.35, 0.85, lat));
    let green = mix(FOREST, GRASS, smoothstep(0.35, 0.65, moisture));
    let mut land = mix(green, DESERT, dryness);
    land = mix(SAND, land, smoothstep(0.0, 0.05, rise)); // playas
    land = mix(land, ROCK, smoothstep(0.20, 0.30, rise)); // montanas
    land = mix(land, SNOW, smoothstep(0.34, 0.42, rise)); // nieve en las cimas
    let mut surface = if is_land { land } else { ocean };

    // Casquetes polares (con borde irregular gracias al ruido)
    let ice_noise = fbm(p_s * 5.0 + Vec3::splat(50.0), 3, 31);
    let ice = smoothstep(0.80, 0.88, lat + 0.18 * (ice_noise - 0.5));
    surface = mix(surface, if is_land { SNOW } else { ICE }, ice);

    // ===================== D. BUMP MAPPING =====================
    let mut shading_normal = n;
    if u.bump && high_quality {
        // Base tangente (t1, t2) perpendicular a la normal
        let helper = if n.y.abs() < 0.9 { v3(0.0, 1.0, 0.0) } else { v3(1.0, 0.0, 0.0) };
        let t1 = helper.cross(n).normalize();
        let t2 = n.cross(t1);
        let eps = 0.015;
        let land_h = h.max(SEA_LEVEL); // el mar queda plano
        let h1 = height((p_s + t1 * eps).normalize(), 5).max(SEA_LEVEL);
        let h2 = height((p_s + t2 * eps).normalize(), 5).max(SEA_LEVEL);
        let d1 = (h1 - land_h) / eps; // pendiente en direccion t1
        let d2 = (h2 - land_h) / eps;
        shading_normal = (n - (t1 * d1 + t2 * d2) * BUMP_STRENGTH).normalize();
    }

    // ===================== E y F. NUBES Y SOMBRAS =====================
    let mut cloud = 0.0;
    let mut shadow = 0.0;
    if u.clouds {
        // La nube que vemos en este pixel esta a cierta altura: la miramos
        // desplazada hacia la camara.
        let p_cloud = (p + view_tangent * (CLOUD_HEIGHT / n.dot(v).max(0.25))).normalize();
        cloud = cloud_alpha(p_cloud, ts, u.cloud_cover);
        if u.cloud_shadows && high_quality {
            // La nube que tapa la luz esta desplazada HACIA el sol.
            let light_tangent = l - n * n_dot_l;
            let p_shadow = (p + light_tangent * (CLOUD_HEIGHT / n_dot_l.max(0.25))).normalize();
            shadow = cloud_alpha(p_shadow, ts, u.cloud_cover) * CLOUD_SHADOW_DARKNESS;
        }
    }

    // ===================== G. ILUMINACION =====================
    let mut diffuse = shading_normal.dot(l).clamp(0.0, 1.0);
    diffuse *= smoothstep(-0.02, 0.15, n_dot_l); // evita luz del lado oscuro
    let ambient = 0.05;
    let light_amount = ambient + (1.0 - ambient) * diffuse * (1.0 - shadow);
    let mut color = surface * light_amount * SUN_COLOR;

    // Brillo especular: SOLO oceano (fuerte) y hielo (suave). La tierra es mate.
    if u.specular {
        let half = (l + v).normalize(); // vector intermedio (Blinn-Phong)
        let n_dot_h = n.dot(half).clamp(0.0, 1.0);
        let ocean_mask = if is_land { 0.0 } else { 1.0 - ice };
        let mut spec = ocean_mask * OCEAN_SPECULAR * n_dot_h.powf(OCEAN_SHININESS);
        spec += ice * 0.25 * n_dot_h.powf(25.0);
        spec *= (1.0 - shadow) * smoothstep(0.0, 0.1, n_dot_l);
        // Reflejo suave del cielo en el mar cuando se mira de lado (Fresnel)
        let fresnel = (1.0 - n.dot(v).clamp(0.0, 1.0)).powi(4) * ocean_mask * 0.35 * diffuse;
        color = color + SUN_COLOR * (spec * 1.2) + col!(150, 190, 255) * fresnel;
    }

    // Nubes: se mezclan encima del terreno, iluminadas por el sol
    if u.clouds {
        let cloud_light = 0.06 + 0.94 * smoothstep(-0.05, 0.9, n_dot_l);
        let cloud_color = mix(col!(255, 255, 255), col!(175, 182, 200), cloud * 0.5) * cloud_light;
        color = mix(color, cloud_color, cloud * 0.92);
    }

    // ===================== H. DIA Y NOCHE: LUCES DE CIUDADES =====================
    if u.night_lights {
        let night = 1.0 - smoothstep(-0.02, 0.18, n_dot_l); // 1 solo en el lado oscuro
        if night > 0.0 && is_land && rise < 0.2 {
            let cluster = fbm(p_s * 6.0 + Vec3::splat(70.0), 3, 50);
            let dots = noise3(p_s * 80.0, 60);
            let city = smoothstep(CITY_DENSITY, CITY_DENSITY + 0.08, cluster) * smoothstep(0.18, 0.42, dots);
            let glow = smoothstep(CITY_DENSITY - 0.05, CITY_DENSITY + 0.1, cluster) * 0.08;
            let lights = (city + glow) * (1.0 - ice) * night * (1.0 - 0.85 * cloud); // las nubes tapan
            color = color + col!(255, 205, 110) * (lights * 1.3);
        }
    }

    // ===================== ATMOSFERA EN EL BORDE =====================
    let mu = n.dot(v).clamp(0.0, 1.0);
    let rim = (1.0 - mu).powf(3.5) * (0.12 + 0.88 * smoothstep(-0.1, 0.6, n_dot_l));
    color + col!(110, 170, 255) * (rim * 0.9)
}
