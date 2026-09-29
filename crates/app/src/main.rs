//! Lost Dutchman Mine: the desktop application.
//!
//! The game runs on its own thread in real time ([`simulation`]). This thread
//! owns the window: it turns SDL events into game input through the `desktop`
//! controller, draws the latest frame or the settings menu at the monitor's
//! refresh rate, and feeds the sound device. Beside the executable live the
//! player's game (`Game`), their saves (`Saves`) and the settings file.

#![allow(
    clippy::cast_precision_loss,
    reason = "screen coordinates are far below 2^24, where f32 represents them exactly"
)]
#![windows_subsystem = "windows"]

mod crash;
mod presentation;
mod simulation;
mod sound;

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use desktop::controller::{Button, Controller};
use desktop::keys::Key;
use desktop::pixels::Pixels;
use engine::Engine;
use sdl2::event::{Event, WindowEvent};
use sdl2::messagebox::{MessageBoxFlag, show_simple_message_box};
use sdl2::mouse::MouseButton;

use crate::presentation::Presentation;
use crate::simulation::Simulation;
use crate::sound::Speakers;

const TITLE: &str = "Lost Dutchman Mine";
const USAGE: &str = "usage: lost-dutchman-mine [--data DIR] [--saves DIR] [--config FILE] \
                     [--settings | --no-settings] [--seconds N] [--screenshot FILE]";

/// Where things are, and whether the settings menu opens at launch. A run
/// may end after a number of seconds with a screenshot of the window, so the
/// whole presentation can be checked without a player.
struct Options {
    crashes: PathBuf,
    data: PathBuf,
    saves: PathBuf,
    config: PathBuf,
    menu: Option<bool>,
    seconds: Option<u64>,
    screenshot: Option<PathBuf>,
}

impl Options {
    fn from_args() -> Result<Self> {
        let exe = std::env::current_exe().context("cannot find the executable")?;
        let app = exe.parent().map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        let mut options = Options {
            crashes: app.join("Crashes"),
            data: app.join("Game"),
            saves: app.join("Saves"),
            config: app.join("display.ini"),
            menu: None,
            seconds: None,
            screenshot: None,
        };
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            let mut value =
                || args.next().map(PathBuf::from).ok_or_else(|| anyhow!("{arg} needs a value"));
            match arg.as_str() {
                "--data" => options.data = value()?,
                "--saves" => options.saves = value()?,
                "--config" => options.config = value()?,
                "--settings" => options.menu = Some(true),
                "--no-settings" => options.menu = Some(false),
                "--seconds" => {
                    let seconds = value()?;
                    let seconds =
                        seconds.to_string_lossy().parse().context("--seconds needs a number")?;
                    options.seconds = Some(seconds);
                }
                "--screenshot" => options.screenshot = Some(value()?),
                _ => bail!("unknown argument {arg}\n{USAGE}"),
            }
        }
        Ok(options)
    }
}

fn main() -> ExitCode {
    let result = Options::from_args().and_then(|options| run(&options));
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error:#}");
            let _ =
                show_simple_message_box(MessageBoxFlag::ERROR, TITLE, &format!("{error:#}"), None);
            ExitCode::FAILURE
        }
    }
}

/// The host's local time as seconds since 1970 in that local calendar: the
/// game keeps DOS time, which has no time zones.
fn local_clock_seconds() -> i64 {
    chrono::Local::now().naive_local().and_utc().timestamp()
}

fn sdl<T>(result: Result<T, String>) -> Result<T> {
    result.map_err(|error| anyhow!(error))
}

fn run(options: &Options) -> Result<()> {
    let canonical = |path: &Path| path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if canonical(&options.data) == canonical(&options.saves) {
        bail!("the saves folder must differ from the original game folder");
    }
    let settings = desktop::settings::load(&options.config);
    let startup_menu = options.menu.unwrap_or(settings.startup);
    let (machine, mut game) =
        game::boot(options.data.clone(), options.saves.clone(), settings.qol)?;
    let clock_base = local_clock_seconds();
    game.dos.clock.base = clock_base;
    // Play runs the readable routines; lockstep proves them equal to the originals.
    game.set_routine_mode(game::RoutineMode::Readable);

    sdl2::hint::set("SDL_WINDOWS_DPI_AWARENESS", "permonitorv2");
    sdl2::hint::set("SDL_MOUSE_FOCUS_CLICKTHROUGH", "1");
    let context = sdl(sdl2::init())?;
    let video = sdl(context.video())?;
    let audio = context.audio().map_err(|error| eprintln!("no sound: {error}")).ok();
    let mut builder = video.window(TITLE, 1040, 780);
    builder.position_centered().resizable().allow_highdpi();
    if settings.window == desktop::settings::FULLSCREEN {
        builder.fullscreen_desktop();
    }
    let mut window = builder.build()?;
    window.set_minimum_size(640, 480)?;
    window.set_icon(presentation::bitmap(presentation::ICON)?);
    let canvas = window.into_canvas().present_vsync().build()?;
    eprintln!("Display renderer: {}", canvas.info().name);
    let creator = canvas.texture_creator();
    let mut view = Presentation::new(canvas, video, &creator)?;
    view.resize(settings)?;

    let mut controller = Controller::new(options.config.clone(), settings, startup_menu);
    let speakers = Speakers::open(audio.as_ref());
    let header = vec![
        ("build".to_owned(), game::build_id()),
        ("clock-base".to_owned(), clock_base.to_string()),
        ("qol".to_owned(), u8::from(settings.qol).to_string()),
        ("routines".to_owned(), "readable".to_owned()),
    ];
    let simulation = Simulation::start(Engine::new(machine, game), options.crashes.clone(), header);
    let mut events = sdl(context.event_pump())?;
    let mouse = context.mouse();
    let start = Instant::now();
    let mut next_frame = 0.0;
    let mut window_setting = settings.window;
    let mut frame: Box<Pixels> = desktop::pixels::filled(0xff00_0000);
    let limit = options.seconds.map(|seconds| seconds as f64 * 1000.0);
    loop {
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        if limit.is_some_and(|limit| elapsed >= limit) {
            break;
        }
        let latest = simulation.latest();
        if let Some(snapshot) = &latest {
            if let Some(error) = &snapshot.error {
                bail!("{error}");
            }
            if snapshot.finished {
                break;
            }
            frame.copy_from_slice(snapshot.frame.pixels());
        }
        controller.tick(elapsed as u64, latest.as_ref().map_or(3, |snapshot| snapshot.video_mode));
        for event in events.poll_iter() {
            if matches!(event, Event::Quit { .. }) {
                return Ok(());
            }
            handle(&event, &mut controller, &mut view);
        }
        simulation.send(controller.take_inputs());
        simulation.pause(controller.paused());
        speakers.pause(controller.menu_open());
        speakers.play(simulation.take_sound());
        if controller.quit() {
            break;
        }
        if controller.settings().window != window_setting {
            window_setting = controller.settings().window;
            view.resize(*controller.settings())?;
        }
        if elapsed >= next_frame {
            let custom_cursor = latest.as_ref().is_some_and(|snapshot| snapshot.custom_cursor);
            mouse.show_cursor(controller.menu_open() || !custom_cursor);
            draw(&mut view, &controller, &frame)?;
            view.present();
            let interval = 1000.0 / f64::from(view.refresh_rate().clamp(30, 240));
            next_frame = f64::max(next_frame + interval, elapsed + interval * 0.1);
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    if let Some(file) = &options.screenshot {
        draw(&mut view, &controller, &frame)?;
        view.capture(file)?;
    }
    Ok(())
}

/// The settings menu over its preview, or the game.
fn draw(view: &mut Presentation, controller: &Controller, frame: &Pixels) -> Result<()> {
    match controller.selected() {
        Some(selected) => {
            let startup = controller.startup_menu();
            view.menu(frame, *controller.preview(), startup, selected, controller.message())
        }
        None => view.game(frame, *controller.settings()),
    }
}

/// One SDL event, as the controller understands it.
fn handle(event: &Event, controller: &mut Controller, view: &mut Presentation) {
    match *event {
        Event::KeyDown { scancode: Some(scancode), keycode, keymod, repeat, .. }
        | Event::KeyUp { scancode: Some(scancode), keycode, keymod, repeat, .. } => {
            let key = Key {
                scancode: scancode as i32 as u16,
                keycode: keycode.map_or(0, |keycode| keycode.into_i32() as u32),
                mods: keymod.bits(),
                repeat,
            };
            controller.key(key, matches!(event, Event::KeyDown { .. }));
        }
        Event::Window { win_event: WindowEvent::FocusLost, .. } => controller.focus_lost(),
        Event::MouseMotion { x, y, .. } => pointer(controller, view, (x, y), None),
        Event::MouseButtonDown { x, y, mouse_btn, .. } => {
            pointer(controller, view, (x, y), Some((button(mouse_btn), true)));
        }
        Event::MouseButtonUp { x, y, mouse_btn, .. } => {
            pointer(controller, view, (x, y), Some((button(mouse_btn), false)));
        }
        _ => {}
    }
}

fn button(button: MouseButton) -> Button {
    match button {
        MouseButton::Left => Button::Left,
        MouseButton::Right => Button::Right,
        _ => Button::Other,
    }
}

/// The pointer at a window point: over the menu's items, or over the picture.
fn pointer(
    controller: &mut Controller,
    view: &mut Presentation,
    at: (i32, i32),
    button: Option<(Button, bool)>,
) {
    if controller.menu_open() {
        let left_press = button == Some((Button::Left, true));
        controller.menu_pointer(view.menu_hit(at), left_press);
        return;
    }
    let Some((point, inside)) = view.game_point(at, *controller.settings()) else { return };
    match button {
        Some(button) => controller.pointer(point, inside, Some(button)),
        None => controller.pointer_moved(point),
    }
}
