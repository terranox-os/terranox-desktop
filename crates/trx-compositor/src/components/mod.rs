//! UI component types for the TerranoxOS compositor.
//!
//! Each type implements `Component` from the ECS core, enabling
//! archetype-based storage and change detection.

use crate::ecs::component::Component;

// ── Spatial ──

/// Entity position in local coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}
impl Component for Position {}

/// Entity dimensions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}
impl Component for Size {}

/// Depth ordering for compositing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZIndex(pub i32);
impl Component for ZIndex {}

/// Computed absolute transform after layout. Written by layout_system.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GlobalTransform {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
impl Component for GlobalTransform {}

// ── Appearance ──

/// ARGB color, stored in individual channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    pub const BLACK: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const TRANSPARENT: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };

    /// Pack to ARGB8888 pixel format.
    pub const fn to_argb8888(self) -> u32 {
        (self.a as u32) << 24 | (self.r as u32) << 16 | (self.g as u32) << 8 | self.b as u32
    }
}

/// Background fill color for an entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackgroundColor(pub Color);
impl Component for BackgroundColor {}

/// Border stroke color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BorderColor(pub Color);
impl Component for BorderColor {}

/// Border stroke width in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BorderWidth(pub f32);
impl Component for BorderWidth {}

/// Corner radius for rounded rectangles.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BorderRadius(pub f32);
impl Component for BorderRadius {}

/// Alpha multiplier (0.0 = fully transparent, 1.0 = fully opaque).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Opacity(pub f32);
impl Component for Opacity {}

/// Visibility toggle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Visible(pub bool);
impl Component for Visible {}

// ── Layout ──

/// Flexbox main-axis direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlexDirection {
    #[default]
    Row,
    Column,
}

/// Cross-axis alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlignItems {
    #[default]
    Start,
    End,
    Center,
    Stretch,
}

/// Main-axis content distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JustifyContent {
    #[default]
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
}

/// Edge insets (padding, margin).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Edges {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

/// Flexbox layout parameters for a container entity.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FlexboxLayout {
    pub direction: FlexDirection,
    pub align_items: AlignItems,
    pub justify_content: JustifyContent,
    pub gap: f32,
    pub padding: Edges,
    pub margin: Edges,
}
impl Component for FlexboxLayout {}

// ── Text ──

/// Fixed-capacity text content (max 256 bytes, no heap).
#[derive(Debug, Clone)]
pub struct TextContent {
    pub bytes: [u8; 256],
    pub len: usize,
}

impl TextContent {
    pub fn new(s: &str) -> Self {
        let mut bytes = [0u8; 256];
        let len = s.len().min(256);
        bytes[..len].copy_from_slice(&s.as_bytes()[..len]);
        Self { bytes, len }
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }
}
impl Component for TextContent {}

/// Font size in points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontSize(pub u16);
impl Component for FontSize {}

/// Text foreground color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextColor(pub Color);
impl Component for TextColor {}

// ── Input ──

/// Pointer interaction state for an entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InteractionState {
    #[default]
    None,
    Hovered,
    Pressed,
}

/// Tracks current interaction state. Updated by input_system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Interaction {
    pub state: InteractionState,
}
impl Component for Interaction {}

/// Marker: entity can receive keyboard focus.
#[derive(Debug, Clone, Copy)]
pub struct Focusable;
impl Component for Focusable {}

// ── Surface (kernel handles) ──

/// Opaque handle to a compositor surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceHandle(pub i64);
impl Component for SurfaceHandle {}

/// Opaque handle to a shared pixel buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferHandle(pub i64);
impl Component for BufferHandle {}

// ── Window identity ──

/// Marker component for top-level window entities.
#[derive(Debug, Clone, Copy)]
pub struct Window;
impl Component for Window {}

/// Fixed-capacity window title (max 64 bytes).
#[derive(Debug, Clone)]
pub struct WindowTitle {
    pub bytes: [u8; 64],
    pub len: usize,
}

impl WindowTitle {
    pub fn new(s: &str) -> Self {
        let mut bytes = [0u8; 64];
        let len = s.len().min(64);
        bytes[..len].copy_from_slice(&s.as_bytes()[..len]);
        Self { bytes, len }
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }
}
impl Component for WindowTitle {}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use super::*;

    #[test]
    fn color_to_argb8888_white() {
        assert_eq!(Color::WHITE.to_argb8888(), 0xFFFF_FFFF);
    }

    #[test]
    fn color_to_argb8888_black() {
        assert_eq!(Color::BLACK.to_argb8888(), 0xFF00_0000);
    }

    #[test]
    fn color_to_argb8888_transparent() {
        assert_eq!(Color::TRANSPARENT.to_argb8888(), 0x0000_0000);
    }

    #[test]
    fn color_to_argb8888_custom() {
        let c = Color {
            r: 0x12,
            g: 0x34,
            b: 0x56,
            a: 0x78,
        };
        assert_eq!(c.to_argb8888(), 0x7812_3456);
    }

    #[test]
    fn text_content_new_and_as_str() {
        let tc = TextContent::new("Hello, world!");
        assert_eq!(tc.as_str(), "Hello, world!");
        assert_eq!(tc.len, 13);
    }

    #[test]
    fn text_content_truncates_at_256() {
        let long = "A".repeat(300);
        let tc = TextContent::new(&long);
        assert_eq!(tc.len, 256);
        assert_eq!(tc.as_str().len(), 256);
    }

    #[test]
    fn text_content_empty() {
        let tc = TextContent::new("");
        assert_eq!(tc.as_str(), "");
        assert_eq!(tc.len, 0);
    }

    #[test]
    fn window_title_new() {
        let wt = WindowTitle::new("My Window");
        assert_eq!(wt.as_str(), "My Window");
        assert_eq!(wt.len, 9);
    }

    #[test]
    fn window_title_truncates_at_64() {
        let long = "B".repeat(100);
        let wt = WindowTitle::new(&long);
        assert_eq!(wt.len, 64);
    }

    #[test]
    fn default_global_transform() {
        let gt = GlobalTransform::default();
        assert_eq!(gt.x, 0.0);
        assert_eq!(gt.y, 0.0);
        assert_eq!(gt.width, 0.0);
        assert_eq!(gt.height, 0.0);
    }

    #[test]
    fn default_flex_direction() {
        assert_eq!(FlexDirection::default(), FlexDirection::Row);
    }

    #[test]
    fn default_align_items() {
        assert_eq!(AlignItems::default(), AlignItems::Start);
    }

    #[test]
    fn default_justify_content() {
        assert_eq!(JustifyContent::default(), JustifyContent::Start);
    }

    #[test]
    fn default_edges() {
        let e = Edges::default();
        assert_eq!(e.top, 0.0);
        assert_eq!(e.right, 0.0);
        assert_eq!(e.bottom, 0.0);
        assert_eq!(e.left, 0.0);
    }

    #[test]
    fn default_flexbox_layout() {
        let fl = FlexboxLayout::default();
        assert_eq!(fl.direction, FlexDirection::Row);
        assert_eq!(fl.gap, 0.0);
    }

    #[test]
    fn default_interaction_state() {
        let i = Interaction::default();
        assert_eq!(i.state, InteractionState::None);
    }

    #[test]
    fn component_sizes_are_reasonable() {
        assert!(core::mem::size_of::<Position>() <= 16);
        assert!(core::mem::size_of::<Size>() <= 16);
        assert!(core::mem::size_of::<ZIndex>() <= 8);
        assert!(core::mem::size_of::<Color>() <= 8);
        assert!(core::mem::size_of::<GlobalTransform>() <= 32);
        assert!(core::mem::size_of::<Interaction>() <= 8);
        assert!(core::mem::size_of::<Focusable>() == 0);
        assert!(core::mem::size_of::<Window>() == 0);
    }
}
