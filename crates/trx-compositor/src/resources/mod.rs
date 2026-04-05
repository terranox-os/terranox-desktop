//! Singleton resources for the compositor runtime.

extern crate alloc;

use crate::components::Color;
use crate::ecs::resource::Resource;

/// Pixel buffer for software rendering (ARGB8888).
pub struct Framebuffer {
    pub pixels: alloc::vec::Vec<u32>,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
}

impl Framebuffer {
    pub fn new(width: u32, height: u32) -> Self {
        let pixels = alloc::vec![0u32; (width * height) as usize];
        Self {
            pixels,
            width,
            height,
            stride: width,
        }
    }

    /// Fill the entire framebuffer with a solid color.
    pub fn clear(&mut self, color: Color) {
        let px = color.to_argb8888();
        for p in &mut self.pixels {
            *p = px;
        }
    }
}
impl Resource for Framebuffer {}

/// Input events from the platform layer.
pub struct InputEvents {
    pub pointer_x: i32,
    pub pointer_y: i32,
    pub pointer_pressed: bool,
    pub events_this_frame: u32,
}

impl InputEvents {
    pub fn new() -> Self {
        Self {
            pointer_x: 0,
            pointer_y: 0,
            pointer_pressed: false,
            events_this_frame: 0,
        }
    }
}

impl Default for InputEvents {
    fn default() -> Self {
        Self::new()
    }
}
impl Resource for InputEvents {}

/// Frame timing information.
pub struct FrameTime {
    pub frame_count: u64,
    pub tick: u32,
}

impl FrameTime {
    pub fn new() -> Self {
        Self {
            frame_count: 0,
            tick: 0,
        }
    }
}

impl Default for FrameTime {
    fn default() -> Self {
        Self::new()
    }
}
impl Resource for FrameTime {}

/// Damage region accumulator for incremental redraw.
pub struct DamageRegion {
    pub rects: [(u32, u32, u32, u32); 16], // (x, y, w, h)
    pub count: usize,
    pub full_redraw: bool,
}

impl DamageRegion {
    pub fn new() -> Self {
        Self {
            rects: [(0, 0, 0, 0); 16],
            count: 0,
            full_redraw: true,
        }
    }

    /// Mark a rectangular region as damaged.
    pub fn mark(&mut self, x: u32, y: u32, w: u32, h: u32) {
        if self.count < 16 {
            self.rects[self.count] = (x, y, w, h);
            self.count += 1;
        } else {
            self.full_redraw = true;
        }
    }

    /// Reset damage state for the next frame.
    pub fn clear(&mut self) {
        self.count = 0;
        self.full_redraw = false;
    }
}

impl Default for DamageRegion {
    fn default() -> Self {
        Self::new()
    }
}
impl Resource for DamageRegion {}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use super::*;
    use crate::components::Color;

    #[test]
    fn framebuffer_new_zeroed() {
        let fb = Framebuffer::new(4, 4);
        assert_eq!(fb.width, 4);
        assert_eq!(fb.height, 4);
        assert_eq!(fb.stride, 4);
        assert_eq!(fb.pixels.len(), 16);
        assert!(fb.pixels.iter().all(|&p| p == 0));
    }

    #[test]
    fn framebuffer_clear_fills_all() {
        let mut fb = Framebuffer::new(8, 8);
        fb.clear(Color::WHITE);
        assert!(fb.pixels.iter().all(|&p| p == 0xFFFF_FFFF));
    }

    #[test]
    fn framebuffer_clear_with_color() {
        let mut fb = Framebuffer::new(2, 2);
        let c = Color {
            r: 0xAA,
            g: 0xBB,
            b: 0xCC,
            a: 0xDD,
        };
        fb.clear(c);
        let expected = c.to_argb8888();
        assert!(fb.pixels.iter().all(|&p| p == expected));
    }

    #[test]
    fn damage_region_mark_and_read() {
        let mut dr = DamageRegion::new();
        dr.clear(); // start clean
        assert_eq!(dr.count, 0);
        assert!(!dr.full_redraw);

        dr.mark(10, 20, 30, 40);
        assert_eq!(dr.count, 1);
        assert_eq!(dr.rects[0], (10, 20, 30, 40));
    }

    #[test]
    fn damage_region_overflow_triggers_full_redraw() {
        let mut dr = DamageRegion::new();
        dr.clear();
        for i in 0..16 {
            dr.mark(i, i, 1, 1);
        }
        assert_eq!(dr.count, 16);
        assert!(!dr.full_redraw);

        // 17th mark overflows
        dr.mark(99, 99, 1, 1);
        assert!(dr.full_redraw);
        assert_eq!(dr.count, 16); // count stays at 16
    }

    #[test]
    fn damage_region_clear_resets() {
        let mut dr = DamageRegion::new();
        dr.mark(1, 2, 3, 4);
        dr.full_redraw = true;
        dr.clear();
        assert_eq!(dr.count, 0);
        assert!(!dr.full_redraw);
    }

    #[test]
    fn input_events_default() {
        let ie = InputEvents::new();
        assert_eq!(ie.pointer_x, 0);
        assert_eq!(ie.pointer_y, 0);
        assert!(!ie.pointer_pressed);
        assert_eq!(ie.events_this_frame, 0);
    }

    #[test]
    fn frame_time_default() {
        let ft = FrameTime::new();
        assert_eq!(ft.frame_count, 0);
        assert_eq!(ft.tick, 0);
    }
}
