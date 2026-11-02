//! The game on the computer instead of the device: the same `App`, with the
//! screen, piezo, flash and WiFi stood in for, playing a scripted case
//! (headquarters, the briefing, an interview, a henchman, the crime computer,
//! flights, a warning in the final city, the chase and the jail) and
//! recording it as `docs/demo.gif` through ffmpeg.
//!
//! The case is the one seed `SEED` gives a Rookie called Jon. The script
//! knows where to go from its own copy of the game, made with the same seed
//! and sent the same moves, since the rules are deterministic.
//!
//! In this folder: `cargo run --release` (needs ffmpeg). It plays in real
//! time, as scenes are timed by the clock: about a minute and a half.

// The game's modules are built whole, and the script uses only some of them.
#![allow(dead_code)]

use std::io::Write;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::rc::Rc;
use std::time::{Duration, Instant};

#[path = "../../../src/app.rs"]
mod app;
#[path = "../../../src/art/mod.rs"]
mod art;
#[path = "../../../src/lang.rs"]
mod lang;
#[path = "../../../src/rules/mod.rs"]
mod rules;
#[path = "../../../src/scenes.rs"]
mod scenes;
#[path = "../../../src/text.rs"]
mod text;
#[path = "../../../src/ui.rs"]
mod ui;

mod display {
    pub const W: usize = 320;
    pub const H: usize = 240;
}

mod device;

use app::App;
use art::map::MapView;
use display::{H, W};
use rules::data::Data;
use rules::game::{Game, RANKS, same};
use rules::rng::Rng;
use ui::{BUTTONS, Fonts, PICTURE, Pager, TEXT};

/// The case recorded: a short one with a henchman on the way and a warning before the hideout.
const SEED: u64 = 7;
const NAME: &str = "Jon";
/// Frames a second in the GIF, as the desktop game's demo.
const FPS: u64 = 8;
/// Each screen pixel as a square this big.
const SCALE: usize = 2;
const OUT: &str = "../../docs/demo.gif";

/// The game, ticked as `src/main.rs` ticks it, and a frame sent to ffmpeg every 1/FPS s.
struct Run {
    app: App,
    ffmpeg: Child,
    frames: ChildStdin,
    next_frame: Instant,
    next_tick: Instant,
}

impl Run {
    /// Runs for `seconds`.
    fn wait(&mut self, seconds: f64) {
        let end = Instant::now() + Duration::from_secs_f64(seconds);
        while Instant::now() < end {
            let now = Instant::now();
            if self.app.animating() && now >= self.next_tick {
                self.next_tick = now + Duration::from_millis(33);
                self.app.tick();
            }
            self.app.poll();
            self.app.take_cues();
            if now >= self.next_frame {
                self.next_frame += Duration::from_millis(1000 / FPS);
                self.frame();
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Runs until nothing's moving, then `then` seconds more, to read.
    fn settle(&mut self, then: f64) {
        while self.app.animating() {
            self.wait(0.1);
        }
        self.wait(then);
    }

    /// A finger on (x, y), lifted a moment later.
    fn tap(&mut self, (x, y): (i32, i32)) {
        self.app.press(x, y);
        self.wait(0.2);
        self.app.release();
        self.wait(0.1);
    }

    fn frame(&mut self) {
        let cv = self.app.draw();
        let mut bytes = Vec::with_capacity(W * H * SCALE * SCALE * 3);
        for row in cv.pixels().chunks(W) {
            let line: Vec<u8> = row.iter().flat_map(|px| std::iter::repeat_n(*px, SCALE)).flatten().collect();
            for _ in 0..SCALE {
                bytes.extend(&line);
            }
        }
        self.frames.write_all(&bytes).expect("ffmpeg taking frames");
    }
}

fn centre(r: ui::Rect) -> (i32, i32) {
    (r.x + r.w / 2, r.y + r.h / 2)
}

/// Where `app.rs` puts its text, keys and list rows (kept in step with it by hand).
fn text_area() -> ui::Rect {
    let i = TEXT.inner();
    ui::rect(i.x + ui::PAD, i.y + ui::PAD, i.w - 2 * ui::PAD, i.h - 2 * ui::PAD)
}

fn key(k: usize) -> (i32, i32) {
    let area = text_area();
    let top = area.y + 20;
    let (w, h) = (area.w / 6, (area.y + area.h - top) / 5);
    (area.x + (k as i32 % 6) * w + w / 2, top + (k as i32 / 6) * h + h / 2)
}

fn row(fonts: &Fonts, i: usize) -> (i32, i32) {
    let area = text_area();
    (area.x + area.w / 2, area.y + fonts.body.line_height() + 4 + i as i32 * 22 + 10)
}

fn place(i: usize) -> (i32, i32) {
    let p = PICTURE.inner();
    (p.x + p.w / 2, p.y + i as i32 * (p.h / 3) + p.h / 6)
}

/// What the script did to the case, to make it again.
enum Move {
    Investigate(usize),
    Warrant(Vec<(String, Option<String>)>),
    Travel(usize),
}

/// The case as it stands after `moves`, made afresh (the rules are deterministic).
fn replay(data: &Rc<Data>, moves: &[Move]) -> Game {
    let mut game = Game::new(Rc::clone(data), RANKS[0], Rng::new(SEED), NAME);
    game.briefing();
    for m in moves {
        match m {
            Move::Investigate(i) => {
                game.places_in(game.here);
                game.investigate(*i);
            }
            Move::Warrant(w) => {
                game.warrant = w.clone();
                game.compute_warrant();
            }
            Move::Travel(to) => {
                game.destinations();
                game.travel(*to);
            }
        }
    }
    game
}

fn main() {
    let (data, _) = rules::data::load(None).unwrap_or_else(|_| panic!("the data files"));
    let data: Rc<Data> = Rc::new(data);
    let fonts = Fonts::new();
    let panel = centre(TEXT);
    let [notes_button, depart, investigate, crime] = BUTTONS.map(centre);
    let _ = notes_button;

    std::fs::create_dir_all("../../docs").expect("a docs folder");
    let size = format!("{}x{}", W * SCALE, H * SCALE);
    let mut ffmpeg = Command::new("ffmpeg")
        .args(["-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgb24", "-s", &size, "-r", &FPS.to_string(), "-i", "-"])
        .args(["-vf", "split[a][b];[a]palettegen=stats_mode=full[p];[b][p]paletteuse=dither=none", "-loop", "0", OUT])
        .stdin(Stdio::piped())
        .spawn()
        .expect("ffmpeg");
    let frames = ffmpeg.stdin.take().expect("ffmpeg's input");
    let now = Instant::now();
    let app = App::new(Rc::clone(&data), || SEED, None);
    let mut run = Run { app, ffmpeg, frames, next_frame: now, next_tick: now };

    // The script's own copy of the case, sent the same moves as the game.
    let mut game = Game::new(Rc::clone(&data), RANKS[0], Rng::new(SEED), NAME);
    let mut moves = Vec::new();

    // Headquarters: the printer's welcome, then the name typed and OK.
    run.wait(2.5);
    for letter in ["J", "O", "N"] {
        let k = ["A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "Ñ", "O"].iter().position(|l| *l == letter).unwrap();
        run.tap(key(k));
        run.wait(0.3);
    }
    run.tap(key(29));
    run.settle(1.5);

    // The briefing, a page at a time, then the first city.
    let briefing = Pager::new(ui::log_rows(&fonts.body, &game.briefing(), text_area().w));
    for _ in 0..briefing.pages(&fonts.body, text_area()) {
        run.wait(2.5);
        run.tap(panel);
    }
    run.wait(2.5);

    let legs = game.route.len() - 1;
    for leg in 0..legs {
        // An interview in the first two cities: the second has a henchman.
        if leg < 2 {
            run.tap(investigate);
            game.places_in(game.here);
            run.wait(1.5);
            run.tap(place(0));
            game.investigate(0);
            moves.push(Move::Investigate(0));
            run.settle(3.0);
        }
        // The crime computer, once there's a trail: the thief's traits, then COMPUTE.
        if leg == 1 {
            run.tap(crime);
            run.wait(1.0);
            let thief = &data.suspects[game.thief];
            let fields = game.warrant_fields();
            for (i, (key, _, values)) in fields.iter().enumerate() {
                let value = if key == "sex" { thief.sex.as_str() } else { thief.get(key) };
                let at = values.iter().position(|v| same(v, value)).expect("the thief's value on file");
                for _ in 0..=at {
                    run.tap(row(&fonts, i));
                }
                game.warrant[i].1 = Some(values[at].clone());
            }
            run.wait(0.5);
            run.tap(row(&fonts, fields.len()));
            game.compute_warrant();
            moves.push(Move::Warrant(game.warrant.clone()));
            run.settle(2.5);
        }
        // Off to the next city on the trail, tapped on the map.
        run.tap(depart);
        let dests = game.destinations();
        run.wait(2.0);
        let next = game.route[leg + 1];
        let points: Vec<(f64, f64)> =
            std::iter::once(game.here).chain(dests.iter().copied()).map(|c| (game.country(c).lat, game.country(c).lon)).collect();
        let p = PICTURE.inner();
        let (x, y) = MapView::fit(p.w, p.h, &points).project(game.country(next).lat, game.country(next).lon);
        run.tap((p.x + x.round() as i32, p.y + y.round() as i32));
        game.travel(next);
        moves.push(Move::Travel(next));
        run.settle(2.5);
    }

    // The final city: a wrong place first, for its warning, then the hideout,
    // found by trying each place on a copy of the case made with the same moves.
    run.tap(investigate);
    game.places_in(game.here);
    run.wait(1.5);
    let hideout = (0..3)
        .find(|&i| {
            let mut probe = replay(&data, &moves);
            probe.places_in(probe.here);
            probe.investigate(i);
            probe.over
        })
        .expect("a hideout");
    for i in [(hideout + 1) % 3, hideout] {
        run.tap(place(i));
        game.investigate(i);
        if game.over {
            break;
        }
        run.settle(2.5);
        run.tap(investigate);
        run.wait(1.0);
    }

    // The chase, the walk to jail and the jail, each cut short, then the promotion printing.
    run.wait(8.0);
    run.tap(panel);
    run.wait(4.0);
    run.tap(panel);
    run.wait(6.0);
    run.tap(panel);
    run.settle(4.0);

    let Run { ffmpeg, frames, .. } = run;
    drop(frames);
    let status = { ffmpeg }.wait().expect("ffmpeg finishing");
    assert!(status.success(), "ffmpeg failed");
    println!("wrote {OUT}");
}
