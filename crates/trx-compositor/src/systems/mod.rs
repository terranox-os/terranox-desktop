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

use crate::ecs::entity::Entity;
use crate::ecs::hierarchy::Children;

/// Temporary node used during layout computation.
struct LayoutNode {
    entity: Entity,
    position: Option<(f32, f32)>,
    size: Option<(f32, f32)>,
    flex: Option<FlexboxLayout>,
    child_indices: Vec<usize>, // indices into the layout_nodes array
    has_gt: bool,
    // Computed output:
    computed_x: f32,
    computed_y: f32,
    computed_w: f32,
    computed_h: f32,
}

/// Layout system: compute `GlobalTransform` from `Position` + `Size`, with
/// full Flexbox support for entities that have a `FlexboxLayout` component.
///
/// Entities with `FlexboxLayout` + `Children` act as flex containers and
/// distribute their children along the main axis according to direction,
/// justify_content, align_items, gap, and padding.
///
/// Entities without `FlexboxLayout` but with `Position` + `Size` fall back to
/// a direct copy into `GlobalTransform` (backward compatible).
pub fn layout_system(world: &mut World) {
    // ------------------------------------------------------------------
    // Phase 1: Collect all entities that participate in layout.
    // ------------------------------------------------------------------
    // We need to snapshot component data so we can read and write without
    // violating the borrow checker.

    // Collect every entity that has at least Position or Size or
    // FlexboxLayout, along with their optional components.
    let mut layout_nodes: Vec<LayoutNode> = Vec::new();
    // Map from entity id -> index in layout_nodes for parent/child linking.
    let mut entity_to_index: Vec<Option<usize>> = Vec::new();

    // Collect all alive entities from archetypes.
    let mut all_entities: Vec<Entity> = Vec::new();
    for arch in world.archetypes.iter() {
        for &e in &arch.entities {
            all_entities.push(e);
        }
    }

    for &entity in &all_entities {
        if !world.is_alive(entity) {
            continue;
        }
        let eid = entity.id as usize;

        let position = world.get::<Position>(entity).map(|p| (p.x, p.y));
        let size = world.get::<Size>(entity).map(|s| (s.width, s.height));
        let flex = world.get::<FlexboxLayout>(entity).copied();
        let has_gt = world.get::<GlobalTransform>(entity).is_some();

        // Only include entities that participate in layout.
        if position.is_none() && size.is_none() && flex.is_none() && !has_gt {
            continue;
        }

        let idx = layout_nodes.len();
        while entity_to_index.len() <= eid {
            entity_to_index.push(None);
        }
        entity_to_index[eid] = Some(idx);

        layout_nodes.push(LayoutNode {
            entity,
            position,
            size,
            flex,
            child_indices: Vec::new(),
            has_gt,
            computed_x: 0.0,
            computed_y: 0.0,
            computed_w: 0.0,
            computed_h: 0.0,
        });
    }

    // ------------------------------------------------------------------
    // Phase 1b: Link parent→child indices by reading Children components.
    // ------------------------------------------------------------------
    for node in &mut layout_nodes {
        if let Some(children) = world.get::<Children>(node.entity) {
            let child_entities: Vec<Entity> = children.entities.clone();
            let mut child_indices = Vec::new();
            for child_e in &child_entities {
                if (child_e.id as usize) < entity_to_index.len() {
                    if let Some(ci) = entity_to_index[child_e.id as usize] {
                        child_indices.push(ci);
                    }
                }
            }
            node.child_indices = child_indices;
        }
    }

    // ------------------------------------------------------------------
    // Phase 2: Compute layout.
    // ------------------------------------------------------------------
    // Track which nodes have been laid out by a parent flex container so
    // we don't overwrite their positions with the fallback copy.
    let node_count = layout_nodes.len();
    let mut laid_out_by_parent = alloc::vec![false; node_count];

    // Process flex containers.
    for i in 0..node_count {
        let flex = match layout_nodes[i].flex {
            Some(f) => f,
            None => continue,
        };

        if layout_nodes[i].child_indices.is_empty() {
            continue;
        }

        // Parent size (default to 0 if missing).
        let (parent_w, parent_h) = layout_nodes[i].size.unwrap_or((0.0, 0.0));
        let (parent_x, parent_y) = layout_nodes[i].position.unwrap_or((0.0, 0.0));

        let pad = &flex.padding;
        let content_x = parent_x + pad.left;
        let content_y = parent_y + pad.top;
        let content_w = (parent_w - pad.left - pad.right).max(0.0);
        let content_h = (parent_h - pad.top - pad.bottom).max(0.0);

        let child_indices: Vec<usize> = layout_nodes[i].child_indices.clone();
        let n_children = child_indices.len();
        if n_children == 0 {
            continue;
        }

        // Gather child main-axis and cross-axis sizes.
        let is_row = flex.direction == FlexDirection::Row;

        let mut child_main_sizes: Vec<f32> = Vec::with_capacity(n_children);
        let mut child_cross_sizes: Vec<f32> = Vec::with_capacity(n_children);

        for &ci in &child_indices {
            let (cw, ch) = layout_nodes[ci].size.unwrap_or((0.0, 0.0));
            if is_row {
                child_main_sizes.push(cw);
                child_cross_sizes.push(ch);
            } else {
                child_main_sizes.push(ch);
                child_cross_sizes.push(cw);
            }
        }

        let total_main_axis = if is_row { content_w } else { content_h };
        let total_cross_axis = if is_row { content_h } else { content_w };

        let total_children_main: f32 = child_main_sizes.iter().sum();
        let total_gap = if n_children > 1 {
            flex.gap * (n_children as f32 - 1.0)
        } else {
            0.0
        };
        let remaining = (total_main_axis - total_children_main - total_gap).max(0.0);

        // Compute main-axis starting offset and spacing based on justify_content.
        let (mut main_cursor, extra_between) = match flex.justify_content {
            JustifyContent::Start => (0.0, 0.0),
            JustifyContent::End => (remaining, 0.0),
            JustifyContent::Center => (remaining / 2.0, 0.0),
            JustifyContent::SpaceBetween => {
                if n_children > 1 {
                    (0.0, remaining / (n_children as f32 - 1.0))
                } else {
                    (0.0, 0.0)
                }
            }
            JustifyContent::SpaceAround => {
                let space = remaining / n_children as f32;
                (space / 2.0, space)
            }
        };

        for (j, &ci) in child_indices.iter().enumerate() {
            let child_main = child_main_sizes[j];
            let child_cross = child_cross_sizes[j];

            // Cross-axis position.
            let cross_pos = match flex.align_items {
                AlignItems::Start => 0.0,
                AlignItems::End => (total_cross_axis - child_cross).max(0.0),
                AlignItems::Center => ((total_cross_axis - child_cross) / 2.0).max(0.0),
                AlignItems::Stretch => 0.0,
            };

            // Cross-axis size override for Stretch.
            let effective_cross = if flex.align_items == AlignItems::Stretch {
                total_cross_axis
            } else {
                child_cross
            };

            if is_row {
                layout_nodes[ci].computed_x = content_x + main_cursor;
                layout_nodes[ci].computed_y = content_y + cross_pos;
                layout_nodes[ci].computed_w = child_main;
                layout_nodes[ci].computed_h = effective_cross;
            } else {
                layout_nodes[ci].computed_x = content_x + cross_pos;
                layout_nodes[ci].computed_y = content_y + main_cursor;
                layout_nodes[ci].computed_w = effective_cross;
                layout_nodes[ci].computed_h = child_main;
            }

            laid_out_by_parent[ci] = true;

            main_cursor += child_main;
            if j < n_children - 1 {
                main_cursor += flex.gap + extra_between;
            }
        }

        // Also lay out the parent container itself.
        layout_nodes[i].computed_x = parent_x;
        layout_nodes[i].computed_y = parent_y;
        layout_nodes[i].computed_w = parent_w;
        layout_nodes[i].computed_h = parent_h;
        laid_out_by_parent[i] = true;
    }

    // Fallback: entities that were NOT laid out by a flex parent but have
    // Position + Size get a direct copy (backward compatible behavior).
    for i in 0..node_count {
        if laid_out_by_parent[i] {
            continue;
        }
        if let (Some((px, py)), Some((sw, sh))) = (layout_nodes[i].position, layout_nodes[i].size)
        {
            layout_nodes[i].computed_x = px;
            layout_nodes[i].computed_y = py;
            layout_nodes[i].computed_w = sw;
            layout_nodes[i].computed_h = sh;
        }
    }

    // ------------------------------------------------------------------
    // Phase 3: Write computed transforms back to the World.
    // ------------------------------------------------------------------
    for i in 0..node_count {
        let node = &layout_nodes[i];
        if !node.has_gt {
            continue;
        }
        // Only write if the entity was actually computed (has position+size
        // or was laid out by a parent).
        let was_computed = laid_out_by_parent[i]
            || (node.position.is_some() && node.size.is_some());
        if !was_computed {
            continue;
        }
        if let Some(gt) = world.get_mut::<GlobalTransform>(node.entity) {
            gt.x = node.computed_x;
            gt.y = node.computed_y;
            gt.width = node.computed_w;
            gt.height = node.computed_h;
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

    // ── Flexbox layout tests ──

    use crate::ecs::hierarchy::{Children, Parent};

    /// Helper: set up a world with all layout-related component registrations
    /// including FlexboxLayout, Parent, and Children.
    fn setup_flex_world() -> World {
        let mut world = setup_world();
        world.register_component::<FlexboxLayout>();
        world.register_component::<Parent>();
        world.register_component::<Children>();
        world
    }

    #[test]
    fn flexbox_row_distributes_children_horizontally() {
        let mut world = setup_flex_world();

        // Parent: 400x100, FlexboxLayout Row, gap=10
        let parent = world.spawn(Position { x: 0.0, y: 0.0 });
        world.insert(parent, Size { width: 400.0, height: 100.0 });
        world.insert(parent, GlobalTransform::default());
        world.insert(
            parent,
            FlexboxLayout {
                direction: FlexDirection::Row,
                gap: 10.0,
                ..FlexboxLayout::default()
            },
        );

        // 3 children, each 100x50
        let c0 = world.spawn(Position { x: 0.0, y: 0.0 });
        world.insert(c0, Size { width: 100.0, height: 50.0 });
        world.insert(c0, GlobalTransform::default());
        world.insert(c0, Parent(parent));

        let c1 = world.spawn(Position { x: 0.0, y: 0.0 });
        world.insert(c1, Size { width: 100.0, height: 50.0 });
        world.insert(c1, GlobalTransform::default());
        world.insert(c1, Parent(parent));

        let c2 = world.spawn(Position { x: 0.0, y: 0.0 });
        world.insert(c2, Size { width: 100.0, height: 50.0 });
        world.insert(c2, GlobalTransform::default());
        world.insert(c2, Parent(parent));

        let mut ch = Children::new();
        ch.add(c0);
        ch.add(c1);
        ch.add(c2);
        world.insert(parent, ch);

        layout_system(&mut world);

        let gt0 = world.get::<GlobalTransform>(c0).unwrap();
        assert_eq!(gt0.x, 0.0);
        assert_eq!(gt0.y, 0.0);
        assert_eq!(gt0.width, 100.0);

        let gt1 = world.get::<GlobalTransform>(c1).unwrap();
        assert_eq!(gt1.x, 110.0); // 100 + 10 gap

        let gt2 = world.get::<GlobalTransform>(c2).unwrap();
        assert_eq!(gt2.x, 220.0); // 100 + 10 + 100 + 10
    }

    #[test]
    fn flexbox_column_distributes_children_vertically() {
        let mut world = setup_flex_world();

        let parent = world.spawn(Position { x: 0.0, y: 0.0 });
        world.insert(parent, Size { width: 100.0, height: 400.0 });
        world.insert(parent, GlobalTransform::default());
        world.insert(
            parent,
            FlexboxLayout {
                direction: FlexDirection::Column,
                gap: 10.0,
                ..FlexboxLayout::default()
            },
        );

        let c0 = world.spawn(Position { x: 0.0, y: 0.0 });
        world.insert(c0, Size { width: 50.0, height: 100.0 });
        world.insert(c0, GlobalTransform::default());
        world.insert(c0, Parent(parent));

        let c1 = world.spawn(Position { x: 0.0, y: 0.0 });
        world.insert(c1, Size { width: 50.0, height: 100.0 });
        world.insert(c1, GlobalTransform::default());
        world.insert(c1, Parent(parent));

        let c2 = world.spawn(Position { x: 0.0, y: 0.0 });
        world.insert(c2, Size { width: 50.0, height: 100.0 });
        world.insert(c2, GlobalTransform::default());
        world.insert(c2, Parent(parent));

        let mut ch = Children::new();
        ch.add(c0);
        ch.add(c1);
        ch.add(c2);
        world.insert(parent, ch);

        layout_system(&mut world);

        let gt0 = world.get::<GlobalTransform>(c0).unwrap();
        assert_eq!(gt0.y, 0.0);
        assert_eq!(gt0.x, 0.0);

        let gt1 = world.get::<GlobalTransform>(c1).unwrap();
        assert_eq!(gt1.y, 110.0);

        let gt2 = world.get::<GlobalTransform>(c2).unwrap();
        assert_eq!(gt2.y, 220.0);
    }

    #[test]
    fn flexbox_justify_content_space_between() {
        let mut world = setup_flex_world();

        // Parent 400 wide, 3 children 80 wide each.
        // Total children = 240, remaining = 160, gaps = 160/2 = 80 each.
        let parent = world.spawn(Position { x: 0.0, y: 0.0 });
        world.insert(parent, Size { width: 400.0, height: 100.0 });
        world.insert(parent, GlobalTransform::default());
        world.insert(
            parent,
            FlexboxLayout {
                direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                ..FlexboxLayout::default()
            },
        );

        let c0 = world.spawn(Position { x: 0.0, y: 0.0 });
        world.insert(c0, Size { width: 80.0, height: 50.0 });
        world.insert(c0, GlobalTransform::default());
        world.insert(c0, Parent(parent));

        let c1 = world.spawn(Position { x: 0.0, y: 0.0 });
        world.insert(c1, Size { width: 80.0, height: 50.0 });
        world.insert(c1, GlobalTransform::default());
        world.insert(c1, Parent(parent));

        let c2 = world.spawn(Position { x: 0.0, y: 0.0 });
        world.insert(c2, Size { width: 80.0, height: 50.0 });
        world.insert(c2, GlobalTransform::default());
        world.insert(c2, Parent(parent));

        let mut ch = Children::new();
        ch.add(c0);
        ch.add(c1);
        ch.add(c2);
        world.insert(parent, ch);

        layout_system(&mut world);

        let gt0 = world.get::<GlobalTransform>(c0).unwrap();
        assert_eq!(gt0.x, 0.0);

        let gt1 = world.get::<GlobalTransform>(c1).unwrap();
        assert_eq!(gt1.x, 160.0); // 80 + 80 space

        let gt2 = world.get::<GlobalTransform>(c2).unwrap();
        assert_eq!(gt2.x, 320.0); // 80 + 80 + 80 + 80
    }

    #[test]
    fn flexbox_align_items_center_cross_axis() {
        let mut world = setup_flex_world();

        // Row parent 400x200, children 100x50 -> y = (200-50)/2 = 75
        let parent = world.spawn(Position { x: 0.0, y: 0.0 });
        world.insert(parent, Size { width: 400.0, height: 200.0 });
        world.insert(parent, GlobalTransform::default());
        world.insert(
            parent,
            FlexboxLayout {
                direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                ..FlexboxLayout::default()
            },
        );

        let c0 = world.spawn(Position { x: 0.0, y: 0.0 });
        world.insert(c0, Size { width: 100.0, height: 50.0 });
        world.insert(c0, GlobalTransform::default());
        world.insert(c0, Parent(parent));

        let mut ch = Children::new();
        ch.add(c0);
        world.insert(parent, ch);

        layout_system(&mut world);

        let gt0 = world.get::<GlobalTransform>(c0).unwrap();
        assert_eq!(gt0.y, 75.0); // (200 - 50) / 2
        assert_eq!(gt0.x, 0.0);
    }

    #[test]
    fn flexbox_padding_reduces_available_space() {
        let mut world = setup_flex_world();

        // Parent 400x200 with padding 20 all sides -> content area 360x160
        // Child 100x50 -> placed at (20, 20) (content origin)
        let parent = world.spawn(Position { x: 0.0, y: 0.0 });
        world.insert(parent, Size { width: 400.0, height: 200.0 });
        world.insert(parent, GlobalTransform::default());
        world.insert(
            parent,
            FlexboxLayout {
                direction: FlexDirection::Row,
                padding: Edges {
                    top: 20.0,
                    right: 20.0,
                    bottom: 20.0,
                    left: 20.0,
                },
                ..FlexboxLayout::default()
            },
        );

        let c0 = world.spawn(Position { x: 0.0, y: 0.0 });
        world.insert(c0, Size { width: 100.0, height: 50.0 });
        world.insert(c0, GlobalTransform::default());
        world.insert(c0, Parent(parent));

        let c1 = world.spawn(Position { x: 0.0, y: 0.0 });
        world.insert(c1, Size { width: 100.0, height: 50.0 });
        world.insert(c1, GlobalTransform::default());
        world.insert(c1, Parent(parent));

        let mut ch = Children::new();
        ch.add(c0);
        ch.add(c1);
        world.insert(parent, ch);

        layout_system(&mut world);

        let gt0 = world.get::<GlobalTransform>(c0).unwrap();
        assert_eq!(gt0.x, 20.0); // padding left
        assert_eq!(gt0.y, 20.0); // padding top (AlignItems::Start)

        let gt1 = world.get::<GlobalTransform>(c1).unwrap();
        assert_eq!(gt1.x, 120.0); // 20 + 100
    }

    #[test]
    fn entities_without_flexbox_still_get_position_to_global_transform() {
        let mut world = setup_flex_world();

        // Entity with Position + Size but no FlexboxLayout
        let e = world.spawn(Position { x: 42.0, y: 17.0 });
        world.insert(e, Size { width: 300.0, height: 150.0 });
        world.insert(e, GlobalTransform::default());

        layout_system(&mut world);

        let gt = world.get::<GlobalTransform>(e).unwrap();
        assert_eq!(gt.x, 42.0);
        assert_eq!(gt.y, 17.0);
        assert_eq!(gt.width, 300.0);
        assert_eq!(gt.height, 150.0);
    }
}
