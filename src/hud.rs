//! Indicadores en pantalla (HUD): radar del sistema solar, barra de velocidad y progreso del demo.
//! Se dibujan directamente sobre el framebuffer. El texto con datos va en el titulo de la ventana.
use crate::game::{Game, SUN_RADIUS};
use std::f32::consts::PI;

fn put(buf: &mut [u32], w: usize, h: usize, x: i32, y: i32, c: u32) {
    if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
        buf[y as usize * w + x as usize] = c;
    }
}

fn rect(buf: &mut [u32], w: usize, h: usize, x: i32, y: i32, rw: i32, rh: i32, c: u32) {
    for j in 0..rh {
        for i in 0..rw {
            put(buf, w, h, x + i, y + j, c);
        }
    }
}

fn blend(buf: &mut [u32], w: usize, h: usize, x: i32, y: i32, c: u32, a: f32) {
    if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
        let p = buf[y as usize * w + x as usize];
        let mix = |s: u32, d: u32| (s as f32 * a + d as f32 * (1.0 - a)) as u32;
        let r = mix((c >> 16) & 255, (p >> 16) & 255);
        let g = mix((c >> 8) & 255, (p >> 8) & 255);
        let b = mix(c & 255, p & 255);
        buf[y as usize * w + x as usize] = (r << 16) | (g << 8) | b;
    }
}

fn rgb(c: [u8; 3]) -> u32 {
    ((c[0] as u32) << 16) | ((c[1] as u32) << 8) | c[2] as u32
}

pub fn draw(buf: &mut [u32], w: usize, h: usize, game: &Game, radar: bool) {
    // ---- Radar (vista desde arriba del sistema solar) ----
    if radar {
        let size = (h as i32 / 3).clamp(64, 110);
        let (x0, y0) = (w as i32 - size - 6, 6);
        let (cx, cy) = (x0 + size / 2, y0 + size / 2);
        let world = 98.0;
        let k = (size as f32 / 2.0 - 3.0) / world;
        for j in 0..size {
            for i in 0..size {
                blend(buf, w, h, x0 + i, y0 + j, 0x000000, 0.55);
            }
        }
        // orbitas
        for p in &game.planets {
            for s in 0..90 {
                let a = s as f32 / 90.0 * 2.0 * PI;
                let r = p.def.orbit_radius * k;
                blend(buf, w, h, cx + (a.cos() * r) as i32, cy + (a.sin() * r) as i32, 0xFFFFFF, 0.25);
            }
        }
        rect(buf, w, h, cx - 2, cy - 2, 5, 5, 0xFFD040); // sol
        let _ = SUN_RADIUS;
        for p in &game.planets {
            let (px, py) = (cx + (p.pos.x * k) as i32, cy + (p.pos.z * k) as i32);
            rect(buf, w, h, px - 1, py - 1, 3, 3, rgb(p.def.map_color));
        }
        let (sx, sy) = (cx + (game.ship.pos.x * k) as i32, cy + (game.ship.pos.z * k) as i32);
        for t in 0..7 {
            let f = t as f32;
            put(buf, w, h, sx + (game.ship.axes.z.x * f) as i32, sy + (game.ship.axes.z.z * f) as i32, 0x60FF60);
        }
        rect(buf, w, h, sx - 1, sy - 1, 3, 3, 0xFFFFFF);
        // borde
        for i in 0..size {
            for &(x, y) in &[(x0 + i, y0), (x0 + i, y0 + size - 1), (x0, y0 + i), (x0 + size - 1, y0 + i)] {
                put(buf, w, h, x, y, 0x707070);
            }
        }
    }

    // ---- Barra de velocidad ----
    let bw = 90;
    let frac = (game.ship.speed() / 110.0).clamp(0.0, 1.0);
    rect(buf, w, h, 8, h as i32 - 14, bw + 2, 8, 0x202020);
    let color = if frac < 0.4 { 0x40D060 } else if frac < 0.75 { 0xE0B030 } else { 0xE05030 };
    rect(buf, w, h, 9, h as i32 - 13, (bw as f32 * frac) as i32, 6, color);

    // ---- Indicador de modo local (cerca de un planeta) ----
    if game.local_planet.is_some() {
        rect(buf, w, h, 8, h as i32 - 26, 8, 8, 0x40C0FF);
    }

    // ---- Progreso del recorrido automatico ----
    if let Some(p) = game.demo_progress() {
        rect(buf, w, h, 0, 0, w as i32, 3, 0x202020);
        rect(buf, w, h, 0, 0, (w as f32 * p.clamp(0.0, 1.0)) as i32, 3, 0xFF8040);
    }
}
