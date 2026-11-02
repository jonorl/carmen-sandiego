//! The game on the screen: headquarters (the name, typed on an on-screen
//! keyboard), the briefing, the case (the four buttons: notes, flights,
//! places to investigate, the crime computer), the ending, and the next case.
//!
//! Input is a press and a release: `press` highlights what's under the
//! finger, `release` acts on it, and `draw` paints the whole screen from the
//! state. Between touches only scenes and the printer change the screen:
//! `tick` moves them on, and says what to redraw. Sounds are left in
//! `take_cues` for the main loop to play.
//!
//! The picture frame shows the city's postcard, or what goes with the panel:
//! the places stacked while choosing one, the map while departing, the crime
//! computer and its printer, a suspect's photo, and the printer again with a
//! promotion. A witness talking stands in the text panel's bottom left corner.

use std::rc::Rc;
use std::time::Instant;

use crate::art::map::{Land, MapView};
use crate::art::pixels::{Canvas, Rgb, scale};
use crate::art::printer::Printer;
use crate::art::scene::TimeOfDay;
use crate::art::sprites;
use crate::art::{computer, people, places, postcard};
use crate::device::net::Net;
use crate::device::progress::{self, Detective, MAX_NAME, News, Progress};
use crate::device::sound::{self, Notes};
use crate::display::{H, W};
use crate::lang::{self, Lang, lang, tr};
use crate::rules::data::{Data, sex_shown};
use crate::rules::game::{Game, Line, RANKS, Tone, agency, asleep, clock_text, game_title, hour_of_day, promotion_news, rank_for};
use crate::rules::game::{PLACES, same};
use crate::rules::rng::Rng;
use crate::scenes::Stage;
use crate::text::Font;
use crate::ui::{self, BUTTON_ORDER, BUTTONS, Button, CLOCK, Fonts, PAD, PICTURE, Pager, Rect, Row, TEXT, rect};

/// Something the finger can press.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Target {
    Button(Button),
    /// A row of a list in the text panel: a place, a flight, a crime computer field.
    Row(usize),
    /// A key of the on-screen keyboard.
    Key(usize),
    /// The text panel itself, to turn the page.
    Panel,
    /// The clock box: switches language.
    Clock,
    /// The printer, while it prints: a tap prints the rest at once.
    Printer,
    /// The detective roster: a button at headquarters, half the crime computer's last row.
    Roster,
    /// Everyone's files: the other half of that row.
    Files,
}

enum Panel {
    /// Headquarters: the keyboard for the detective's name.
    Name,
    /// Text to read, a page at a time; then `then`.
    Story { pager: Pager, then: Then },
    /// The three places to investigate here (indices into `PLACES`).
    Places([usize; 3]),
    /// Where the flights from here go.
    Flights(Vec<usize>),
    /// The crime computer's fields, and its compute row.
    Crime,
    /// The case notes: deadline, warrant and everything said so far.
    Notes(Pager),
    /// Animated moments, one after another, then what happened.
    Scene(Scene),
    /// Asking the bureau for the detective's arrests as they sign in.
    Checking(Instant),
    /// Every detective on file; `online` is None while the server's asked.
    Roster { pager: Pager, from_hq: bool, online: Option<bool>, asked: Instant },
    /// One suspect's file, `who` in the data's order.
    Dossier { who: usize, pager: Pager },
}

struct Scene {
    /// The one playing first.
    stages: Vec<Stage>,
    started: Instant,
    /// How many of its cues have been played.
    cued: usize,
    then: Vec<Line>,
    /// Whether `then` comes out on the printer (a warrant's verdict) rather than in the panel.
    print: bool,
}

/// What a `tick` changed: the whole screen, or some of its parts.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Redraw {
    pub everything: bool,
    pub panel: bool,
    pub picture: bool,
    pub clock: bool,
}

impl Redraw {
    pub const NOTHING: Redraw = Redraw { everything: false, panel: false, picture: false, clock: false };
    pub const EVERYTHING: Redraw = Redraw { everything: true, ..Redraw::NOTHING };
    pub const PANEL: Redraw = Redraw { panel: true, ..Redraw::NOTHING };
    pub const PICTURE: Redraw = Redraw { picture: true, ..Redraw::NOTHING };
    pub const CLOCK: Redraw = Redraw { clock: true, ..Redraw::NOTHING };

    /// Both changes at once.
    fn and(self, other: Redraw) -> Redraw {
        Redraw {
            everything: self.everything || other.everything,
            panel: self.panel || other.panel,
            picture: self.picture || other.picture,
            clock: self.clock || other.clock,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Then {
    /// Stay: the buttons say what next.
    Stay,
    /// The briefing's over: on to the first city.
    Arrive,
    /// The case is over: back to headquarters for another.
    NextCase,
}

/// The keyboard: letters, Ñ, delete, the language and OK, six to a row.
const KEYS: [&str; 30] = [
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "Ñ", "O", "P", "Q", "R", "S", "T", "U", "V", "W", "X", "Y", "Z",
    "⌫", "LANG", "OK",
];
const KEY_COLUMNS: i32 = 6;

/// Rows of a list in the text panel: a heading, then rows this tall.
const LIST_ROW: i32 = 22;
/// How long signing in waits for the bureau, as the desktop does.
const CHECK_SECONDS: f64 = 4.0;
/// How long the roster waits for the bureau before showing this device's records.
const ROSTER_SECONDS: f64 = 12.0;
/// How long the clock takes over each hour as it catches up with the game, as the desktop's.
const HOUR_SECONDS: f64 = 0.32;

pub struct App {
    data: Rc<Data>,
    fonts: Fonts,
    progress: Progress,
    seed: fn() -> u64,
    name: String,
    game: Option<Game>,
    panel: Panel,
    pressed: Option<Target>,
    /// The postcard on show, painted once per city and time of day.
    postcard: Option<((usize, TimeOfDay), Canvas)>,
    /// The world's coastlines, read the first time the map is wanted.
    land: Option<Land>,
    /// The departure map, and the city it was made for.
    map: Option<(usize, MapView, Canvas)>,
    /// Beside the crime computer for warrants; at the end of a case for a promotion.
    printer: Printer,
    /// Headquarters' printer, where the detective signs in and gets a case.
    hq_printer: Printer,
    /// The time the clock shows, catching up with the game's an hour at a time,
    /// and when it next moves on.
    shown_t: i64,
    next_hour: Instant,
    /// The hours of the last flight, which the clock runs through quickly.
    flown: std::ops::Range<i64>,
    /// The places here and the one being walked to.
    picked: Option<([usize; 3], usize)>,
    /// The witness talking (an index into `PLACES`), whose portrait shows.
    witness: Option<usize>,
    /// The flight under way: from, to.
    flight: Option<(usize, usize)>,
    /// Sounds to play (None to stop), for the main loop.
    cues: Vec<Option<Notes>>,
}

impl App {
    /// Starts at headquarters. `seed` gives each new case its randomness;
    /// `net` reaches the bureau's server, if there's WiFi.
    pub fn new(data: Rc<Data>, seed: fn() -> u64, net: Option<Net>) -> App {
        let mut app = App {
            data,
            fonts: Fonts::new(),
            progress: Progress::load(net),
            seed,
            name: String::new(),
            game: None,
            panel: Panel::Name,
            pressed: None,
            postcard: None,
            land: None,
            map: None,
            printer: Printer::new(),
            hq_printer: Printer::new(),
            shown_t: 0,
            next_hour: Instant::now(),
            flown: 0..0,
            picked: None,
            witness: None,
            flight: None,
            cues: Vec::new(),
        };
        app.hq_print(&format!("{}\n\n{}\n", game_title(), agency().to_uppercase()));
        app.hq_print(tr(
            "Welcome to headquarters.\n\nDetective at keyboard, please identify yourself:",
            "Cuartel general.\n\nDetective al teclado, identificate:",
        ));
        app
    }

    /// Queues `text` on headquarters' printer.
    fn hq_print(&mut self, text: &str) {
        self.hq_printer.print(&self.fonts.label, text, Printer::text_width(PICTURE.inner().w));
    }

    /// Sounds the game wants played since last asked, in order; None stops the sound.
    pub fn take_cues(&mut self) -> Vec<Option<Notes>> {
        std::mem::take(&mut self.cues)
    }

    /// Whether something moves on by itself: a scene, the printer, or a wait for the bureau.
    pub fn animating(&self) -> bool {
        matches!(self.panel, Panel::Scene(_) | Panel::Checking(_) | Panel::Roster { online: None, .. })
            || self.printer.busy()
            || self.hq_printer.busy()
            || self.game.as_ref().is_some_and(|g| g.t != self.shown_t)
    }

    /// Takes in the bureau's answers. Call often.
    pub fn poll(&mut self) -> Redraw {
        let mut redraw = Redraw::NOTHING;
        for news in self.progress.poll() {
            match news {
                News::LookedUp if matches!(self.panel, Panel::Checking(_)) => {
                    self.new_case();
                    redraw = Redraw::EVERYTHING;
                }
                News::Roster(list) if matches!(self.panel, Panel::Roster { online: None, .. }) => {
                    self.show_roster(list);
                    redraw = Redraw::EVERYTHING;
                }
                _ => {}
            }
        }
        redraw
    }

    /// Moves a scene and the printer on: plays their sounds as they come
    /// due, and goes to a scene's next stage, or to what happened, when one
    /// ends. Gives up on the bureau when it's slow.
    pub fn tick(&mut self) -> Redraw {
        let mut printed = Redraw::NOTHING;
        for printer in [&mut self.printer, &mut self.hq_printer] {
            if printer.busy() {
                if printer.update() > 0 {
                    self.cues.push(Some(sound::PRINT));
                }
                printed = Redraw::PICTURE;
            }
        }
        let clock = self.tick_clock();
        printed.and(clock).and(self.tick_scene())
    }

    /// Moves the clock on an hour, ticking, if it's behind the game and the
    /// hour's up; hours asleep or in the air go by twice as fast.
    fn tick_clock(&mut self) -> Redraw {
        let Some(game) = &self.game else { return Redraw::NOTHING };
        if self.shown_t > game.t {
            self.shown_t = game.t;
            return Redraw::CLOCK;
        }
        if self.shown_t == game.t || Instant::now() < self.next_hour {
            return Redraw::NOTHING;
        }
        self.shown_t += 1;
        let quick = asleep(self.shown_t) || self.flown.contains(&self.shown_t);
        self.next_hour = Instant::now() + std::time::Duration::from_secs_f64(if quick { HOUR_SECONDS / 2.0 } else { HOUR_SECONDS });
        // The piezo plays one sound at a time: a tick would cut a scene's tune short.
        if !matches!(self.panel, Panel::Scene(_)) {
            self.cues.push(Some(sound::TICK));
        }
        Redraw::CLOCK
    }

    fn tick_scene(&mut self) -> Redraw {
        match &self.panel {
            Panel::Checking(since) if since.elapsed().as_secs_f64() >= CHECK_SECONDS => {
                self.new_case();
                return Redraw::EVERYTHING;
            }
            Panel::Roster { online: None, asked, .. } if asked.elapsed().as_secs_f64() >= ROSTER_SECONDS => {
                self.show_roster(None);
                return Redraw::EVERYTHING;
            }
            _ => {}
        }
        let Panel::Scene(scene) = &mut self.panel else { return Redraw::NOTHING };
        let t = scene.started.elapsed().as_secs_f64();
        let cues = scene.stages[0].cues();
        while let Some(&(at, notes)) = cues.get(scene.cued)
            && at <= t
        {
            self.cues.push(Some(notes));
            scene.cued += 1;
        }
        if t < scene.stages[0].seconds(scene_area()) {
            // The map's plane and the computer's lights move with these.
            return if matches!(scene.stages[0], Stage::Fly(_) | Stage::Compute) {
                Redraw::PANEL.and(Redraw::PICTURE)
            } else {
                Redraw::PANEL
            };
        }
        self.next_stage();
        if matches!(self.panel, Panel::Scene(_)) { Redraw::PANEL.and(Redraw::PICTURE) } else { Redraw::EVERYTHING }
    }

    /// Ends the scene's current stage: on to the next, or to what happened.
    fn next_stage(&mut self) {
        let Panel::Scene(scene) = &mut self.panel else { return };
        self.cues.push(None);
        scene.stages.remove(0);
        if scene.stages.is_empty() {
            let then = std::mem::take(&mut scene.then);
            if scene.print {
                self.print_verdict(&then);
            } else {
                self.tell(then);
            }
        } else {
            scene.started = Instant::now();
            scene.cued = 0;
        }
    }

    /// Plays `stages`, then shows `then`.
    fn play(&mut self, stages: Vec<Stage>, then: Vec<Line>) {
        if stages.is_empty() {
            self.tell(then);
        } else {
            self.panel = Panel::Scene(Scene { stages, started: Instant::now(), cued: 0, then, print: false });
        }
    }

    /// The crime computer's verdict, on the printer beside it, as in the original.
    fn print_verdict(&mut self, lines: &[Line]) {
        for line in lines {
            log::info!("[{:?}] {}", line.tone, line.text);
        }
        let verdict: Vec<&str> = lines.iter().map(|l| l.text.as_str()).filter(|t| !t.is_empty()).collect();
        let (f, width) = (&self.fonts.label, Printer::text_width(PICTURE.inner().w));
        self.printer.print(f, &format!("\n{}", verdict.join("\n")), width);
        self.printer.print(f, &format!("\n{}", ready()), width);
        self.panel = Panel::Crime;
    }

    // ------------------------------------------------------------ input

    /// The finger went down at (x, y); true if the screen should change.
    pub fn press(&mut self, x: i32, y: i32) -> bool {
        self.pressed = self.target_at(x, y);
        self.pressed.is_some()
    }

    /// The finger came up: act on what it pressed. True if the screen should change.
    pub fn release(&mut self) -> bool {
        let Some(target) = self.pressed.take() else { return false };
        self.act(target);
        true
    }

    fn buttons_on(&self) -> bool {
        self.game.as_ref().is_some_and(|g| !g.over)
            && !matches!(self.panel, Panel::Name | Panel::Story { then: Then::Arrive, .. } | Panel::Scene(_) | Panel::Checking(_))
    }

    fn target_at(&self, x: i32, y: i32) -> Option<Target> {
        if CLOCK.contains(x, y) {
            return Some(Target::Clock);
        }
        if let Some(i) = BUTTONS.iter().position(|b| b.contains(x, y)) {
            return self.buttons_on().then_some(Target::Button(BUTTON_ORDER[i]));
        }
        if matches!(self.panel, Panel::Name) && hq_roster_button().contains(x, y) {
            return Some(Target::Roster);
        }
        if PICTURE.contains(x, y) {
            return self.picture_target(x, y);
        }
        if !TEXT.contains(x, y) {
            return None;
        }
        let area = text_area();
        match &self.panel {
            Panel::Name => {
                let (kx, ky) = ((x - area.x) / key_size().0, (y - keyboard_top()) / key_size().1);
                let k = (ky * KEY_COLUMNS + kx) as usize;
                (y >= keyboard_top() && kx < KEY_COLUMNS && k < KEYS.len()).then_some(Target::Key(k))
            }
            Panel::Story { .. } | Panel::Notes(_) | Panel::Scene(_) | Panel::Roster { .. } | Panel::Dossier { .. } => Some(Target::Panel),
            Panel::Checking(_) => None,
            Panel::Places(_) => self.list_row(y, 3),
            Panel::Flights(dests) => self.list_row(y, dests.len()),
            Panel::Crime => {
                // The fields, COMPUTE, then a row split between the files and the roster.
                let fields = self.game.as_ref().map_or(0, |g| g.warrant.len());
                match self.list_row(y, fields + 2) {
                    Some(Target::Row(i)) if i == fields + 1 => Some(if x < area.x + area.w / 2 { Target::Files } else { Target::Roster }),
                    other => other,
                }
            }
        }
    }

    /// What's under the finger in the picture frame: a place's picture, a
    /// city on the map, or the printer.
    fn picture_target(&self, x: i32, y: i32) -> Option<Target> {
        if self.printer.busy() || (self.at_hq() && self.hq_printer.busy()) {
            return Some(Target::Printer);
        }
        match &self.panel {
            Panel::Places(_) => (0..3).find(|&i| place_slot(i).contains(x, y)).map(Target::Row),
            Panel::Flights(dests) => {
                let (_, view, _) = self.map.as_ref()?;
                let game = self.game.as_ref()?;
                let p = PICTURE.inner();
                let near = |&(_, &d): &(usize, &usize)| {
                    let (mx, my) = view.project(game.country(d).lat, game.country(d).lon);
                    let (dx, dy) = (mx - (x - p.x) as f32, my - (y - p.y) as f32);
                    dx * dx + dy * dy
                };
                let (i, d) = dests.iter().enumerate().min_by(|a, b| near(a).total_cmp(&near(b)))?;
                (near(&(i, d)) <= 12.0 * 12.0).then_some(Target::Row(i))
            }
            _ => None,
        }
    }

    /// Which of `n` list rows is at height `y`.
    fn list_row(&self, y: i32, n: usize) -> Option<Target> {
        let top = list_top(&self.fonts);
        let row = (y - top).div_euclid(LIST_ROW);
        (y >= top && (row as usize) < n).then_some(Target::Row(row as usize))
    }

    fn act(&mut self, target: Target) {
        match target {
            Target::Clock => {
                lang::choose(if lang() == Lang::En { Lang::Es } else { Lang::En });
            }
            Target::Key(k) => {
                self.cues.push(Some(sound::KEY));
                self.key(k);
            }
            // A tap skips the rest of a scene's stage.
            Target::Panel if self.animating() => self.next_stage(),
            Target::Panel => {
                self.cues.push(Some(sound::TICK));
                self.turn_page();
            }
            Target::Button(b) => {
                self.cues.push(Some(sound::TICK));
                self.button(b);
            }
            Target::Row(i) => self.row(i),
            Target::Printer => {
                self.printer.finish();
                self.hq_printer.finish();
            }
            Target::Roster => {
                self.cues.push(Some(sound::TICK));
                self.ask_roster();
            }
            Target::Files => {
                self.cues.push(Some(sound::TICK));
                self.dossier(0);
            }
        }
    }

    fn key(&mut self, k: usize) {
        match KEYS[k] {
            "⌫" => {
                self.name.pop();
            }
            "LANG" => lang::choose(if lang() == Lang::En { Lang::Es } else { Lang::En }),
            "OK" => {
                if !self.name.trim().is_empty() {
                    let who = self.name.trim().to_string();
                    self.hq_print(&format!("{who}\n"));
                    self.hq_print(&tr(format!("You have been identified, {who}."), format!("Identidad confirmada, {who}.")));
                    // The bureau may know them from another device; don't wait long.
                    if self.progress.look_up(&self.name) {
                        self.panel = Panel::Checking(Instant::now());
                    } else {
                        self.new_case();
                    }
                }
            }
            letter => {
                if self.name.chars().count() < MAX_NAME {
                    // Typed in capitals, kept as a name: Jon, not JON.
                    let c = if self.name.is_empty() { letter.to_string() } else { letter.to_lowercase() };
                    self.name.push_str(&c);
                }
            }
        }
    }

    // Turning a page changes it, which a match guard can't do.
    #[allow(clippy::collapsible_match)]
    fn turn_page(&mut self) {
        let (f, area, story) = (&self.fonts.body, text_area(), self.story_area());
        match &mut self.panel {
            Panel::Story { pager, then } => {
                if pager.turn(f, story) {
                    return;
                }
                match *then {
                    Then::Stay => {}
                    Then::Arrive => self.arrive(),
                    Then::NextCase => self.new_case(),
                }
            }
            Panel::Notes(pager) => {
                if !pager.turn(f, area) {
                    pager.page = 0;
                }
            }
            Panel::Roster { pager, from_hq, .. } => {
                if !pager.turn(f, area) {
                    self.panel = if *from_hq { Panel::Name } else { Panel::Crime };
                }
            }
            Panel::Dossier { who, pager } => {
                // After the last page of a file, the next suspect's.
                if !pager.turn(f, area) {
                    let next = (*who + 1) % self.data.suspects.len();
                    self.dossier(next);
                }
            }
            _ => {}
        }
    }

    /// Asks the bureau for the roster; this device's records if there's no server.
    fn ask_roster(&mut self) {
        let from_hq = self.game.is_none();
        if self.progress.ask_roster() {
            let waiting = ui::rows(&self.fonts.body, tr("Asking the bureau…", "Consultando a la agencia…"), ui::GREY, text_area().w);
            self.panel = Panel::Roster { pager: Pager::new(waiting), from_hq, online: None, asked: Instant::now() };
        } else {
            self.panel = Panel::Roster { pager: Pager::default(), from_hq, online: Some(false), asked: Instant::now() };
            self.show_roster(None);
        }
    }

    /// The roster: `list` from the bureau, or this device's records if None.
    fn show_roster(&mut self, list: Option<Vec<Detective>>) {
        let Panel::Roster { from_hq, .. } = self.panel else { return };
        let online = list.is_some();
        let list = list.unwrap_or_else(|| self.progress.local_roster());
        let (f, width) = (&self.fonts.body, text_area().w);
        let mut rows = ui::rows(f, tr("DETECTIVE ROSTER", "LISTA DE DETECTIVES"), ui::YELLOW, width);
        if !online {
            rows.extend(ui::rows(
                f,
                tr("(This device only: no answer from the bureau.)", "(Solo este equipo: la agencia no responde.)"),
                ui::GREY,
                width,
            ));
        }
        rows.push(Row::gap());
        let me = progress::key(&self.name);
        for d in &list {
            let mine = me.is_some() && progress::key(&d.name) == me;
            let mut name: Vec<char> = d.name.chars().collect();
            if let Some(first) = name.first_mut() {
                *first = first.to_uppercase().next().unwrap_or(*first);
            }
            let name: String = name.into_iter().collect();
            rows.extend(ui::rows(f, &name, if mine { ui::YELLOW } else { ui::WHITE }, width));
            let rank = RANKS[rank_for(d.arrests)].label();
            let arrests = tr(format!("  {rank}, {} arrests", d.arrests), format!("  {rank}, {} arrestos", d.arrests));
            rows.extend(ui::rows(f, &arrests, ui::GREY, width));
        }
        if list.is_empty() {
            rows.extend(ui::rows(f, tr("Nobody on file yet.", "Todavía no hay nadie."), ui::GREY, width));
        }
        self.panel = Panel::Roster { pager: Pager::new(rows), from_hq, online: Some(online), asked: Instant::now() };
    }

    /// Suspect `who`'s file: the photo in the picture frame, the facts in the panel.
    fn dossier(&mut self, who: usize) {
        let s = &self.data.suspects[who];
        let (f, width) = (&self.fonts.body, text_area().w);
        let mut rows = ui::rows(f, &s.name.to_uppercase(), ui::YELLOW, width);
        let mut facts = vec![(tr("Sex", "Sexo").to_string(), sex_shown(&s.sex).to_string())];
        for t in &self.data.traits {
            facts.push((t.label().to_string(), s.shown(t).to_string()));
        }
        if let Some(bio) = &s.bio {
            facts.push((tr("Notes", "Notas").to_string(), bio.get().to_string()));
        }
        for (label, value) in facts {
            let mut lines = ui::rows(f, &format!("{label}: {value}"), ui::WHITE, width);
            if let Some(first) = lines.first_mut() {
                first.speaker = (label.chars().count() + 1).min(first.text.chars().count());
            }
            rows.extend(lines);
        }
        if let Some(said) = &s.catchphrase {
            rows.extend(ui::rows(f, &format!("\u{201c}{said}\u{201d}"), ui::LIME, width));
        }
        let n = self.data.suspects.len();
        rows.extend(ui::rows(
            f,
            &tr(format!("File {} of {n}: tap for the next.", who + 1), format!("Ficha {} de {n}: tocá para la siguiente.", who + 1)),
            ui::GREY,
            width,
        ));
        self.panel = Panel::Dossier { who, pager: Pager::new(rows) };
    }

    fn button(&mut self, b: Button) {
        let Some(game) = self.game.as_mut() else { return };
        self.witness = None;
        self.printer.finish();
        if b == Button::Depart && self.map.as_ref().is_none_or(|(city, _, _)| *city != game.here) {
            // The map fits the city and everywhere it flies to.
            let here = game.here;
            let points: Vec<(f64, f64)> =
                std::iter::once(here).chain(game.destinations()).map(|c| (game.country(c).lat, game.country(c).lon)).collect();
            let p = PICTURE.inner();
            let view = MapView::fit(p.w, p.h, &points);
            let land = self.land.get_or_insert_with(Land::load);
            self.map = Some((here, view, view.render(land)));
        }
        self.panel = match b {
            Button::Notes => {
                let mut pager = Pager::new(notes_rows(&self.fonts, game));
                // Open on the newest page; a tap goes on round from the oldest.
                pager.page = pager.pages(&self.fonts.body, text_area()) - 1;
                Panel::Notes(pager)
            }
            Button::Depart => Panel::Flights(game.destinations()),
            Button::Investigate => Panel::Places(game.places_in(game.here)),
            Button::Crime => Panel::Crime,
        };
    }

    fn row(&mut self, i: usize) {
        let Some(game) = self.game.as_mut() else { return };
        let before = game.log.len();
        let mut stages = Vec::new();
        match &self.panel {
            Panel::Places(places) => {
                // Places already visited are no walk away.
                if !game.visited_here().contains(&i) {
                    stages.push(Stage::Walk(PLACES[places[i]].label().to_string()));
                }
                self.picked = Some((*places, i));
                game.investigate(i);
                // The witness shows while they talk; the hideout has none.
                self.witness = (!game.showdown).then_some(places[i]);
                if let Some((kind, first)) = game.henchman.take() {
                    stages.push(Stage::Henchman { kind, tell: first });
                }
                if let Some(kind) = game.danger.take() {
                    stages.push(Stage::Danger(kind));
                }
                if game.over && game.showdown {
                    stages.push(Stage::Chase { female: game.thief_is_female() });
                    stages.push(Stage::March { caught: game.won });
                    stages.push(Stage::Jail { caught: game.won });
                }
            }
            Panel::Flights(dests) => {
                stages.push(Stage::Fly(game.country(dests[i]).city().to_string()));
                self.flight = Some((game.here, dests[i]));
                let took_off = game.t;
                game.travel(dests[i]);
                self.flown = took_off..game.t;
            }
            Panel::Crime => {
                if i == game.warrant.len() {
                    game.compute_warrant();
                    let then = game.log[before..].to_vec();
                    self.panel = Panel::Scene(Scene { stages: vec![Stage::Compute], started: Instant::now(), cued: 0, then, print: true });
                    return;
                } else {
                    let fields = game.warrant_fields();
                    let (_, _, values) = &fields[i];
                    let now = game.warrant[i].1.clone();
                    // Each tap steps to the next value, and from the last back to unknown.
                    let next = match now.and_then(|v| values.iter().position(|x| same(x, &v))) {
                        None => values.first().cloned(),
                        Some(p) => values.get(p + 1).cloned(),
                    };
                    game.warrant[i].1 = next;
                    self.cues.push(Some(sound::KEY));
                    return;
                }
            }
            _ => return,
        }
        let then = game.log[before..].to_vec();
        self.play(stages, then);
    }

    // ------------------------------------------------------------ the story

    /// Shows what just happened; if the case ended with it, counts an arrest
    /// and says what it means for the detective's rank.
    fn tell(&mut self, mut lines: Vec<Line>) {
        // Everything said goes to the serial monitor too, to look back on.
        for line in &lines {
            match &line.speaker {
                Some(who) => log::info!("[{:?}] {who}: {}", line.tone, line.text),
                None => log::info!("[{:?}] {}", line.tone, line.text),
            }
        }
        let game = self.game.as_ref().expect("a case");
        let mut then = Then::Stay;
        if game.over {
            then = Then::NextCase;
            if game.won {
                let arrests = self.progress.add_arrest(&self.name);
                let news = promotion_news(arrests);
                lines.push(said(Tone::Plain, ""));
                lines.push(said(Tone::Good, &news));
                // The rank comes out on the printer too, as the original's ending did.
                self.printer = Printer::new();
                self.printer.print(&self.fonts.label, &news, Printer::text_width(PICTURE.inner().w));
            }
        }
        self.panel = Panel::Story { pager: Pager::new(ui::log_rows(&self.fonts.body, &lines, text_area().w)), then };
    }

    /// A new case for the detective at their rank, with the welcome and briefing.
    fn new_case(&mut self) {
        let arrests = self.progress.arrests(&self.name);
        let rank = RANKS[rank_for(arrests)];
        let mut game = Game::new(Rc::clone(&self.data), rank, Rng::new((self.seed)()), self.name.trim());
        // Headquarters' printer signs the detective in (or back in) and hands
        // over the case, as the desktop's does; the briefing is in the panel.
        let who = self.name.trim().to_string();
        if self.game.is_some() {
            self.hq_printer = Printer::new();
            self.hq_print(&format!("{}\n\n{}\n", game_title(), agency().to_uppercase()));
            self.hq_print(&tr(format!("Welcome back, {who}."), format!("Hola de nuevo, {who}.")));
        }
        let level = rank.label();
        self.hq_print(&tr(format!("\nYour rank: {level}."), format!("\nTu rango: {level}.")));
        self.hq_print(&tr(format!("\nA case for a {level}."), format!("\nUn caso de {level}.")));
        let lines = game.briefing();
        self.shown_t = game.t;
        self.game = Some(game);
        self.printer = Printer::new();
        self.printer.print(&self.fonts.label, ready(), Printer::text_width(PICTURE.inner().w));
        self.printer.finish();
        (self.picked, self.witness, self.flight, self.flown) = (None, None, None, 0..0);
        self.panel = Panel::Story { pager: Pager::new(ui::log_rows(&self.fonts.body, &lines, text_area().w)), then: Then::Arrive };
    }

    /// The first city: its name and what it's like.
    fn arrive(&mut self) {
        let game = self.game.as_ref().expect("a case");
        let country = game.country(game.here);
        let mut lines = vec![said(Tone::Heading, &country.label().to_uppercase())];
        if let Some(about) = &country.about {
            lines.push(said(Tone::Plain, about.get()));
        }
        self.panel = Panel::Story { pager: Pager::new(ui::log_rows(&self.fonts.body, &lines, text_area().w)), then: Then::Stay };
    }

    // ------------------------------------------------------------ drawing

    pub fn draw(&mut self) -> Canvas {
        let mut cv = Canvas::new(W as i32, H as i32);
        self.draw_clock(&mut cv);
        self.draw_picture(&mut cv);
        self.draw_panel(&mut cv);
        let on = self.buttons_on();
        for (i, &b) in BUTTON_ORDER.iter().enumerate() {
            ui::button(&mut cv, &self.fonts, b, BUTTONS[i], self.pressed == Some(Target::Button(b)), on);
        }
        cv
    }

    fn draw_clock(&self, cv: &mut Canvas) {
        ui::frame(cv, CLOCK, if self.pressed == Some(Target::Clock) { ui::YELLOW } else { ui::WHITE });
        let c = CLOCK.inner();
        let f = &self.fonts.body;
        let (text, colour) = match &self.game {
            Some(_) if self.at_hq() => (tr("Headquarters", "Cuartel general").to_string(), ui::WHITE),
            // Red when the deadline's close, as the desktop's clock box.
            Some(g) => (clock_text(self.shown_t), if !g.over && g.deadline - g.t <= 12 { ui::RED } else { ui::WHITE }),
            None => (tr("Headquarters", "Cuartel general").to_string(), ui::WHITE),
        };
        let text = f.wrap(&text, c.w - 2 * PAD).into_iter().next().unwrap_or_default();
        f.draw(cv, c.x + PAD, c.y + (c.h - f.line_height()) / 2 + f.ascent(), &text, colour);
        let tag = if lang() == Lang::En { "EN" } else { "ES" };
        self.fonts.label.draw(cv, c.x + c.w - self.fonts.label.width(tag) - 1, c.y + self.fonts.label.ascent(), tag, ui::GREY);
    }

    fn draw_picture(&mut self, cv: &mut Canvas) {
        ui::frame(cv, PICTURE, ui::WHITE);
        let p = PICTURE.inner();
        if self.at_hq() {
            // Headquarters: its printer, with the roster's button under it while signing in.
            cv.rect(p.x, p.y, p.w, p.h, ui::BLACK);
            let f = &self.fonts.label;
            let mut paper = p;
            if matches!(self.panel, Panel::Name) {
                let b = hq_roster_button();
                ui::frame(cv, b, if self.pressed == Some(Target::Roster) { ui::YELLOW } else { ui::WHITE });
                let label = tr("DETECTIVE ROSTER", "LISTA DE DETECTIVES");
                f.draw(cv, b.x + (b.w - f.width(label)) / 2, b.y + (b.h - f.line_height()) / 2 + f.ascent(), label, ui::WHITE);
                paper.h = b.y - 4 - p.y;
            }
            // The name echoes on the paper as it's typed, once the welcome's printed.
            let typing = matches!(self.panel, Panel::Name) && !self.hq_printer.busy();
            let echo = format!("{}_", self.name);
            self.hq_printer.draw(cv, f, paper, typing.then_some(echo.as_str()));
            return;
        }
        let Some(game) = &self.game else { return };
        let stage = match &self.panel {
            Panel::Scene(scene) => Some((&scene.stages[0], scene.started.elapsed().as_secs_f64())),
            _ => None,
        };
        let (x, y) = (p.x + (p.w - 2 * people::PORTRAIT_W) / 2, p.y + (p.h - 2 * people::PORTRAIT_H) / 2);
        match (&self.panel, stage) {
            (Panel::Dossier { who, .. }, _) => {
                // The photo at double size nearly fills the frame, as on the original's sheet.
                cv.rect(p.x, p.y, p.w, p.h, ui::BLACK);
                let suspect = &self.data.suspects[*who];
                if !sprites::photo(cv, &suspect.name, x, y, 2) {
                    blit(cv, &people::suspect_portrait(suspect), x, y, 2);
                }
                return;
            }
            (Panel::Crime, _) | (_, Some((Stage::Compute, _))) => {
                cv.rect(p.x, p.y, p.w, p.h, ui::BLACK);
                let computing = stage.map(|(_, t)| t);
                let screen = computer::draw(cv, p, computing);
                let f = &self.fonts.label;
                let (status, colour) = match computing {
                    // Blinking while it works.
                    Some(t) if t.fract() < 0.5 => ("", ui::LIME),
                    Some(_) => (tr("SEARCHING…", "BUSCANDO…"), ui::LIME),
                    None => (tr("Enter the clues.", "Cargá las pistas."), ui::GREY),
                };
                let lines = [(tr("CRIME", "COMPUTADORA"), ui::YELLOW), (tr("COMPUTER", "DE CRÍMENES"), ui::YELLOW), (status, colour)];
                for (k, (text, colour)) in lines.into_iter().enumerate() {
                    f.draw(cv, screen.x + 4, screen.y + 3 + k as i32 * f.line_height() + f.ascent(), text, colour);
                }
                self.printer.draw(cv, f, rect(p.x, p.y + computer::H + 4, p.w, p.h - computer::H - 4), None);
                return;
            }
            (Panel::Flights(dests), _) => {
                self.draw_map(cv, dests, None);
                return;
            }
            (_, Some((Stage::Fly(_), t))) => {
                if let Some((from, to)) = self.flight
                    && self.map.as_ref().is_some_and(|(city, _, _)| *city == from)
                {
                    let seconds = Stage::Fly(String::new()).seconds(scene_area());
                    self.draw_map(cv, &[], Some((from, to, (t / seconds).clamp(0.0, 1.0) as f32)));
                    return;
                }
            }
            (Panel::Places(places), _) => {
                self.draw_places(cv, places, None);
                return;
            }
            (_, Some((Stage::Walk(_), _))) => {
                if let Some((places, i)) = self.picked {
                    self.draw_places(cv, &places, Some(i));
                    return;
                }
            }
            (Panel::Story { then: Then::NextCase, .. }, _) if game.won => {
                cv.rect(p.x, p.y, p.w, p.h, ui::BLACK);
                self.printer.draw(cv, &self.fonts.label, p, None);
                return;
            }
            _ => {}
        }
        let key = (game.here, TimeOfDay::at(hour_of_day(game.t)));
        if self.postcard.as_ref().is_none_or(|(k, _)| *k != key) {
            let country = game.country(game.here);
            self.postcard = Some((key, postcard::fitted(&country.country, country.lat, key.1, p.w, p.h)));
        }
        if let Some((_, card)) = &self.postcard {
            for (i, &px) in card.pixels().iter().enumerate() {
                cv.set(p.x + i as i32 % p.w, p.y + i as i32 / p.w, px);
            }
        }
    }

    fn draw_panel(&self, cv: &mut Canvas) {
        ui::frame(cv, TEXT, ui::WHITE);
        let inner = TEXT.inner();
        cv.rect(inner.x, inner.y, inner.w, inner.h, ui::NAVY);
        let area = text_area();
        let f = &self.fonts;
        match &self.panel {
            Panel::Name => self.draw_keyboard(cv),
            Panel::Story { pager, .. } => {
                pager.draw(cv, &f.body, self.story_area());
                if let Some(w) = self.witness {
                    self.draw_witness(cv, w);
                }
            }
            Panel::Notes(pager) => pager.draw(cv, &f.body, area),
            Panel::Scene(scene) => scene.stages[0].draw(cv, f, scene_area(), scene.started.elapsed().as_secs_f64()),
            Panel::Roster { pager, .. } | Panel::Dossier { pager, .. } => pager.draw(cv, &f.body, area),
            Panel::Checking(_) => {
                for (k, line) in
                    f.body.wrap(tr("Checking the bureau's files…", "Revisando los archivos de la agencia…"), area.w).iter().enumerate()
                {
                    f.body.draw(cv, area.x, area.y + k as i32 * f.body.line_height() + f.body.ascent(), line, ui::GREY);
                }
            }
            Panel::Places(places) => {
                let game = self.game.as_ref().expect("a case");
                let visited = game.visited_here();
                let rows: Vec<(String, Rgb)> = places
                    .iter()
                    .enumerate()
                    .map(|(i, &p)| (PLACES[p].label().to_string(), if visited.contains(&i) { ui::GREY } else { ui::WHITE }))
                    .collect();
                self.draw_list(cv, tr("Where to investigate?", "¿Dónde investigar?"), &rows);
            }
            Panel::Flights(dests) => {
                let game = self.game.as_ref().expect("a case");
                let rows: Vec<(String, Rgb)> = dests.iter().map(|&d| (game.country(d).label(), ui::WHITE)).collect();
                self.draw_list(cv, tr("Fly to:", "Volar a:"), &rows);
            }
            Panel::Crime => {
                let game = self.game.as_ref().expect("a case");
                let mut rows: Vec<(String, Rgb)> = game
                    .warrant_fields()
                    .iter()
                    .zip(&game.warrant)
                    .map(|((key, label, _), (_, value))| match value {
                        Some(v) => (format!("{label}: {}", game.shown(key, v)), ui::WHITE),
                        None => (format!("{label}: ?"), ui::GREY),
                    })
                    .collect();
                rows.push((tr("COMPUTE", "CALCULAR").to_string(), ui::YELLOW));
                self.draw_list(cv, tr("Crime computer", "Computadora"), &rows);
                // The last row, split: the files on the left, the roster on the right.
                let y = list_top(f) + rows.len() as i32 * LIST_ROW;
                let half = area.w / 2;
                for (k, (label, target)) in
                    [(tr("FILES", "FICHAS"), Target::Files), (tr("ROSTER", "LISTA"), Target::Roster)].into_iter().enumerate()
                {
                    let x = area.x + k as i32 * half;
                    if self.pressed == Some(target) {
                        cv.rect(x, y, half - 2, LIST_ROW - 2, ui::BLUE);
                    }
                    cv.rect(x, y + LIST_ROW - 2, half - 2, 1, ui::DARK);
                    f.body.draw(cv, x + 1, y + (LIST_ROW - f.body.line_height()) / 2 + f.body.ascent(), label, ui::CYAN);
                }
            }
        }
    }

    /// Whether the detective is still at headquarters: signing in, or reading
    /// the briefing before the first city.
    fn at_hq(&self) -> bool {
        self.game.is_none() || matches!(self.panel, Panel::Name | Panel::Checking(_) | Panel::Story { then: Then::Arrive, .. })
    }

    /// The three places here stacked in the picture frame: the one pressed
    /// or being walked to framed in yellow, those visited (or not chosen) dimmed.
    fn draw_places(&self, cv: &mut Canvas, places: &[usize; 3], walking: Option<usize>) {
        let p = PICTURE.inner();
        cv.rect(p.x, p.y, p.w, p.h, ui::BLACK);
        let visited = self.game.as_ref().map_or(&[][..], |g| g.visited_here());
        for (i, &place) in places.iter().enumerate() {
            let slot = place_slot(i);
            let art = places::picture(PLACES[place].name);
            let lit = walking == Some(i) || self.pressed == Some(Target::Row(i));
            let dim = match walking {
                Some(w) => w != i,
                None => visited.contains(&i) && !lit,
            };
            // The picture's drawn rows, centred in the slot: a slot is a little shorter than the canvas.
            let rows: Vec<i32> = (0..places::ART_H).filter(|&y| (0..places::ART_W).any(|x| art.get(x, y) != Some(ui::BLACK))).collect();
            let (top, bottom) = (rows.first().copied().unwrap_or(0), rows.last().copied().unwrap_or(0));
            let dy = slot.y + (slot.h - (bottom - top + 1)) / 2 - top;
            let dx = slot.x + (slot.w - places::ART_W) / 2;
            for y in top.max(slot.y - dy)..=bottom.min(slot.y + slot.h - 1 - dy) {
                for x in 0..places::ART_W {
                    let c = art.get(x, y).unwrap_or(ui::BLACK);
                    cv.set(dx + x, dy + y, if dim { scale(c, 0.4) } else { c });
                }
            }
            if lit {
                cv.rect(slot.x, slot.y, slot.w, 1, ui::YELLOW);
                cv.rect(slot.x, slot.y + slot.h - 1, slot.w, 1, ui::YELLOW);
                cv.rect(slot.x, slot.y, 1, slot.h, ui::YELLOW);
                cv.rect(slot.x + slot.w - 1, slot.y, 1, slot.h, ui::YELLOW);
            }
        }
    }

    /// The departure map: the trail so far in yellow, the flights from here as
    /// red dots with their cities, and here blinking. `flying` draws a flight
    /// instead (from, to, how far along), the plane leaving a white trail.
    fn draw_map(&self, cv: &mut Canvas, dests: &[usize], flying: Option<(usize, usize, f32)>) {
        let p = PICTURE.inner();
        let (Some(game), Some((_, view, land))) = (&self.game, &self.map) else { return };
        // Drawn on a copy, which keeps lines and labels inside the frame.
        let mut m = Canvas::new(p.w, p.h);
        for (i, &px) in land.pixels().iter().enumerate() {
            m.set(i as i32 % p.w, i as i32 / p.w, px);
        }
        let at = |c: usize| {
            let (x, y) = view.project(game.country(c).lat, game.country(c).lon);
            (x.round() as i32, y.round() as i32)
        };
        // In the air, the path already ends where the plane is going.
        let flown = game.path.len() - usize::from(flying.is_some());
        for pair in game.path[..flown].windows(2) {
            let ((x0, y0), (x1, y1)) = (at(pair[0]), at(pair[1]));
            m.line(x0, y0, x1, y1, ui::YELLOW);
        }
        let f = &self.fonts.label;
        let dot = |m: &mut Canvas, (x, y): (i32, i32), c: Rgb| {
            m.disc(x, y, 3, ui::BLACK);
            m.disc(x, y, 2, c);
        };
        if let Some((from, to, t)) = flying {
            let ((x0, y0), (x1, y1)) = (at(from), at(to));
            let t = t * t * (3.0 - 2.0 * t);
            let (x, y) = (x0 + ((x1 - x0) as f32 * t) as i32, y0 + ((y1 - y0) as f32 * t) as i32);
            m.line(x0, y0, x, y, ui::WHITE);
            dot(&mut m, (x1, y1), ui::YELLOW);
            map_label(&mut m, f, (x1, y1), game.country(to).city(), ui::YELLOW);
            m.disc(x, y, 2, ui::WHITE);
        } else {
            for (i, &d) in dests.iter().enumerate() {
                let lit = self.pressed == Some(Target::Row(i));
                dot(&mut m, at(d), if lit { ui::YELLOW } else { ui::RED });
                map_label(&mut m, f, at(d), game.country(d).city(), if lit { ui::YELLOW } else { ui::WHITE });
            }
            dot(&mut m, at(game.here), ui::YELLOW);
        }
        blit(cv, &m, p.x, p.y, 1);
    }

    /// Where a story's text goes: above the witness, if one is talking.
    fn story_area(&self) -> Rect {
        let area = text_area();
        match self.witness {
            Some(_) => rect(area.x, area.y, area.w, witness_top() - 2 - area.y),
            None => area,
        }
    }

    /// The witness in the bottom left corner of the text panel, as the
    /// original showed them, with their job beside them.
    fn draw_witness(&self, cv: &mut Canvas, w: usize) {
        let inner = TEXT.inner();
        let (x, y) = (inner.x + 2, witness_top());
        let portrait = people::witness(PLACES[w].witness);
        // At half size, each pixel the mean of a 2x2 block; the portrait's
        // black background lets the panel's blue through.
        for by in 0..WITNESS_H {
            for bx in 0..WITNESS_W {
                let block: Vec<Rgb> = [(0, 0), (1, 0), (0, 1), (1, 1)]
                    .iter()
                    .filter_map(|&(dx, dy)| portrait.get(2 * bx + dx, 2 * by + dy))
                    .filter(|&c| c != ui::BLACK)
                    .collect();
                if block.len() >= 2 {
                    let mean = [0, 1, 2].map(|k| (block.iter().map(|c| c[k] as u32).sum::<u32>() / block.len() as u32) as u8);
                    cv.set(x + bx, y + by, mean);
                }
            }
        }
        let f = &self.fonts.label;
        let (jx, width) = (x + WITNESS_W + 4, inner.x + inner.w - PAD - (x + WITNESS_W + 4));
        let lines = f.wrap(PLACES[w].witness_label(), width);
        let top = inner.y + inner.h - PAD - lines.len() as i32 * f.line_height();
        for (k, line) in lines.iter().enumerate() {
            f.draw(cv, jx, top + k as i32 * f.line_height() + f.ascent(), line, ui::CYAN);
        }
    }

    /// A heading, then rows to tap, the pressed one highlighted.
    fn draw_list(&self, cv: &mut Canvas, heading: &str, rows: &[(String, Rgb)]) {
        let (f, area) = (&self.fonts.body, text_area());
        f.draw(cv, area.x, area.y + f.ascent(), heading, ui::YELLOW);
        let top = list_top(&self.fonts);
        for (i, (text, colour)) in rows.iter().enumerate() {
            let y = top + i as i32 * LIST_ROW;
            if self.pressed == Some(Target::Row(i)) {
                cv.rect(area.x - 1, y, area.w + 2, LIST_ROW - 2, ui::BLUE);
            }
            cv.rect(area.x, y + LIST_ROW - 2, area.w, 1, ui::DARK);
            // A long name is cut short to keep each row one line.
            let text = f.wrap(text, area.w - 2).into_iter().next().unwrap_or_default();
            f.draw(cv, area.x + 1, y + (LIST_ROW - f.line_height()) / 2 + f.ascent(), &text, *colour);
        }
    }

    fn draw_keyboard(&self, cv: &mut Canvas) {
        let (f, area) = (&self.fonts, text_area());
        let (kw, kh) = key_size();
        let top = keyboard_top();
        f.body.draw(cv, area.x, area.y + f.body.ascent(), tr("Your name:", "Tu nombre:"), ui::YELLOW);
        for (k, &label) in KEYS.iter().enumerate() {
            let (col, row) = (k as i32 % KEY_COLUMNS, k as i32 / KEY_COLUMNS);
            let key = rect(area.x + col * kw, top + row * kh, kw - 2, kh - 2);
            let pressed = self.pressed == Some(Target::Key(k));
            cv.rect(key.x, key.y, key.w, key.h, if pressed { ui::YELLOW } else { ui::BLUE });
            let label = match label {
                "LANG" => tr("ES", "EN"),
                "⌫" => "<",
                l => l,
            };
            let font = if label.chars().count() > 1 { &f.label } else { &f.body };
            let ink = if pressed { ui::NAVY } else { ui::WHITE };
            font.draw(cv, key.x + (key.w - font.width(label)) / 2, key.y + (key.h - font.line_height()) / 2 + font.ascent(), label, ink);
        }
    }
}

/// What the crime computer's printer says when it's waiting.
fn ready() -> &'static str {
    tr("READY.", "LISTO.")
}

/// The top of a talking witness's portrait, which stands on the text panel's bottom edge.
fn witness_top() -> i32 {
    let inner = TEXT.inner();
    inner.y + inner.h - WITNESS_H
}

/// A talking witness's portrait, at half its size.
const WITNESS_W: i32 = people::PORTRAIT_W / 2;
const WITNESS_H: i32 = people::PORTRAIT_H / 2;

/// Place `i`'s picture in the picture frame, three stacked.
fn place_slot(i: usize) -> Rect {
    let p = PICTURE.inner();
    let h = p.h / 3;
    rect(p.x, p.y + i as i32 * h, p.w, h)
}

/// `src` drawn at (x, y), each pixel a `k` square.
fn blit(cv: &mut Canvas, src: &Canvas, x: i32, y: i32, k: i32) {
    for (i, &c) in src.pixels().iter().enumerate() {
        let (sx, sy) = (i as i32 % src.width(), i as i32 / src.width());
        cv.rect(x + sx * k, y + sy * k, k, k, c);
    }
}

/// A city's name beside its dot on the map, outlined in black so it reads on land and sea.
fn map_label(m: &mut Canvas, f: &Font, (x, y): (i32, i32), text: &str, colour: Rgb) {
    let w = f.width(text);
    let lx = if x + 5 + w < m.width() { x + 5 } else { x - 5 - w };
    let ly = (y + f.ascent() / 2).clamp(f.ascent() + 1, m.height() - 3);
    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        f.draw(m, lx + dx, ly + dy, text, ui::BLACK);
    }
    f.draw(m, lx, ly, text, colour);
}

/// Where scenes play: the whole of the text panel inside its frame, so the
/// characters stand on its bottom edge.
fn scene_area() -> Rect {
    TEXT.inner()
}

/// The roster's button at headquarters, at the bottom of the picture frame.
fn hq_roster_button() -> Rect {
    let p = PICTURE.inner();
    rect(p.x + PAD, p.y + p.h - 30, p.w - 2 * PAD, 26)
}

/// Where the text panel's text goes.
fn text_area() -> Rect {
    let i = TEXT.inner();
    rect(i.x + PAD, i.y + PAD, i.w - 2 * PAD, i.h - 2 * PAD)
}

fn list_top(f: &Fonts) -> i32 {
    text_area().y + f.body.line_height() + 4
}

fn key_size() -> (i32, i32) {
    let area = text_area();
    let rows = KEYS.len().div_ceil(KEY_COLUMNS as usize) as i32;
    (area.w / KEY_COLUMNS, (area.y + area.h - keyboard_top()) / rows)
}

fn keyboard_top() -> i32 {
    text_area().y + 20
}

/// The notes: the deadline, the warrant, then everything said in the case.
fn notes_rows(f: &Fonts, game: &Game) -> Vec<Row> {
    let width = text_area().w;
    let left = (game.deadline - game.t).max(0);
    let due = clock_text(game.deadline);
    let mut lines = vec![
        said(Tone::Heading, tr("CASE NOTES", "NOTAS DEL CASO")),
        said(
            if left <= 12 { Tone::Bad } else { Tone::Plain },
            &tr(format!("Deadline {due} ({left} hours left)"), format!("Vence el {due} ({left} h)")),
        ),
        said(
            Tone::Dim,
            &match game.warrant_for {
                Some(s) => tr(format!("Warrant: {}", game.data.suspects[s].name), format!("Orden: {}", game.data.suspects[s].name)),
                None => tr("Warrant: none", "Orden: ninguna").to_string(),
            },
        ),
        said(Tone::Plain, ""),
    ];
    lines.extend(game.log.iter().cloned());
    ui::log_rows(&f.body, &lines, width)
}

fn said(tone: Tone, text: &str) -> Line {
    Line { tone, speaker: None, text: text.to_string() }
}
