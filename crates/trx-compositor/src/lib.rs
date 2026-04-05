//! TerranoxOS ECS-based compositor and UI runtime.
//!
//! Custom Entity Component System with archetype storage,
//! change detection, and Bevy-inspired API.

#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]

extern crate alloc;

pub mod ecs;

pub mod components;
pub mod platform;
pub mod render;
pub mod resources;
pub mod systems;

// ECS re-exports
pub use ecs::entity::Entity;
pub use ecs::event::{EventReader, Events};
pub use ecs::hierarchy::{Children, Parent};
pub use ecs::plugin::App;
pub use ecs::resource::Resource;
pub use ecs::system::into_system;
pub use ecs::world::World;

// Compositor re-exports
pub use components::{
    BackgroundColor, BorderColor, BorderWidth, Color, FlexboxLayout, GlobalTransform, Interaction,
    InteractionState, Opacity, Position, Size, TextContent, Visible, Window, WindowTitle, ZIndex,
};
pub use platform::{MockPlatform, PlatformBackend};
pub use resources::{DamageRegion, Framebuffer, FrameTime, InputEvents};
pub use systems::build_compositor_systems;
