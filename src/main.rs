//! Sistema solar explorable - Proyecto final de Graficos por Computadora (Software Renderer).
//!
//! Uso:
//!   cargo run --release                       -> abre la ventana
//!   cargo run --release -- --demo             -> empieza con el recorrido automatico (para el video)
//!   cargo run --release -- --seed 7           -> otra distribucion de objetos en los planetas
//!   cargo run --release -- --width 480 --height 300 --scale 2
//!   cargo run --release -- --save foto.png --focus 4 --dist 3.2 --fp   (imagen sin ventana)
//!   cargo run --release -- --record frames --fps 30   (guarda el recorrido como PNGs sin ventana)
//! Controles: ver README.md o presiona H dentro del programa.
mod game;
mod hud;
mod math;
mod mesh;
mod noise;
mod png;
mod renderer;
mod shaders;
mod ship;
mod util;
mod window;

use game::Game;
use math::Vec3;
use renderer::{Renderer, Uniforms};
use ship::Input;
use std::time::Instant;
use window::{Display, Event};

struct Args {
    width: usize,
    height: usize,
    scale: usize,
    seed: u32,
    demo: bool,
    save: Option<String>,
    focus: Option<usize>,
    dist: f32,
    fp: bool,
    time: f32,
    record: Option<String>,
    fps: f32,
    no_cull: bool,
    no_lod: bool,
    pipe: bool,
}

fn parse_args() -> Args {
    let mut a = Args { width: 400, height: 250, scale: 2, seed: 2026, demo: false, save: None, focus: None, dist: 3.5, fp: false, time: 0.0, record: None, fps: 30.0, no_cull: false, no_lod: false, pipe: false };
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < argv.len() {
        let val = argv.get(i + 1).cloned().unwrap_or_default();
        let step = match argv[i].as_str() {
            "--width" => { a.width = val.parse().unwrap_or(400); 2 }
            "--height" => { a.height = val.parse().unwrap_or(250); 2 }
            "--scale" => { a.scale = val.parse().unwrap_or(2); 2 }
            "--seed" => { a.seed = val.parse().unwrap_or(2026); 2 }
            "--save" => { a.save = Some(val); 2 }
            "--focus" => { a.focus = val.parse().ok(); 2 }
            "--dist" => { a.dist = val.parse().unwrap_or(3.5); 2 }
            "--time" => { a.time = val.parse().unwrap_or(0.0); 2 }
            "--record" => { a.record = Some(val); 2 }
            "--fps" => { a.fps = val.parse().unwrap_or(30.0); 2 }
            "--demo" => { a.demo = true; 1 }
            "--fp" => { a.fp = true; 1 }
            "--no-cull" => { a.no_cull = true; 1 }
            "--no-lod" => { a.no_lod = true; 1 }
            "--pipe" => { a.pipe = true; 1 }
            _ => 1,
        };
        i += step;
    }
    a
}

fn print_help() {
    println!("=== CONTROLES ===");
    println!(" W / S          acelerar / retroceder        A / D   moverse a los lados");
    println!(" R / F          subir / bajar                Shift   impulso (boost)");
    println!(" Flechas        cabeceo (arriba/abajo) y giro (izq/der)     Q / E  alabeo");
    println!(" Espacio        frenar                       V       camara: tercera / primera persona");
    println!(" Tab            recorrido automatico (para el video)");
    println!(" G gravedad | L modo de control local | K descarte (culling) | O niveles de detalle (LOD)");
    println!(" P pausa | T velocidad del tiempo x1/x4/x16 | Y cambiar semilla de los objetos | M radar");
    println!(" Z calidad de shaders | Enter captura | Esc salir");
    println!(" Atajos de desarrollo: 0 = cerca del sol, 1..5 = cerca de cada planeta");
}

fn render_frame(game: &Game, renderer: &mut Renderer, buf: &mut Vec<u32>, hud_on: bool) {
    let items = game.draw_items();
    let cam = game.camera();
    let mut u = Uniforms::default();
    u.time = game.time;
    u.quality = game.uniforms_quality;
    renderer.render(&items, &cam, Vec3::ZERO, &u, game.culling, buf);
    if hud_on {
        hud::draw(buf, renderer.w, renderer.h, game, true);
    }
}

fn main() {
    let args = parse_args();
    let mut game = Game::new(args.seed).unwrap_or_else(|e| {
        eprintln!("Error al cargar el juego: {}", e);
        std::process::exit(1);
    });
    let mut renderer = Renderer::new(args.width, args.height);
    let mut buf: Vec<u32> = Vec::new();
    game.culling = !args.no_cull;
    game.lod_enabled = !args.no_lod;

    // ---------- Modo imagen: guarda un PNG y termina ----------
    if let Some(path) = &args.save {
        game.set_time(args.time);
        if let Some(f) = args.focus {
            game.warp_near(f, args.dist);
        }
        game.first_person = args.fp;
        game.update(0.0, &Input::default());
        let t0 = Instant::now();
        render_frame(&game, &mut renderer, &mut buf, !args.fp);
        let ms = t0.elapsed().as_secs_f32() * 1000.0;
        png::save_png(path, args.width, args.height, &buf).expect("no pude guardar el PNG");
        println!("Imagen guardada en {} | render {:.0} ms | {:?}", path, ms, renderer.stats);
        return;
    }

    // ---------- Modo grabacion: recorrido automatico -> PNGs (tiempo fijo por cuadro) ----------
    if args.pipe {
        // Envia los cuadros (RGB crudo) por la salida estandar, para usar con ffmpeg:
        //   sistema_solar --pipe --fps 30 | ffmpeg -f rawvideo -pix_fmt rgb24 -s 560x350 -r 30 -i - video.mp4
        use std::io::Write;
        game.start_demo();
        let dt = 1.0 / args.fps;
        let mut out = std::io::stdout().lock();
        let mut bytes = Vec::with_capacity(args.width * args.height * 3);
        while game.demo.is_some() {
            game.update(dt, &Input::default());
            render_frame(&game, &mut renderer, &mut buf, true);
            bytes.clear();
            for &p in &buf {
                bytes.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8]);
            }
            if out.write_all(&bytes).is_err() {
                break;
            }
        }
        return;
    }

    if let Some(dir) = &args.record {
        std::fs::create_dir_all(dir).ok();
        game.start_demo();
        let dt = 1.0 / args.fps;
        let mut n = 0;
        while game.demo.is_some() {
            game.update(dt, &Input::default());
            render_frame(&game, &mut renderer, &mut buf, true);
            png::save_png(&format!("{}/frame_{:05}.png", dir, n), args.width, args.height, &buf).expect("PNG");
            n += 1;
            if n % 60 == 0 {
                println!("  {} cuadros ({:.0}%)", n, game.demo_progress().unwrap_or(1.0) * 100.0);
            }
        }
        println!("Listo: {} cuadros en {}/ (a {} fps = {:.1} s)", n, dir, args.fps, n as f32 / args.fps);
        return;
    }

    // ---------- Modo interactivo ----------
    let mut display = Display::new("Sistema solar", args.width, args.height, args.scale);
    print_help();
    if args.demo {
        game.start_demo();
    }
    let mut radar = true;
    let mut shots = 0;
    let mut last = Instant::now();
    let mut fps_smooth = 60.0f32;
    let mut frame_ms = 0.0f32;

    while display.is_open() {
        let now = Instant::now();
        let dt = now.duration_since(last).as_secs_f32();
        last = now;
        fps_smooth = fps_smooth * 0.9 + (1.0 / dt.max(1e-4)) * 0.1;

        let (input, events) = display.poll();
        let mut quit = false;
        let mut want_shot = false;
        for e in events {
            match e {
                Event::Quit => quit = true,
                Event::Screenshot => want_shot = true,
                Event::ToggleDemo => {
                    if game.demo.is_some() { game.stop_demo() } else { game.start_demo() }
                }
                Event::ToggleGravity => game.gravity = !game.gravity,
                Event::ToggleAssist => game.assist = !game.assist,
                Event::ToggleCulling => game.culling = !game.culling,
                Event::ToggleLod => game.lod_enabled = !game.lod_enabled,
                Event::ToggleCamera => game.first_person = !game.first_person,
                Event::TogglePause => game.paused = !game.paused,
                Event::CycleTimeScale => game.time_scale = if game.time_scale < 3.0 { 4.0 } else if game.time_scale < 10.0 { 16.0 } else { 1.0 },
                Event::NextSeed => {
                    game.seed = game.seed.wrapping_add(1);
                    game.regenerate_props();
                }
                Event::ToggleRadar => radar = !radar,
                Event::ToggleQuality => game.uniforms_quality = 1 - game.uniforms_quality,
                Event::PrintHelp => print_help(),
                Event::Warp(i) => game.warp_near(i, 3.6),
            }
        }
        if quit {
            break;
        }

        game.update(dt, &input);
        let t0 = Instant::now();
        render_frame(&game, &mut renderer, &mut buf, false);
        hud::draw(&mut buf, renderer.w, renderer.h, &game, radar);
        frame_ms = frame_ms * 0.9 + t0.elapsed().as_secs_f32() * 1000.0 * 0.1;

        if want_shot {
            shots += 1;
            let path = format!("capturas/captura_{:02}.png", shots);
            if png::save_png(&path, args.width, args.height, &buf).is_ok() {
                println!("Captura guardada en {}", path);
            }
        }

        // Datos en el titulo de la ventana
        let (pi, alt) = game.nearest_planet();
        let s = renderer.stats;
        let title = format!(
            "{:.0} fps ({:.0} ms) | vel {:.1} | {} alt {:.1}{} | obj {}/{} tri {} (espalda {}) | grav:{} local:{} cull:{} lod:{} x{}{}{}",
            fps_smooth, frame_ms, game.ship.speed(),
            game.planets[pi].def.name, alt.max(0.0),
            if game.local_planet.is_some() { " [LOCAL]" } else { "" },
            s.objects_drawn, s.objects_total, s.tris_rasterized, s.tris_backface,
            on(game.gravity), on(game.assist), on(game.culling), on(game.lod_enabled), game.time_scale,
            if game.demo.is_some() { " | RECORRIDO AUTOMATICO" } else { "" },
            if game.paused { " | PAUSA" } else { "" },
        );
        display.set_title(&title);
        display.present(&buf);
    }
}

fn on(b: bool) -> &'static str {
    if b { "ON" } else { "off" }
}
