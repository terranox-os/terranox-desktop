//! Software rasterizer for the compositor.
//!
//! Operates on `Framebuffer` using ARGB8888 pixel format.
//! Phase 1 provides rectangle fill, border drawing, and
//! placeholder text rendering.

use crate::components::Color;
use crate::resources::Framebuffer;

/// Fill a rectangle in the framebuffer with a solid color.
///
/// Coordinates are clipped to framebuffer bounds.
pub fn fill_rect(fb: &mut Framebuffer, x: i32, y: i32, w: u32, h: u32, color: Color) {
    let px = color.to_argb8888();
    let x0 = x.max(0) as u32;
    let y0 = y.max(0) as u32;
    let x1 = if x < 0 {
        w.saturating_sub(x.unsigned_abs())
    } else {
        (x as u32).saturating_add(w)
    }
    .min(fb.width);
    let y1 = if y < 0 {
        h.saturating_sub(y.unsigned_abs())
    } else {
        (y as u32).saturating_add(h)
    }
    .min(fb.height);
    for row in y0..y1 {
        let start = (row * fb.stride + x0) as usize;
        let end = (row * fb.stride + x1) as usize;
        for px_ref in &mut fb.pixels[start..end] {
            *px_ref = px;
        }
    }
}

/// Draw a rectangle border (outline only).
pub fn draw_border(
    fb: &mut Framebuffer,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    color: Color,
    width: u32,
) {
    // Top edge
    fill_rect(fb, x, y, w, width, color);
    // Bottom edge
    fill_rect(fb, x, y + h as i32 - width as i32, w, width, color);
    // Left edge
    fill_rect(fb, x, y, width, h, color);
    // Right edge
    fill_rect(fb, x + w as i32 - width as i32, y, width, h, color);
}

/// Render a single character glyph (placeholder: 6x12 colored rectangle).
pub fn draw_char(fb: &mut Framebuffer, x: i32, y: i32, _ch: u8, color: Color) {
    fill_rect(fb, x + 1, y + 2, 6, 12, color);
}

/// Render a text string with fixed 8px advance per character.
pub fn draw_text(fb: &mut Framebuffer, x: i32, y: i32, text: &str, color: Color, _font_size: u16) {
    let mut cx = x;
    for &ch in text.as_bytes() {
        draw_char(fb, cx, y, ch, color);
        cx += 8; // 8px advance per character
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use super::*;
    use crate::components::Color;
    use crate::resources::Framebuffer;

    #[test]
    fn fill_rect_basic() {
        let mut fb = Framebuffer::new(10, 10);
        fill_rect(&mut fb, 2, 3, 4, 2, Color::WHITE);
        // Row 3, cols 2..6 should be white
        for col in 2..6 {
            assert_eq!(fb.pixels[(3 * 10 + col) as usize], 0xFFFF_FFFF);
        }
        // Row 4, cols 2..6 should be white
        for col in 2..6 {
            assert_eq!(fb.pixels[(4 * 10 + col) as usize], 0xFFFF_FFFF);
        }
        // Row 5 should be untouched
        for col in 2..6 {
            assert_eq!(fb.pixels[(5 * 10 + col) as usize], 0);
        }
        // Column 1 should be untouched
        assert_eq!(fb.pixels[(3 * 10 + 1) as usize], 0);
    }

    #[test]
    fn fill_rect_clips_negative_x() {
        let mut fb = Framebuffer::new(10, 10);
        fill_rect(&mut fb, -2, 0, 5, 1, Color::WHITE);
        // Should fill cols 0..3 of row 0
        assert_eq!(fb.pixels[0], 0xFFFF_FFFF);
        assert_eq!(fb.pixels[1], 0xFFFF_FFFF);
        assert_eq!(fb.pixels[2], 0xFFFF_FFFF);
        assert_eq!(fb.pixels[3], 0);
    }

    #[test]
    fn fill_rect_clips_negative_y() {
        let mut fb = Framebuffer::new(10, 10);
        fill_rect(&mut fb, 0, -1, 2, 3, Color::WHITE);
        // row 0 cols 0..2 should be filled (y=-1 + h=3 means rows -1,0,1)
        assert_eq!(fb.pixels[0], 0xFFFF_FFFF);
        assert_eq!(fb.pixels[1], 0xFFFF_FFFF);
        assert_eq!(fb.pixels[10], 0xFFFF_FFFF);
        assert_eq!(fb.pixels[11], 0xFFFF_FFFF);
        // row 2 should be untouched
        assert_eq!(fb.pixels[20], 0);
    }

    #[test]
    fn fill_rect_clips_right_edge() {
        let mut fb = Framebuffer::new(4, 4);
        fill_rect(&mut fb, 2, 0, 10, 1, Color::WHITE);
        // cols 2,3 filled; beyond 4 clipped
        assert_eq!(fb.pixels[0], 0);
        assert_eq!(fb.pixels[1], 0);
        assert_eq!(fb.pixels[2], 0xFFFF_FFFF);
        assert_eq!(fb.pixels[3], 0xFFFF_FFFF);
    }

    #[test]
    fn fill_rect_entirely_offscreen() {
        let mut fb = Framebuffer::new(4, 4);
        fill_rect(&mut fb, -10, -10, 2, 2, Color::WHITE);
        assert!(fb.pixels.iter().all(|&p| p == 0));
    }

    #[test]
    fn draw_border_draws_four_edges() {
        let mut fb = Framebuffer::new(20, 20);
        let c = Color {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        };
        draw_border(&mut fb, 2, 2, 10, 10, c, 1);
        let px = c.to_argb8888();

        // Top edge: row 2, cols 2..12
        for col in 2..12 {
            assert_eq!(fb.pixels[2 * 20 + col], px, "top edge col {col}");
        }
        // Bottom edge: row 11, cols 2..12
        for col in 2..12 {
            assert_eq!(fb.pixels[11 * 20 + col], px, "bottom edge col {col}");
        }
        // Left edge: rows 2..12, col 2
        for row in 2..12 {
            assert_eq!(fb.pixels[row * 20 + 2], px, "left edge row {row}");
        }
        // Right edge: rows 2..12, col 11
        for row in 2..12 {
            assert_eq!(fb.pixels[row * 20 + 11], px, "right edge row {row}");
        }
        // Interior should be empty
        assert_eq!(fb.pixels[5 * 20 + 5], 0);
    }

    #[test]
    fn draw_text_advances_correctly() {
        let mut fb = Framebuffer::new(80, 20);
        draw_text(&mut fb, 0, 0, "AB", Color::WHITE, 16);
        // First char: fill_rect(0+1, 0+2, 6, 12) -> cols 1..7, rows 2..14
        assert_eq!(fb.pixels[2 * 80 + 1], 0xFFFF_FFFF);
        // Second char: fill_rect(8+1, 0+2, 6, 12) -> cols 9..15, rows 2..14
        assert_eq!(fb.pixels[2 * 80 + 9], 0xFFFF_FFFF);
        // Gap between chars at col 7 should be empty
        assert_eq!(fb.pixels[2 * 80 + 7], 0);
    }

    #[test]
    fn draw_text_empty_string() {
        let mut fb = Framebuffer::new(10, 10);
        draw_text(&mut fb, 0, 0, "", Color::WHITE, 16);
        assert!(fb.pixels.iter().all(|&p| p == 0));
    }
}
