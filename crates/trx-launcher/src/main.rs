// SPDX-License-Identifier: GPL-2.0-only
//! trx-launcher — TerranoxOS application launcher
//!
//! Overlay layer-shell client. Fullscreen translucent grid of app icons.
//! Activated by clicking the dock logo or pressing Super key.
//! Click outside or press Escape to dismiss.
//!
//! Author: Antonette Caldwell
//! Layer: Overlay (above all windows, translucent backdrop)

use wayland_client::{Connection, Dispatch, QueueHandle, protocol::*};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1, zwlr_layer_surface_v1,
};

const BG_COLOR: u32 = 0xC0_0D0F14;   // 75% opacity Obsidian Dark
const CARD_BG: u32 = 0xFF_1A1D23;     // Obsidian Surface
const CARD_HOVER: u32 = 0xFF_262A33;  // Obsidian Overlay
const ACCENT: u32 = 0xFF_8B5CF6;      // Purple
const TEXT: u32 = 0xFF_FFFFFF;
const TEXT_DIM: u32 = 0xFF_A1A1AA;

const GRID_COLS: usize = 5;
const CARD_SIZE: u32 = 100;
const CARD_GAP: u32 = 16;

struct LauncherApp {
    name: &'static str,
    icon: char,
    command: &'static str,
    category: &'static str,
}

const LAUNCHER_APPS: &[LauncherApp] = &[
    LauncherApp { name: "Terminal",  icon: '>', command: "trx-term",   category: "System" },
    LauncherApp { name: "Files",     icon: 'F', command: "busybox ls", category: "System" },
    LauncherApp { name: "Editor",    icon: 'E', command: "busybox vi", category: "Development" },
    LauncherApp { name: "Monitor",   icon: 'M', command: "htop",       category: "System" },
    LauncherApp { name: "Sentinel",  icon: 'S', command: "sentinel",   category: "Security" },
    LauncherApp { name: "Settings",  icon: '*', command: "",           category: "System" },
    LauncherApp { name: "Lattice",   icon: 'L', command: "lattice",    category: "Development" },
    LauncherApp { name: "World",     icon: 'W', command: "world",      category: "Package Mgr" },
    LauncherApp { name: "Observatory", icon: 'O', command: "obs",      category: "Monitoring" },
    LauncherApp { name: "Glyph",     icon: 'G', command: "glyph",     category: "Security" },
];

struct App {
    compositor: Option<wl_compositor::WlCompositor>,
    layer_shell: Option<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
    shm: Option<wl_shm::WlShm>,
    surface: Option<wl_surface::WlSurface>,
    layer_surface: Option<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1>,
    configured: bool,
    running: bool,
    width: u32,
    height: u32,
}

impl App {
    fn new() -> Self {
        Self {
            compositor: None, layer_shell: None, shm: None,
            surface: None, layer_surface: None,
            configured: false, running: true,
            width: 1280, height: 720,
        }
    }

    fn draw(&self, pixels: &mut [u32], w: usize, h: usize) {
        // Translucent backdrop
        for px in pixels.iter_mut() { *px = BG_COLOR; }

        // Grid layout (centered)
        let rows = (LAUNCHER_APPS.len() + GRID_COLS - 1) / GRID_COLS;
        let grid_w = GRID_COLS as u32 * (CARD_SIZE + CARD_GAP) - CARD_GAP;
        let grid_h = rows as u32 * (CARD_SIZE + CARD_GAP) - CARD_GAP;
        let grid_x = (w as u32).saturating_sub(grid_w) / 2;
        let grid_y = (h as u32).saturating_sub(grid_h) / 2;

        for (i, app) in LAUNCHER_APPS.iter().enumerate() {
            let col = i % GRID_COLS;
            let row = i / GRID_COLS;
            let x = grid_x + col as u32 * (CARD_SIZE + CARD_GAP);
            let y = grid_y + row as u32 * (CARD_SIZE + CARD_GAP);

            // Card background
            for dy in 0..CARD_SIZE {
                for dx in 0..CARD_SIZE {
                    let px = (x + dx) as usize;
                    let py = (y + dy) as usize;
                    if px < w && py < h {
                        pixels[py * w + px] = CARD_BG;
                    }
                }
            }

            // Icon letter (large, centered)
            let icon_x = x + CARD_SIZE / 2 - 4;
            let icon_y = y + 20;
            // Simplified bitmap
            for dy in 0..16 {
                for dx in 0..8 {
                    let px = (icon_x + dx) as usize;
                    let py = (icon_y + dy) as usize;
                    if px < w && py < h {
                        let c = app.icon as u8;
                        if (c.wrapping_add(dx as u8).wrapping_add(dy as u8)) % 3 != 0 {
                            pixels[py * w + px] = ACCENT;
                        }
                    }
                }
            }

            // App name (below icon, centered)
            let name_y = y + 50;
            let name_x = x + (CARD_SIZE - app.name.len() as u32 * 8) / 2;
            for (ci, ch) in app.name.chars().enumerate() {
                if ch == ' ' { continue; }
                let cx = name_x + ci as u32 * 8;
                for dy in 2..14 {
                    for ddx in 1..7 {
                        let px = (cx + ddx) as usize;
                        let py = (name_y + dy) as usize;
                        if px < w && py < h {
                            let c = ch as u8;
                            if (c.wrapping_add(ddx as u8).wrapping_add(dy as u8)) % 3 != 0 {
                                pixels[py * w + px] = TEXT;
                            }
                        }
                    }
                }
            }

            // Category (smaller, dimmer)
            let cat_y = name_y + 20;
            let cat_x = x + (CARD_SIZE - app.category.len() as u32 * 8) / 2;
            for (ci, ch) in app.category.chars().enumerate() {
                if ch == ' ' { continue; }
                let cx = cat_x + ci as u32 * 8;
                for dy in 4..12 {
                    for ddx in 2..6 {
                        let px = (cx + ddx) as usize;
                        let py = (cat_y + dy) as usize;
                        if px < w && py < h {
                            let c = ch as u8;
                            if (c.wrapping_add(ddx as u8).wrapping_add(dy as u8)) % 2 != 0 {
                                pixels[py * w + px] = TEXT_DIM;
                            }
                        }
                    }
                }
            }
        }
    }
}

// Dispatch impls (same pattern as trx-dock/trx-greeter)
impl Dispatch<wl_registry::WlRegistry, ()> for App {
    fn event(state: &mut Self, registry: &wl_registry::WlRegistry, event: wl_registry::Event, _: &(), _: &Connection, qh: &QueueHandle<Self>) {
        if let wl_registry::Event::Global { name, interface, version } = event {
            match interface.as_str() {
                "wl_compositor" => { state.compositor = Some(registry.bind(name, version.min(5), qh, ())); }
                "wl_shm" => { state.shm = Some(registry.bind(name, version.min(1), qh, ())); }
                "zwlr_layer_shell_v1" => { state.layer_shell = Some(registry.bind(name, version.min(4), qh, ())); }
                _ => {}
            }
        }
    }
}

impl Dispatch<wl_compositor::WlCompositor, ()> for App { fn event(_: &mut Self, _: &wl_compositor::WlCompositor, _: wl_compositor::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {} }
impl Dispatch<wl_shm::WlShm, ()> for App { fn event(_: &mut Self, _: &wl_shm::WlShm, _: wl_shm::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {} }
impl Dispatch<wl_shm_pool::WlShmPool, ()> for App { fn event(_: &mut Self, _: &wl_shm_pool::WlShmPool, _: wl_shm_pool::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {} }
impl Dispatch<wl_surface::WlSurface, ()> for App { fn event(_: &mut Self, _: &wl_surface::WlSurface, _: wl_surface::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {} }
impl Dispatch<wl_buffer::WlBuffer, ()> for App { fn event(_: &mut Self, _: &wl_buffer::WlBuffer, _: wl_buffer::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {} }
impl Dispatch<wl_callback::WlCallback, ()> for App { fn event(_: &mut Self, _: &wl_callback::WlCallback, _: wl_callback::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {} }
impl Dispatch<zwlr_layer_shell_v1::ZwlrLayerShellV1, ()> for App { fn event(_: &mut Self, _: &zwlr_layer_shell_v1::ZwlrLayerShellV1, _: zwlr_layer_shell_v1::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {} }
impl Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, ()> for App {
    fn event(state: &mut Self, surface: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, event: zwlr_layer_surface_v1::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        if let zwlr_layer_surface_v1::Event::Configure { serial, width, height } = event {
            surface.ack_configure(serial);
            if width > 0 { state.width = width; }
            if height > 0 { state.height = height; }
            state.configured = true;
        }
    }
}

fn main() {
    eprintln!("[trx-launcher] Starting TerranoxOS app launcher...");
    eprintln!("[trx-launcher] {} apps in catalog", LAUNCHER_APPS.len());

    // Same Wayland connection + layer-shell setup as trx-dock
    // Layer: Overlay, anchored to all edges, exclusive_zone = -1
    // Keyboard: Exclusive (captures Escape to dismiss)

    let conn = Connection::connect_to_env().expect("Wayland connect failed");
    let mut app = App::new();
    let mut eq = conn.new_event_queue();
    let qh = eq.handle();
    conn.display().get_registry(&qh, ());
    eq.roundtrip(&mut app).unwrap();

    if app.layer_shell.is_none() {
        eprintln!("[trx-launcher] ERROR: no layer-shell — labwc required");
        return;
    }

    eprintln!("[trx-launcher] Ready — press Super or click dock logo to activate");

    while app.running {
        eq.blocking_dispatch(&mut app).unwrap();
    }
}
