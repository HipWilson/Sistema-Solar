//! Ventana, teclado y presentacion del framebuffer.
//!
//! IMPORTANTE: este es el UNICO archivo que usa la libreria de ventana (minifb). Si el profesor
//! exige raylib, solo hay que reescribir este archivo con raylib-rs (crear ventana, leer teclas y
//! mostrar el buffer de pixeles); el renderer, los shaders y la nave no cambian.
use crate::ship::Input;
use minifb::{Key, KeyRepeat, Scale, Window, WindowOptions};

/// Cosas que el jugador pide una sola vez (al presionar la tecla).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    Quit,
    Screenshot,
    ToggleDemo,
    ToggleGravity,
    ToggleAssist,
    ToggleCulling,
    ToggleLod,
    ToggleCamera,
    TogglePause,
    CycleTimeScale,
    NextSeed,
    ToggleRadar,
    ToggleQuality,
    PrintHelp,
    Warp(usize), // atajo de desarrollo: 0 = sol, 1..5 = planetas
}

pub struct Display {
    window: Window,
    w: usize,
    h: usize,
}

impl Display {
    pub fn new(title: &str, w: usize, h: usize, scale: usize) -> Display {
        let scale = match scale {
            1 => Scale::X1,
            4 => Scale::X4,
            _ => Scale::X2,
        };
        let mut window = Window::new(title, w, h, WindowOptions { scale, ..WindowOptions::default() })
            .expect("no pude abrir la ventana");
        window.limit_update_rate(Some(std::time::Duration::from_micros(8_000)));
        Display { window, w, h }
    }

    pub fn is_open(&self) -> bool {
        self.window.is_open()
    }

    pub fn set_title(&mut self, t: &str) {
        self.window.set_title(t);
    }

    pub fn present(&mut self, buffer: &[u32]) {
        self.window.update_with_buffer(buffer, self.w, self.h).expect("no pude actualizar la ventana");
    }

    /// Lee el teclado: devuelve los controles de la nave (mantenidos) y los eventos (un toque).
    pub fn poll(&mut self) -> (Input, Vec<Event>) {
        let w = &self.window;
        let axis = |neg: Key, pos: Key| (w.is_key_down(pos) as i32 - w.is_key_down(neg) as i32) as f32;
        let input = Input {
            pitch: axis(Key::Down, Key::Up),
            yaw: axis(Key::Left, Key::Right),
            roll: axis(Key::Q, Key::E),
            thrust: axis(Key::S, Key::W),
            strafe_x: axis(Key::A, Key::D),
            strafe_y: axis(Key::F, Key::R),
            boost: w.is_key_down(Key::LeftShift) || w.is_key_down(Key::RightShift),
            brake: w.is_key_down(Key::Space),
        };
        let pressed = |k: Key| w.is_key_pressed(k, KeyRepeat::No);
        let mut ev = Vec::new();
        let table = [
            (Key::Escape, Event::Quit),
            (Key::Enter, Event::Screenshot),
            (Key::Tab, Event::ToggleDemo),
            (Key::G, Event::ToggleGravity),
            (Key::L, Event::ToggleAssist),
            (Key::K, Event::ToggleCulling),
            (Key::O, Event::ToggleLod),
            (Key::V, Event::ToggleCamera),
            (Key::P, Event::TogglePause),
            (Key::T, Event::CycleTimeScale),
            (Key::Y, Event::NextSeed),
            (Key::M, Event::ToggleRadar),
            (Key::Z, Event::ToggleQuality),
            (Key::H, Event::PrintHelp),
            (Key::Key0, Event::Warp(0)),
            (Key::Key1, Event::Warp(1)),
            (Key::Key2, Event::Warp(2)),
            (Key::Key3, Event::Warp(3)),
            (Key::Key4, Event::Warp(4)),
            (Key::Key5, Event::Warp(5)),
        ];
        for (k, e) in table {
            if pressed(k) {
                ev.push(e);
            }
        }
        (input, ev)
    }
}
