#![windows_subsystem = "windows"]

use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};

use quickclock::render::{self, Fonts, BG, FG};
use quickclock::single::{SingleInstance, acquire_or_raise};
use quickclock::timefmt;
use quickclock::{center_offset, scaled};
use chrono::Timelike;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Fullscreen, Window};

const MUTEX_NAME: &str = "QuickClock_SingleInstance";
const WINDOW_TITLE: &str = "QuickClock";

struct App {
    window: Option<Arc<Window>>,
    fonts: Option<Fonts>,
    surface: Option<softbuffer::Surface<Arc<Window>, Arc<Window>>>,
    context: Option<softbuffer::Context<Arc<Window>>>,
    last_second: u32,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            fonts: None,
            surface: None,
            context: None,
            last_second: u32::MAX,
        }
    }

    fn draw(&mut self) {
        let Some(window) = &self.window else { return };
        let Some(fonts) = &self.fonts else { return };
        let Some(surface) = &mut self.surface else { return };

        let size = window.inner_size();
        let (w, h) = (size.width.max(1), size.height.max(1));
        // Transient surface failures (display topology change, session
        // events) skip this frame; the next second retries.
        if surface
            .resize(NonZeroU32::new(w).unwrap(), NonZeroU32::new(h).unwrap())
            .is_err()
        {
            return;
        }
        let mut buffer = match surface.buffer_mut() {
            Ok(buffer) => buffer,
            Err(_) => return,
        };
        buffer.fill(BG);

        let sf = window.scale_factor();
        let now = timefmt::now_local();
        let time_s = timefmt::format_time(&now.time());
        let date_s = timefmt::format_date(&now.date_naive());
        let day_s = timefmt::weekday_name(&now.date_naive());

        let time_px = scaled(150.0, sf);
        let date_px = scaled(42.0, sf);
        let day_px = scaled(28.0, sf);

        let (tw, th) = render::measure(&time_s, &fonts.mono, time_px);
        let (dw, dh) = render::measure(&date_s, &fonts.cjk, date_px);
        let (yw, yh) = render::measure(&day_s, &fonts.cjk, day_px);

        let margin = (10.0 * sf) as i32;

        // Time: screen center.
        render::draw_text(
            &mut buffer, w, &time_s, &fonts.mono, time_px, FG,
            center_offset(w as i32, tw as i32),
            center_offset(h as i32, th as i32),
        );

        // Date: bottom center with margin, whole bottom block raised 50px.
        let date_y = h as i32 - dh as i32 - margin - 50;
        render::draw_text(
            &mut buffer, w, &date_s, &fonts.cjk, date_px, FG,
            center_offset(w as i32, dw as i32),
            date_y,
        );

        // Weekday: above the date.
        let day_y = date_y - yh as i32 - margin / 2;
        render::draw_text(
            &mut buffer, w, &day_s, &fonts.cjk, day_px, FG,
            center_offset(w as i32, yw as i32),
            day_y,
        );

        if buffer.present().is_err() {
            // Frame skipped; the next second retries.
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        // Fonts first: a fatal font error must not flash a fullscreen window.
        self.fonts = Some(match render::load_fonts() {
            Some(fonts) => fonts,
            None => fail("Failed to load system fonts (Consolas / Microsoft YaHei)."),
        });
        let window = match event_loop.create_window(
            Window::default_attributes()
                .with_title(WINDOW_TITLE)
                .with_fullscreen(Some(Fullscreen::Borderless(None)))
                // Hidden until the first frame is painted: avoids the
                // default-white flash between window creation and first draw.
                .with_visible(false),
        ) {
            Ok(window) => window,
            Err(_) => fail("Failed to create window."),
        };
        let window = Arc::new(window);
        let context = match softbuffer::Context::new(window.clone()) {
            Ok(context) => context,
            Err(_) => fail("Failed to initialize graphics context."),
        };
        let surface = match softbuffer::Surface::new(&context, window.clone()) {
            Ok(surface) => surface,
            Err(_) => fail("Failed to initialize graphics surface."),
        };
        self.window = Some(window);
        self.context = Some(context);
        self.surface = Some(surface);
        // Paint the first frame while hidden, then reveal and schedule a
        // fresh redraw in case the pre-show present did not stick.
        self.draw();
        if let Some(window) = &self.window {
            window.set_visible(true);
            window.request_redraw();
        }
    }

    fn window_event(&mut self, _event_loop: &ActiveEventLoop, _id: winit::window::WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => std::process::exit(0),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } if matches!(code, KeyCode::Escape | KeyCode::KeyQ) => std::process::exit(0),
            WindowEvent::RedrawRequested => self.draw(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = timefmt::now_local();
        // Wake up at the next whole second.
        let millis_to_next = 1000 - now.timestamp_subsec_millis();
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(millis_to_next as u64),
        ));
        let sec = now.second();
        if sec != self.last_second {
            self.last_second = sec;
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }
}

/// Show a fatal-error dialog and exit with a nonzero code (spec section 6).
fn fail(message: &str) -> ! {
    error_box("QuickClock", message);
    std::process::exit(1);
}

fn error_box(title: &str, message: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW;
    let wide = |s: &str| -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() };
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            wide(message).as_ptr(),
            wide(title).as_ptr(),
            0, // MB_OK
        );
    }
}

fn main() {
    match acquire_or_raise(MUTEX_NAME, WINDOW_TITLE) {
        SingleInstance::First => {}
        // Another instance is running and has been raised; exit quietly.
        SingleInstance::AlreadyRunning => return,
        SingleInstance::MutexFailed => fail("Failed to create single-instance mutex."),
    }
    let event_loop = EventLoop::new().expect("create event loop");
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::new();
    event_loop.run_app(&mut app).expect("run event loop");
}
