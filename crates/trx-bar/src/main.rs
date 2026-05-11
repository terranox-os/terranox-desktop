// SPDX-License-Identifier: GPL-2.0-only
/*
 * trx-bar — TerranoxOS status bar (Rust)
 * Author: Antonette Caldwell
 *
 * Minimal Wayland status bar using xdg_toplevel.
 * Renders workspaces, window title, Sentinel status, and clock
 * using a built-in bitmap font — no fontconfig/fcft dependency.
 *
 * Dependencies: wayland-client (pure Rust), rustix (memfd/mmap)
 * Reference: Wayland protocol spec, TerranoxDesktop.jsx design
 * Implementation: original
 */

mod font;
mod render;

use std::os::fd::{AsFd, OwnedFd};
use wayland_client::{
    Connection, Dispatch, QueueHandle,
    protocol::{
        wl_buffer, wl_callback, wl_compositor, wl_registry, wl_shm,
        wl_shm_pool, wl_surface,
    },
};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};

// ── App state ──────────────────────────────────────────────

struct App {
    compositor: Option<wl_compositor::WlCompositor>,
    wm_base: Option<xdg_wm_base::XdgWmBase>,
    shm: Option<wl_shm::WlShm>,
    surface: Option<wl_surface::WlSurface>,
    xdg_surface: Option<xdg_surface::XdgSurface>,
    xdg_toplevel: Option<xdg_toplevel::XdgToplevel>,
    buffer: Option<wl_buffer::WlBuffer>,
    shm_data: Option<ShmBuffer>,
    configured: bool,
    running: bool,
}

struct ShmBuffer {
    ptr: *mut u32,
    len: usize,
    _fd: OwnedFd,
}

// SAFETY: SHM buffer is only accessed from the main thread.
unsafe impl Send for ShmBuffer {}

impl ShmBuffer {
    fn create(size: usize) -> Option<Self> {
        use rustix::fs::{memfd_create, MemfdFlags};
        use rustix::mm::{MapFlags, ProtFlags, mmap};

        let fd = memfd_create("trx-bar", MemfdFlags::CLOEXEC).ok()?;
        rustix::fs::ftruncate(&fd, size as u64).ok()?;

        let ptr = unsafe {
            mmap(
                std::ptr::null_mut(),
                size,
                ProtFlags::READ | ProtFlags::WRITE,
                MapFlags::SHARED,
                &fd,
                0,
            ).ok()?
        };

        Some(ShmBuffer {
            ptr: ptr as *mut u32,
            len: size / 4,
            _fd: fd,
        })
    }

    fn pixels(&mut self) -> &mut [u32] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }

    fn fd(&self) -> &OwnedFd {
        &self._fd
    }
}

impl App {
    fn new() -> Self {
        Self {
            compositor: None,
            wm_base: None,
            shm: None,
            surface: None,
            xdg_surface: None,
            xdg_toplevel: None,
            buffer: None,
            shm_data: None,
            configured: false,
            running: true,
        }
    }

    fn setup_surface(&mut self, qh: &QueueHandle<Self>) {
        let compositor = self.compositor.as_ref().expect("no compositor");
        let wm_base = self.wm_base.as_ref().expect("no xdg_wm_base");
        let shm = self.shm.as_ref().expect("no wl_shm");

        // Create surface
        let surface = compositor.create_surface(qh, ());
        let xdg_surface = wm_base.get_xdg_surface(&surface, qh, ());
        let xdg_toplevel = xdg_surface.get_toplevel(qh, ());
        xdg_toplevel.set_title("trx-bar".into());
        xdg_toplevel.set_app_id("trx-bar".into());

        // Create SHM buffer
        let stride = render::BAR_W * 4;
        let size = stride * render::BAR_H;
        let shm_buf = ShmBuffer::create(size).expect("failed to create SHM buffer");

        let pool = shm.create_pool(shm_buf.fd().as_fd(), size as i32, qh, ());
        let buffer = pool.create_buffer(
            0,
            render::BAR_W as i32,
            render::BAR_H as i32,
            stride as i32,
            wl_shm::Format::Argb8888,
            qh,
            (),
        );
        pool.destroy();

        surface.commit();

        self.surface = Some(surface);
        self.xdg_surface = Some(xdg_surface);
        self.xdg_toplevel = Some(xdg_toplevel);
        self.buffer = Some(buffer);
        self.shm_data = Some(shm_buf);
    }

    fn render(&mut self) {
        let Some(ref mut shm_data) = self.shm_data else { return };
        let Some(ref surface) = self.surface else { return };
        let Some(ref buffer) = self.buffer else { return };

        // Format uptime clock (kernel may not have RTC)
        let secs = {
            let ts = rustix::time::clock_gettime(rustix::time::ClockId::Monotonic);
            ts.tv_sec
        };
        let h = (secs / 3600) % 100;
        let m = (secs / 60) % 60;
        let s = secs % 60;
        let clock = format!("up {:02}:{:02}:{:02}", h, m, s);

        render::render_bar(shm_data.pixels(), &clock);

        surface.attach(Some(buffer), 0, 0);
        surface.damage_buffer(0, 0, render::BAR_W as i32, render::BAR_H as i32);
        surface.commit();
    }
}

// ── Wayland dispatch implementations ───────────────────────

impl Dispatch<wl_registry::WlRegistry, ()> for App {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global { name, interface, version } = event {
            match interface.as_str() {
                "wl_compositor" => {
                    state.compositor = Some(registry.bind(name, version.min(6), qh, ()));
                }
                "xdg_wm_base" => {
                    state.wm_base = Some(registry.bind(name, version.min(6), qh, ()));
                }
                "wl_shm" => {
                    state.shm = Some(registry.bind(name, version.min(1), qh, ()));
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<xdg_wm_base::XdgWmBase, ()> for App {
    fn event(
        _: &mut Self,
        wm_base: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            wm_base.pong(serial);
        }
    }
}

impl Dispatch<xdg_surface::XdgSurface, ()> for App {
    fn event(
        state: &mut Self,
        xdg_surface: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            xdg_surface.ack_configure(serial);
            state.configured = true;
            state.render();
            eprintln!("trx-bar: configure (serial={}), rendered", serial);
        }
    }
}

impl Dispatch<xdg_toplevel::XdgToplevel, ()> for App {
    fn event(
        state: &mut Self,
        _: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_toplevel::Event::Close = event {
            state.running = false;
        }
    }
}

// No-op dispatchers for objects we don't need events from
impl Dispatch<wl_compositor::WlCompositor, ()> for App {
    fn event(_: &mut Self, _: &wl_compositor::WlCompositor, _: wl_compositor::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}
impl Dispatch<wl_shm::WlShm, ()> for App {
    fn event(_: &mut Self, _: &wl_shm::WlShm, _: wl_shm::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}
impl Dispatch<wl_shm_pool::WlShmPool, ()> for App {
    fn event(_: &mut Self, _: &wl_shm_pool::WlShmPool, _: wl_shm_pool::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}
impl Dispatch<wl_surface::WlSurface, ()> for App {
    fn event(_: &mut Self, _: &wl_surface::WlSurface, _: wl_surface::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}
impl Dispatch<wl_buffer::WlBuffer, ()> for App {
    fn event(_: &mut Self, _: &wl_buffer::WlBuffer, _: wl_buffer::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}
impl Dispatch<wl_callback::WlCallback, ()> for App {
    fn event(_: &mut Self, _: &wl_callback::WlCallback, _: wl_callback::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

// ── Main ───────────────────────────────────────────────────

fn main() {
    eprintln!("trx-bar: starting (Rust)");

    // Set WAYLAND_DISPLAY if not already set (our init doesn't set it)
    // SAFETY: single-threaded, before any Wayland operations
    unsafe {
        if std::env::var("WAYLAND_DISPLAY").is_err() {
            std::env::set_var("WAYLAND_DISPLAY", "wayland-0");
        }
        if std::env::var("XDG_RUNTIME_DIR").is_err() {
            std::env::set_var("XDG_RUNTIME_DIR", "/run");
        }
    }

    // Retry connection — compositor may not have created the socket yet
    let conn = {
        let mut attempts = 0;
        loop {
            match Connection::connect_to_env() {
                Ok(c) => {
                    eprintln!("trx-bar: connected to compositor");
                    break c;
                }
                Err(e) => {
                    attempts += 1;
                    if attempts >= 50 {
                        eprintln!("trx-bar: giving up after {attempts} attempts: {e}");
                        return;
                    }
                    let ts = rustix::time::Timespec { tv_sec: 0, tv_nsec: 100_000_000 };
                    let _ = rustix::thread::nanosleep(&ts);
                }
            }
        }
    };
    let display = conn.display();

    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    let mut app = App::new();

    // Get registry + roundtrip for globals
    display.get_registry(&qh, ());
    event_queue.roundtrip(&mut app).expect("roundtrip failed");

    if app.compositor.is_none() || app.wm_base.is_none() || app.shm.is_none() {
        eprintln!("trx-bar: missing required globals");
        return;
    }

    // Create surface + buffer
    app.setup_surface(&qh);
    event_queue.roundtrip(&mut app).expect("roundtrip failed");

    eprintln!("trx-bar: entering event loop");

    // Event loop
    while app.running {
        // Re-render for clock updates
        if app.configured {
            app.render();
        }

        // Flush + dispatch with ~1 second timeout
        conn.flush().ok();
        event_queue.dispatch_pending(&mut app).ok();

        // Poll the Wayland fd with 1-second timeout
        let fd = conn.as_fd();
        let mut pollfd = [rustix::event::PollFd::new(&fd, rustix::event::PollFlags::IN)];
        let timeout = rustix::time::Timespec { tv_sec: 1, tv_nsec: 0 };
        let _ = rustix::event::poll(&mut pollfd, Some(&timeout));

        if pollfd[0].revents().contains(rustix::event::PollFlags::IN) {
            event_queue.dispatch_pending(&mut app).ok();
        }
    }
}
