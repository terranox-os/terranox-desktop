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
pub use ecs::world::World;
