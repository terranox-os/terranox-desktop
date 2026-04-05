//! Compositor systems that run each frame.
//!
//! Each system is a `fn(&mut World)` wrapped via `into_system`.
//! Execution order: input -> layout -> damage -> render -> present.

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::components::*;
use crate::ecs::system::{into_system, System};
use crate::ecs::world::World;
use crate::render;
use crate::resources::*;

/// Layout system: compute `GlobalTransform` from `Position` + `Size`.
///
/// Phase 1 simplified -- just copies Position + Size into GlobalTransform.
/// Full Flexbox layout comes in Phase 2.
pub fn layout_system(world: &mut World) {
    let pos_id = world.components.get_id::<Position>();
    let size_id = world.components.get_id::<Size>();
    let gt_id = world.components.get_id::<GlobalTransform>();

    if let (Some(pos_id), Some(size_id), Some(gt_id)) = (pos_id, size_id, gt_id) {
        for arch in world.archetypes.iter_mut() {
            if !arch.has_component(pos_id)
                || !arch.has_component(size_id)
                || !arch.has_component(gt_id)
            {
                continue;
            }

            let pos_col = arch.column_index(pos_id).unwrap();
            let size_col = arch.column_index(size_id).unwrap();
            let gt_col = arch.column_index(gt_id).unwrap();

            for row in 0..arch.len() {
                // SAFETY: row is in bounds, column types match the registered components.
                unsafe {
                    let pos = &*(arch.columns[pos_col].get_raw(row) as *const Position);
                    let size = &*(arch.columns[size_col].get_raw(row) as *const Size);
                    let gt =
                        &mut *(arch.columns[gt_col].get_raw_mut(row) as *mut GlobalTransform);
                    gt.x = pos.x;
                    gt.y = pos.y;
                    gt.width = size.width;
                    gt.height = size.height;
                }
            }
        }
    }
}

/// Render system: draw visible entities with `BackgroundColor` + `GlobalTransform`
/// to the `Framebuffer` resource.
pub fn render_system(world: &mut World) {
    let bg_id = world.components.get_id::<BackgroundColor>();
    let gt_id = world.components.get_id::<GlobalTransform>();

    // Get framebuffer as raw pointer so we can iterate archetypes mutably
    let fb_ptr: *mut Framebuffer = match world.get_resource_mut::<Framebuffer>() {
        Some(fb) => fb as *mut Framebuffer,
        None => return,
    };

    if let (Some(bg_id), Some(gt_id)) = (bg_id, gt_id) {
        for arch in world.archetypes.iter() {
            if !arch.has_component(gt_id) || !arch.has_component(bg_id) {
                continue;
            }

            let gt_col = arch.column_index(gt_id).unwrap();
            let bg_col = arch.column_index(bg_id).unwrap();

            for row in 0..arch.len() {
                // SAFETY: row is in bounds, types match, fb_ptr is valid
                // (no aliasing -- fb is a Resource, columns hold component data).
                unsafe {
                    let gt = &*(arch.columns[gt_col].get_raw(row) as *const GlobalTransform);
                    let bg = &*(arch.columns[bg_col].get_raw(row) as *const BackgroundColor);
                    render::fill_rect(
                        &mut *fb_ptr,
                        gt.x as i32,
                        gt.y as i32,
                        gt.width as u32,
                        gt.height as u32,
                        bg.0,
                    );
                }
            }
        }
    }
}

/// Input system: update `Interaction` components based on pointer position.
pub fn input_system(world: &mut World) {
    let (px, py, pressed) = match world.get_resource::<InputEvents>() {
        Some(i) => (i.pointer_x, i.pointer_y, i.pointer_pressed),
        None => return,
    };

    let gt_id = world.components.get_id::<GlobalTransform>();
    let inter_id = world.components.get_id::<Interaction>();

    if let (Some(gt_id), Some(inter_id)) = (gt_id, inter_id) {
        for arch in world.archetypes.iter_mut() {
            if !arch.has_component(gt_id) || !arch.has_component(inter_id) {
                continue;
            }

            let gt_col = arch.column_index(gt_id).unwrap();
            let inter_col = arch.column_index(inter_id).unwrap();

            for row in 0..arch.len() {
                // SAFETY: row is in bounds, types match registered components.
                unsafe {
                    let gt = &*(arch.columns[gt_col].get_raw(row) as *const GlobalTransform);
                    let inter = &mut *(arch.columns[inter_col].get_raw_mut(row)
                        as *mut Interaction);

                    let fpx = px as f32;
                    let fpy = py as f32;
                    let hit = fpx >= gt.x
                        && fpx < gt.x + gt.width
                        && fpy >= gt.y
                        && fpy < gt.y + gt.height;

                    inter.state = if hit && pressed {
                        InteractionState::Pressed
                    } else if hit {
                        InteractionState::Hovered
                    } else {
                        InteractionState::None
                    };
                }
            }
        }
    }
}

/// Damage system: mark full redraw for Phase 1 (no incremental).
pub fn damage_system(world: &mut World) {
    if let Some(damage) = world.get_resource_mut::<DamageRegion>() {
        damage.full_redraw = true;
    }
}

/// Present system: placeholder -- in real compositor, calls trx_compositor_present.
/// Increments frame counter.
pub fn present_system(world: &mut World) {
    if let Some(ft) = world.get_resource_mut::<FrameTime>() {
        ft.frame_count += 1;
    }
}

/// Create the standard compositor schedule with all 5 systems.
pub fn build_compositor_systems() -> Vec<Box<dyn System>> {
    alloc::vec![
        into_system("input", input_system),
        into_system("layout", layout_system),
        into_system("damage", damage_system),
        into_system("render", render_system),
        into_system("present", present_system),
    ]
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use super::*;
    use crate::ecs::world::World;

    /// Helper: set up a world with all required component registrations.
    fn setup_world() -> World {
        let mut world = World::new();
        world.register_component::<Position>();
        world.register_component::<Size>();
        world.register_component::<GlobalTransform>();
        world.register_component::<BackgroundColor>();
        world.register_component::<Interaction>();
        world.register_component::<Visible>();
        world
    }

    #[test]
    fn layout_system_copies_position_and_size() {
        let mut world = setup_world();

        let e = world.spawn(Position { x: 10.0, y: 20.0 });
        world.insert(e, Size { width: 100.0, height: 50.0 });
        world.insert(e, GlobalTransform::default());

        layout_system(&mut world);

        let gt = world.get::<GlobalTransform>(e).unwrap();
        assert_eq!(gt.x, 10.0);
        assert_eq!(gt.y, 20.0);
        assert_eq!(gt.width, 100.0);
        assert_eq!(gt.height, 50.0);
    }

    #[test]
    fn layout_system_updates_multiple_entities() {
        let mut world = setup_world();

        let e1 = world.spawn(Position { x: 0.0, y: 0.0 });
        world.insert(e1, Size { width: 10.0, height: 10.0 });
        world.insert(e1, GlobalTransform::default());

        let e2 = world.spawn(Position { x: 50.0, y: 60.0 });
        world.insert(e2, Size { width: 200.0, height: 100.0 });
        world.insert(e2, GlobalTransform::default());

        layout_system(&mut world);

        let gt1 = world.get::<GlobalTransform>(e1).unwrap();
        assert_eq!(gt1.x, 0.0);
        assert_eq!(gt1.width, 10.0);

        let gt2 = world.get::<GlobalTransform>(e2).unwrap();
        assert_eq!(gt2.x, 50.0);
        assert_eq!(gt2.height, 100.0);
    }

    #[test]
    fn layout_system_ignores_entities_without_position() {
        let mut world = setup_world();

        // Entity with only Size + GlobalTransform (no Position)
        let e = world.spawn(Size { width: 10.0, height: 10.0 });
        world.insert(e, GlobalTransform::default());

        layout_system(&mut world); // should not panic
    }

    #[test]
    fn render_system_fills_framebuffer() {
        let mut world = setup_world();
        world.insert_resource(Framebuffer::new(20, 20));

        let e = world.spawn(GlobalTransform {
            x: 2.0,
            y: 3.0,
            width: 4.0,
            height: 2.0,
        });
        world.insert(e, BackgroundColor(Color::WHITE));

        render_system(&mut world);

        let fb = world.get_resource::<Framebuffer>().unwrap();
        // Row 3, col 2 should be white
        assert_eq!(fb.pixels[3 * 20 + 2], 0xFFFF_FFFF);
        // Row 3, col 5 should be white
        assert_eq!(fb.pixels[3 * 20 + 5], 0xFFFF_FFFF);
        // Row 4, col 3 should be white
        assert_eq!(fb.pixels[4 * 20 + 3], 0xFFFF_FFFF);
        // Row 5 should be untouched
        assert_eq!(fb.pixels[5 * 20 + 3], 0);
    }

    #[test]
    fn render_system_no_framebuffer_is_noop() {
        let mut world = setup_world();
        // No framebuffer inserted
        let e = world.spawn(GlobalTransform {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        });
        world.insert(e, BackgroundColor(Color::WHITE));

        render_system(&mut world); // should not panic
    }

    #[test]
    fn input_system_detects_hover() {
        let mut world = setup_world();

        let mut input = InputEvents::new();
        input.pointer_x = 15;
        input.pointer_y = 25;
        input.pointer_pressed = false;
        world.insert_resource(input);

        let e = world.spawn(GlobalTransform {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        });
        world.insert(e, Interaction::default());

        input_system(&mut world);

        let inter = world.get::<Interaction>(e).unwrap();
        assert_eq!(inter.state, InteractionState::Hovered);
    }

    #[test]
    fn input_system_detects_pressed() {
        let mut world = setup_world();

        let mut input = InputEvents::new();
        input.pointer_x = 15;
        input.pointer_y = 25;
        input.pointer_pressed = true;
        world.insert_resource(input);

        let e = world.spawn(GlobalTransform {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        });
        world.insert(e, Interaction::default());

        input_system(&mut world);

        let inter = world.get::<Interaction>(e).unwrap();
        assert_eq!(inter.state, InteractionState::Pressed);
    }

    #[test]
    fn input_system_outside_is_none() {
        let mut world = setup_world();

        let mut input = InputEvents::new();
        input.pointer_x = 500;
        input.pointer_y = 500;
        input.pointer_pressed = false;
        world.insert_resource(input);

        let e = world.spawn(GlobalTransform {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        });
        world.insert(e, Interaction::default());

        input_system(&mut world);

        let inter = world.get::<Interaction>(e).unwrap();
        assert_eq!(inter.state, InteractionState::None);
    }

    #[test]
    fn input_system_no_resource_is_noop() {
        let mut world = setup_world();
        // No InputEvents inserted
        let e = world.spawn(GlobalTransform {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        });
        world.insert(e, Interaction::default());

        input_system(&mut world); // should not panic
    }

    #[test]
    fn damage_system_sets_full_redraw() {
        let mut world = World::new();
        let mut dr = DamageRegion::new();
        dr.clear();
        assert!(!dr.full_redraw);
        world.insert_resource(dr);

        damage_system(&mut world);

        assert!(world.get_resource::<DamageRegion>().unwrap().full_redraw);
    }

    #[test]
    fn present_system_increments_frame_count() {
        let mut world = World::new();
        world.insert_resource(FrameTime::new());

        present_system(&mut world);
        assert_eq!(world.get_resource::<FrameTime>().unwrap().frame_count, 1);

        present_system(&mut world);
        assert_eq!(world.get_resource::<FrameTime>().unwrap().frame_count, 2);
    }

    #[test]
    fn present_system_no_resource_is_noop() {
        let mut world = World::new();
        present_system(&mut world); // should not panic
    }

    #[test]
    fn build_compositor_systems_returns_5() {
        let systems = build_compositor_systems();
        assert_eq!(systems.len(), 5);
    }

    #[test]
    fn build_compositor_systems_correct_order() {
        let systems = build_compositor_systems();
        let names: alloc::vec::Vec<&str> = systems.iter().map(|s| s.name()).collect();
        assert_eq!(names, alloc::vec!["input", "layout", "damage", "render", "present"]);
    }

    #[test]
    fn full_frame_integration() {
        let mut world = setup_world();
        world.insert_resource(Framebuffer::new(100, 100));
        world.insert_resource(InputEvents::new());
        world.insert_resource(DamageRegion::new());
        world.insert_resource(FrameTime::new());

        // Spawn a widget
        let e = world.spawn(Position { x: 5.0, y: 5.0 });
        world.insert(e, Size { width: 20.0, height: 10.0 });
        world.insert(e, GlobalTransform::default());
        world.insert(e, BackgroundColor(Color::WHITE));
        world.insert(e, Interaction::default());

        // Run all systems in order
        let mut systems = build_compositor_systems();
        for sys in &mut systems {
            sys.run(&mut world);
        }

        // Verify layout wrote GlobalTransform
        let gt = world.get::<GlobalTransform>(e).unwrap();
        assert_eq!(gt.x, 5.0);
        assert_eq!(gt.width, 20.0);

        // Verify render wrote pixels
        let fb = world.get_resource::<Framebuffer>().unwrap();
        assert_eq!(fb.pixels[5 * 100 + 5], 0xFFFF_FFFF);

        // Verify present incremented frame count
        assert_eq!(world.get_resource::<FrameTime>().unwrap().frame_count, 1);

        // Verify damage marked full redraw
        assert!(world.get_resource::<DamageRegion>().unwrap().full_redraw);
    }
}
