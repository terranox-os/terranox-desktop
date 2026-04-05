//! TerranoxOS ECS-based compositor and UI runtime.
//!
//! Custom Entity Component System with archetype storage,
//! change detection, and Bevy-inspired API.

#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]

extern crate alloc;

pub mod ecs;

// Re-exports
pub use ecs::entity::Entity;
pub use ecs::event::{EventReader, Events};
pub use ecs::hierarchy::{Children, Parent};
pub use ecs::plugin::App;
pub use ecs::resource::Resource;
pub use ecs::system::into_system;
pub use ecs::world::World;
