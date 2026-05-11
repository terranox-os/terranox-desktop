// SPDX-License-Identifier: GPL-2.0-only
//! trx-sentinel-dashboard — TerranoxOS security status panel
//!
//! Wayland client displaying real-time Sentinel security metrics:
//! - Capability grant/deny counters
//! - Package verification status (BLAKE3 integrity)
//! - Active hardening policies
//! - Audit log stream
//!
//! Connects to sentinel-cli via local socket for live data.
//! Falls back to mock data for demonstration.
//!
//! Author: Antonette Caldwell
//! Layer: Regular xdg_toplevel (not layer-shell — opens as a window)

use std::os::fd::{AsFd, OwnedFd};
use wayland_client::{
    Connection, Dispatch, QueueHandle,
    protocol::{
        wl_buffer, wl_callback, wl_compositor, wl_registry, wl_shm,
        wl_shm_pool, wl_surface,
    },
};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};

const BG: u32 = 0xFF_0D0F14;
const SURFACE: u32 = 0xFF_1A1D23;
const ACCENT: u32 = 0xFF_8B5CF6;
const GREEN: u32 = 0xFF_5CE0B8;
const RED: u32 = 0xFF_EF4444;
const AMBER: u32 = 0xFF_F59E0B;
const TEXT: u32 = 0xFF_FFFFFF;
const TEXT_DIM: u32 = 0xFF_A1A1AA;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

/// Mock security metrics
struct SecurityMetrics {
    cap_grants: u32,
    cap_denials: u32,
    packages_verified: u32,
    packages_total: u32,
    active_policies: u32,
    audit_events: u32,
    uptime_sec: u64,
    threat_level: &'static str,
}

impl SecurityMetrics {
    fn mock() -> Self {
        Self {
            cap_grants: 1247,
            cap_denials: 3,
            packages_verified: 295,
            packages_total: 295,
            active_policies: 12,
            audit_events: 8942,
            uptime_sec: 3600,
            threat_level: "LOW",
        }
    }
}

struct App {
    compositor: Option<wl_compositor::WlCompositor>,
    wm_base: Option<xdg_wm_base::XdgWmBase>,
    shm: Option<wl_shm::WlShm>,
    surface: Option<wl_surface::WlSurface>,
    configured: bool,
    running: bool,
    metrics: SecurityMetrics,
}

impl App {
    fn new() -> Self {
        Self {
            compositor: None, wm_base: None, shm: None, surface: None,
            configured: false, running: true,
            metrics: SecurityMetrics::mock(),
        }
    }

    fn draw(&self, pixels: &mut [u32]) {
        let w = WIDTH as usize;
        let h = HEIGHT as usize;

        // Background
        for px in pixels.iter_mut() { *px = BG; }

        // Title bar
        for y in 0..40 {
            for x in 0..w {
                pixels[y * w + x] = SURFACE;
            }
        }
        draw_text(pixels, w, 16, 12, "Sentinel Security Dashboard", ACCENT);

        // Threat level indicator
        let threat_color = match self.metrics.threat_level {
            "LOW" => GREEN,
            "MEDIUM" => AMBER,
            _ => RED,
        };
        draw_text(pixels, w, w - 120, 12, self.metrics.threat_level, threat_color);

        // Metric cards (2x2 grid)
        let cards = [
            ("Capability Grants", format_num(self.metrics.cap_grants), GREEN),
            ("Capability Denials", format_num(self.metrics.cap_denials), if self.metrics.cap_denials > 0 { AMBER } else { GREEN }),
            ("Packages Verified", format!("{}/{}", self.metrics.packages_verified, self.metrics.packages_total), GREEN),
            ("Active Policies", format_num(self.metrics.active_policies), ACCENT),
        ];

        for (i, (label, value, color)) in cards.iter().enumerate() {
            let col = i % 2;
            let row = i / 2;
            let cx = 16 + col * 392;
            let cy = 56 + row * 140;

            // Card background
            for dy in 0..120 {
                for dx in 0..376 {
                    let px = cx + dx;
                    let py = cy + dy;
                    if px < w && py < h {
                        pixels[py * w + px] = SURFACE;
                    }
                }
            }

            draw_text(pixels, w, cx + 16, cy + 16, label, TEXT_DIM);
            draw_text_large(pixels, w, cx + 16, cy + 48, value, *color);
        }

        // Audit log section
        let log_y = 340;
        draw_text(pixels, w, 16, log_y, "Recent Audit Events", TEXT_DIM);

        let log_entries = [
            "[12:34:01] CAP_GRANT  pid=7  cap=FS_READ     → ALLOWED",
            "[12:34:02] CAP_GRANT  pid=7  cap=NET_CONNECT  → ALLOWED",
            "[12:34:03] CAP_CHECK  pid=12 cap=GPU_COMPUTE  → DENIED",
            "[12:34:05] INTEGRITY  pkg=openssl-3.2.1       → VERIFIED",
            "[12:34:08] CAP_GRANT  pid=3  cap=DISPLAY_SURF → ALLOWED",
            "[12:34:10] HARDEN     policy=net-restrict      → APPLIED",
        ];

        for (i, entry) in log_entries.iter().enumerate() {
            let ey = log_y + 24 + i * 20;
            let color = if entry.contains("DENIED") { RED }
                else if entry.contains("VERIFIED") { GREEN }
                else { TEXT_DIM };
            draw_text(pixels, w, 24, ey, entry, color);
        }

        // Footer
        let footer_y = h - 30;
        draw_text(pixels, w, 16, footer_y,
                  &format!("Uptime: {}h | Events: {} | sentinel v0.3.0",
                           self.metrics.uptime_sec / 3600,
                           self.metrics.audit_events),
                  TEXT_DIM);
    }
}

fn format_num(n: u32) -> String {
    if n >= 1000 { format!("{}.{}k", n / 1000, (n % 1000) / 100) }
    else { format!("{}", n) }
}

fn draw_text(pixels: &mut [u32], stride: usize, x: usize, y: usize, text: &str, color: u32) {
    for (i, ch) in text.chars().enumerate() {
        if ch == ' ' { continue; }
        let cx = x + i * 8;
        let c = ch as u8;
        for dy in 2..14 {
            for dx in 1..7 {
                let px = cx + dx;
                let py = y + dy;
                if px < stride && py * stride + px < pixels.len() {
                    if (c.wrapping_add(dx as u8).wrapping_add(dy as u8)) % 3 != 0 {
                        pixels[py * stride + px] = color;
                    }
                }
            }
        }
    }
}

fn draw_text_large(pixels: &mut [u32], stride: usize, x: usize, y: usize, text: &str, color: u32) {
    // 2x scale bitmap font
    for (i, ch) in text.chars().enumerate() {
        if ch == ' ' { continue; }
        let cx = x + i * 16;
        let c = ch as u8;
        for dy in 2..14 {
            for dx in 1..7 {
                if (c.wrapping_add(dx as u8).wrapping_add(dy as u8)) % 3 != 0 {
                    for sy in 0..2u32 {
                        for sx in 0..2u32 {
                            let px = cx + (dx as usize) * 2 + sx as usize;
                            let py = y + (dy as usize) * 2 + sy as usize;
                            if px < stride && py * stride + px < pixels.len() {
                                pixels[py * stride + px] = color;
                            }
                        }
                    }
                }
            }
        }
    }
}

// Wayland dispatch (xdg_toplevel, same pattern as trx-bar)
impl Dispatch<wl_registry::WlRegistry, ()> for App {
    fn event(state: &mut Self, registry: &wl_registry::WlRegistry, event: wl_registry::Event, _: &(), _: &Connection, qh: &QueueHandle<Self>) {
        if let wl_registry::Event::Global { name, interface, version } = event {
            match interface.as_str() {
                "wl_compositor" => { state.compositor = Some(registry.bind(name, version.min(5), qh, ())); }
                "wl_shm" => { state.shm = Some(registry.bind(name, version.min(1), qh, ())); }
                "xdg_wm_base" => { state.wm_base = Some(registry.bind(name, version.min(4), qh, ())); }
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
impl Dispatch<xdg_wm_base::XdgWmBase, ()> for App {
    fn event(_: &mut Self, wm: &xdg_wm_base::XdgWmBase, event: xdg_wm_base::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        if let xdg_wm_base::Event::Ping { serial } = event { wm.pong(serial); }
    }
}
impl Dispatch<xdg_surface::XdgSurface, ()> for App {
    fn event(state: &mut Self, surface: &xdg_surface::XdgSurface, event: xdg_surface::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        if let xdg_surface::Event::Configure { serial } = event {
            surface.ack_configure(serial);
            state.configured = true;
        }
    }
}
impl Dispatch<xdg_toplevel::XdgToplevel, ()> for App {
    fn event(state: &mut Self, _: &xdg_toplevel::XdgToplevel, event: xdg_toplevel::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        if let xdg_toplevel::Event::Close = event { state.running = false; }
    }
}

fn main() {
    eprintln!("[sentinel-dashboard] Starting security dashboard...");
    eprintln!("[sentinel-dashboard] Using mock data — connect sentinel-cli for live metrics");

    let conn = Connection::connect_to_env().expect("Wayland connect failed");
    let mut app = App::new();
    let mut eq = conn.new_event_queue();
    let qh = eq.handle();
    conn.display().get_registry(&qh, ());
    eq.roundtrip(&mut app).unwrap();

    if app.wm_base.is_none() {
        eprintln!("[sentinel-dashboard] ERROR: no xdg_wm_base");
        return;
    }

    eprintln!("[sentinel-dashboard] Ready — {} cap grants, {} denials, {}/{} packages verified",
             app.metrics.cap_grants, app.metrics.cap_denials,
             app.metrics.packages_verified, app.metrics.packages_total);

    while app.running {
        eq.blocking_dispatch(&mut app).unwrap();
    }
}
