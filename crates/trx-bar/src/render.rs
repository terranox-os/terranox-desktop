// SPDX-License-Identifier: GPL-2.0-only
//! Bar rendering — matches TerranoxDesktop.jsx design spec.

use crate::font;

// ── Theme (from TerranoxDesktop.jsx) ────────────────────────
pub const BAR_W: usize = 1280;
pub const BAR_H: usize = 30;

const BG: u32         = 0xFF0D_0F14; // near-black bar
const BORDER: u32     = 0xFF1A_1E28; // subtle border
const FG: u32         = 0xFFD0_D4DC; // light text
const DIM: u32        = 0xFF60_6878; // dim text
const ACCENT: u32     = 0xFF5C_E0B8; // teal accent
const ACCENT_DIM: u32 = 0xFF2A_7A62; // active workspace bg

// ── Drawing primitives ──────────────────────────────────────

fn fill_rect(buf: &mut [u32], x: i32, y: i32, w: i32, h: i32, color: u32) {
    for dy in y..y + h {
        for dx in x..x + w {
            if dx >= 0 && dx < BAR_W as i32 && dy >= 0 && dy < BAR_H as i32 {
                buf[dy as usize * BAR_W + dx as usize] = color;
            }
        }
    }
}

fn fill_circle(buf: &mut [u32], cx: i32, cy: i32, r: i32, color: u32) {
    for dy in cy - r..=cy + r {
        for dx in cx - r..=cx + r {
            if (dx - cx) * (dx - cx) + (dy - cy) * (dy - cy) <= r * r {
                if dx >= 0 && dx < BAR_W as i32 && dy >= 0 && dy < BAR_H as i32 {
                    buf[dy as usize * BAR_W + dx as usize] = color;
                }
            }
        }
    }
}

// ── Main render ─────────────────────────────────────────────

pub fn render_bar(buf: &mut [u32], clock: &str) {
    // Vertically center the 10px-tall glyphs in the 30px bar
    let text_y = (BAR_H as i32 - font::GLYPH_H as i32) / 2;

    // Background
    buf[..BAR_W * BAR_H].fill(BG);

    // Bottom border (1px)
    for x in 0..BAR_W {
        buf[(BAR_H - 1) * BAR_W + x] = BORDER;
    }

    // ── Left: workspace indicators ──
    let workspaces = ["a", "b", "g", "d", "e"];
    let mut ws_x: i32 = 8;
    let pad = 6i32;
    let ws_w = font::GLYPH_W as i32 + pad * 2;
    let ws_h = 16i32;
    let ws_y = (BAR_H as i32 - ws_h) / 2;

    for (i, label) in workspaces.iter().enumerate() {
        if i == 0 {
            // Active workspace: accent background + bright text
            fill_rect(buf, ws_x, ws_y, ws_w, ws_h, ACCENT_DIM);
            font::draw_text(buf, BAR_W, BAR_H, ws_x + pad, text_y, ACCENT, label);
        } else {
            font::draw_text(buf, BAR_W, BAR_H, ws_x + pad, text_y, DIM, label);
        }
        ws_x += ws_w + 2;
    }

    // ── Center: active window title ──
    let title = "trx-term  ~/terranox-os";
    let title_w = font::text_width(title);
    font::draw_text(buf, BAR_W, BAR_H, BAR_W as i32 / 2 - title_w / 2, text_y, FG, title);

    // ── Right: sentinel status + uptime clock ──
    let mut rx = BAR_W as i32 - 12;

    // Clock (uptime)
    let clock_w = font::text_width(clock);
    rx -= clock_w;
    font::draw_text(buf, BAR_W, BAR_H, rx, text_y, FG, clock);

    // Separator
    rx -= 14;

    // Sentinel status indicator
    let sentinel = "SENTINEL";
    let sentinel_w = font::text_width(sentinel);
    rx -= sentinel_w;
    font::draw_text(buf, BAR_W, BAR_H, rx, text_y, DIM, sentinel);

    // Green dot = sentinel running
    rx -= 10;
    fill_circle(buf, rx, BAR_H as i32 / 2, 3, ACCENT);

    // Separator
    rx -= 14;

    // Hostname
    let host = "terranox";
    let host_w = font::text_width(host);
    rx -= host_w;
    font::draw_text(buf, BAR_W, BAR_H, rx, text_y, ACCENT, host);
}
