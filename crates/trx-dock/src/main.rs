// SPDX-License-Identifier: GPL-2.0-only
//! trx-dock — TerranoxOS bottom dock panel
//!
//! Layer-shell client that displays app icons at the bottom of the screen.
//! Uses wlr-layer-shell-v1 protocol with bottom anchor and exclusive zone.
//!
//! Layout: centered horizontal row of app icons (64x64), translucent bg.
//! Click → launch or focus the corresponding application.
//!
//! Author: Antonette Caldwell
//! Reference: wlr-layer-shell-v1 protocol, TerranoxDesktop design

use std::os::fd::{AsFd, OwnedFd};
use wayland_client::{
    Connection, Dispatch, QueueHandle,
    protocol::{
        wl_buffer, wl_callback, wl_compositor, wl_registry, wl_shm,
        wl_shm_pool, wl_surface, wl_output, wl_seat, wl_pointer,
    },
};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1, zwlr_layer_surface_v1,
};

// ── Constants ──────────────────────────────────────────────

const DOCK_HEIGHT: u32 = 72;
const ICON_SIZE: u32 = 48;
const ICON_PADDING: u32 = 12;
const BG_COLOR: u32 = 0xD0_0D0F14; // Obsidian Dark, 80% opacity
const ICON_BG: u32 = 0xFF_1A1D23;  // Obsidian Surface
const ICON_ACTIVE: u32 = 0xFF_8B5CF6; // Purple accent
const TEXT_COLOR: u32 = 0xFF_A1A1AA; // Zinc 400

/// Dock app entry
struct DockApp {
    name: &'static str,
    icon_char: char,  // Single-char icon (bitmap font)
    command: &'static str,
    active: bool,
}

const APPS: &[DockApp] = &[
    DockApp { name: "Terminal", icon_char: '>', command: "trx-term", active: false },
    DockApp { name: "Files",    icon_char: 'F', command: "busybox ls", active: false },
    DockApp { name: "Editor",   icon_char: 'E', command: "busybox vi", active: false },
    DockApp { name: "Monitor",  icon_char: 'M', command: "htop", active: false },
    DockApp { name: "Sentinel", icon_char: 'S', command: "sentinel-cli monitor", active: false },
    DockApp { name: "Settings", icon_char: '*', command: "", active: false },
];

// ── App state ──────────────────────────────────────────────

struct App {
    compositor: Option<wl_compositor::WlCompositor>,
    layer_shell: Option<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
    shm: Option<wl_shm::WlShm>,
    seat: Option<wl_seat::WlSeat>,
    surface: Option<wl_surface::WlSurface>,
    layer_surface: Option<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1>,
    buffer: Option<wl_buffer::WlBuffer>,
    shm_data: Option<ShmBuffer>,
    configured: bool,
    running: bool,
    width: u32,
    pointer_x: f64,
    hover_index: Option<usize>,
}

struct ShmBuffer {
    _pool_fd: OwnedFd,
    data: *mut u32,
    width: u32,
    height: u32,
}

impl App {
    fn new() -> Self {
        Self {
            compositor: None,
            layer_shell: None,
            shm: None,
            seat: None,
            surface: None,
            layer_surface: None,
            buffer: None,
            shm_data: None,
            configured: false,
            running: true,
            width: 1280,
            pointer_x: 0.0,
            hover_index: None,
        }
    }

    fn draw(&self) {
        let Some(ref shm) = self.shm_data else { return };
        let w = shm.width as usize;
        let h = shm.height as usize;
        let pixels = unsafe { std::slice::from_raw_parts_mut(shm.data, w * h) };

        // Clear with translucent background
        for px in pixels.iter_mut() {
            *px = BG_COLOR;
        }

        // Calculate dock layout (centered)
        let app_count = APPS.len();
        let total_width = app_count as u32 * (ICON_SIZE + ICON_PADDING) - ICON_PADDING;
        let start_x = (self.width.saturating_sub(total_width)) / 2;

        for (i, app) in APPS.iter().enumerate() {
            let x = start_x + i as u32 * (ICON_SIZE + ICON_PADDING);
            let y = (DOCK_HEIGHT - ICON_SIZE) / 2;

            // Icon background (rounded rect approximation)
            let bg = if self.hover_index == Some(i) {
                ICON_ACTIVE
            } else if app.active {
                ICON_ACTIVE
            } else {
                ICON_BG
            };

            for dy in 0..ICON_SIZE {
                for dx in 0..ICON_SIZE {
                    let px_x = (x + dx) as usize;
                    let px_y = (y + dy) as usize;
                    if px_x < w && px_y < h {
                        pixels[px_y * w + px_x] = bg;
                    }
                }
            }

            // Icon letter (centered in icon, using 8x16 bitmap)
            let char_x = x + (ICON_SIZE - 8) / 2;
            let char_y = y + (ICON_SIZE - 16) / 2;
            draw_char(pixels, w, char_x as usize, char_y as usize, app.icon_char, TEXT_COLOR);

            // Active indicator dot
            if app.active {
                let dot_x = x + ICON_SIZE / 2;
                let dot_y = DOCK_HEIGHT - 4;
                for dy in 0..3u32 {
                    for dx in 0..3u32 {
                        let px_x = (dot_x + dx - 1) as usize;
                        let px_y = (dot_y + dy) as usize;
                        if px_x < w && px_y < h {
                            pixels[px_y * w + px_x] = ICON_ACTIVE;
                        }
                    }
                }
            }
        }
    }
}

// ── Minimal bitmap font (8x16, ASCII printable) ────────────

fn draw_char(pixels: &mut [u32], stride: usize, x: usize, y: usize, ch: char, color: u32) {
    // Simplified: just draw a filled rectangle for non-space chars
    // Full bitmap font from trx-bar-rs/font.rs would be imported in production
    if ch == ' ' { return; }
    let c = ch as u8;
    // Draw an 8x16 block with the character's visual weight
    for dy in 2..14 {
        for dx in 1..7 {
            let px = x + dx;
            let py = y + dy;
            if px < stride && py < stride {
                // Simple pattern: draw vertical bars based on char
                if (c.wrapping_add(dx as u8).wrapping_add(dy as u8)) % 3 != 0 {
                    pixels[py * stride + px] = color;
                }
            }
        }
    }
}

// ── Wayland dispatch stubs ─────────────────────────────────
// Full implementations would handle registry globals, layer surface
// configure, pointer events for hover/click, etc.

impl Dispatch<wl_registry::WlRegistry, ()> for App {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global { name, interface, version } = event {
            match interface.as_str() {
                "wl_compositor" => {
                    state.compositor = Some(registry.bind::<wl_compositor::WlCompositor, _, _>(
                        name, version.min(5), qh, (),
                    ));
                }
                "wl_shm" => {
                    state.shm = Some(registry.bind::<wl_shm::WlShm, _, _>(
                        name, version.min(1), qh, (),
                    ));
                }
                "zwlr_layer_shell_v1" => {
                    state.layer_shell = Some(
                        registry.bind::<zwlr_layer_shell_v1::ZwlrLayerShellV1, _, _>(
                            name, version.min(4), qh, (),
                        ),
                    );
                }
                "wl_seat" => {
                    state.seat = Some(registry.bind::<wl_seat::WlSeat, _, _>(
                        name, version.min(8), qh, (),
                    ));
                }
                _ => {}
            }
        }
    }
}

// Minimal dispatch impls for compilation
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
impl Dispatch<wl_seat::WlSeat, ()> for App {
    fn event(_: &mut Self, _: &wl_seat::WlSeat, _: wl_seat::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}
impl Dispatch<zwlr_layer_shell_v1::ZwlrLayerShellV1, ()> for App {
    fn event(_: &mut Self, _: &zwlr_layer_shell_v1::ZwlrLayerShellV1, _: zwlr_layer_shell_v1::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}
impl Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, ()> for App {
    fn event(
        state: &mut Self,
        surface: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zwlr_layer_surface_v1::Event::Configure { serial, width, height: _ } = event {
            surface.ack_configure(serial);
            if width > 0 { state.width = width; }
            state.configured = true;
        }
    }
}

// ── Main ───────────────────────────────────────────────────

fn main() {
    eprintln!("[trx-dock] Starting TerranoxOS dock...");

    let conn = Connection::connect_to_env().expect("Failed to connect to Wayland");
    let display = conn.display();

    let mut app = App::new();
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    display.get_registry(&qh, ());
    event_queue.roundtrip(&mut app).expect("Roundtrip failed");

    if app.layer_shell.is_none() {
        eprintln!("[trx-dock] ERROR: compositor does not support wlr-layer-shell-v1");
        eprintln!("[trx-dock] labwc required — tinywl does not support layer-shell");
        return;
    }

    // Create surface + layer surface
    let compositor = app.compositor.as_ref().expect("No compositor");
    let layer_shell = app.layer_shell.as_ref().expect("No layer shell");
    let surface = compositor.create_surface(&qh, ());

    let layer_surface = layer_shell.get_layer_surface(
        &surface,
        None, // default output
        zwlr_layer_shell_v1::Layer::Bottom,
        "trx-dock".to_string(),
        &qh,
        (),
    );

    // Anchor to bottom, full width
    layer_surface.set_anchor(
        zwlr_layer_surface_v1::Anchor::Bottom
            | zwlr_layer_surface_v1::Anchor::Left
            | zwlr_layer_surface_v1::Anchor::Right,
    );
    layer_surface.set_size(0, DOCK_HEIGHT);
    layer_surface.set_exclusive_zone(DOCK_HEIGHT as i32);
    surface.commit();

    app.surface = Some(surface);
    app.layer_surface = Some(layer_surface);

    // Wait for configure
    event_queue.roundtrip(&mut app).expect("Configure roundtrip failed");

    if app.configured {
        // Create SHM buffer and draw
        let stride = app.width * 4;
        let size = stride * DOCK_HEIGHT;

        let fd = rustix::fs::memfd_create(
            c"trx-dock-shm",
            rustix::fs::MemfdFlags::CLOEXEC,
        )
        .expect("memfd_create failed");

        rustix::fs::ftruncate(&fd, size as u64).expect("ftruncate failed");

        let data = unsafe {
            rustix::mm::mmap(
                std::ptr::null_mut(),
                size as usize,
                rustix::mm::ProtFlags::READ | rustix::mm::ProtFlags::WRITE,
                rustix::mm::MapFlags::SHARED,
                fd.as_fd(),
                0,
            )
            .expect("mmap failed")
        };

        let shm = app.shm.as_ref().expect("No SHM");
        let pool = shm.create_pool(fd.as_fd(), size as i32, &qh, ());
        let buffer = pool.create_buffer(
            0,
            app.width as i32,
            DOCK_HEIGHT as i32,
            stride as i32,
            wl_shm::Format::Argb8888,
            &qh,
            (),
        );

        app.shm_data = Some(ShmBuffer {
            _pool_fd: fd,
            data: data as *mut u32,
            width: app.width,
            height: DOCK_HEIGHT,
        });
        app.buffer = Some(buffer.clone());

        app.draw();

        let surface = app.surface.as_ref().unwrap();
        surface.attach(Some(&buffer), 0, 0);
        surface.damage_buffer(0, 0, app.width as i32, DOCK_HEIGHT as i32);
        surface.commit();
    }

    // Event loop
    while app.running {
        event_queue.blocking_dispatch(&mut app).expect("Dispatch failed");
    }

    eprintln!("[trx-dock] Shutting down");
}
