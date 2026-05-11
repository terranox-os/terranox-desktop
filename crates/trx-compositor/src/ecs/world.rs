use alloc::vec::Vec;

use crate::ecs::archetype::{ArchetypeId, Archetypes};
use crate::ecs::component::{Component, ComponentId, ComponentRegistry};
use crate::ecs::entity::{Entity, EntityAllocator};
use crate::ecs::resource::{Resource, Resources};

/// The ECS World: owns all entities, components, and archetypes.
pub struct World {
    pub(crate) entities: EntityAllocator,
    pub(crate) components: ComponentRegistry,
    pub(crate) archetypes: Archetypes,
    /// Maps entity id to (archetype_id, row index within archetype).
    pub(crate) entity_locations: Vec<Option<(ArchetypeId, usize)>>,
    /// Singleton resource storage.
    pub(crate) resources: Resources,
    /// Current world tick (incremented each frame).
    pub(crate) current_tick: u32,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    pub fn new() -> Self {
        Self {
            entities: EntityAllocator::new(),
            components: ComponentRegistry::new(),
            archetypes: Archetypes::new(),
            entity_locations: Vec::new(),
            resources: Resources::new(),
            current_tick: 0,
        }
    }

    /// Register a component type. Must be called before using the type.
    pub fn register_component<T: Component>(&mut self) -> ComponentId {
        self.components.register::<T>()
    }

    /// Spawn a new empty entity.
    pub fn spawn_empty(&mut self) -> Entity {
        let entity = self.entities.allocate();
        // Ensure location storage is large enough
        while self.entity_locations.len() <= entity.id as usize {
            self.entity_locations.push(None);
        }
        entity
    }

    /// Spawn an entity with a single component.
    pub fn spawn<T: Component>(&mut self, component: T) -> Entity {
        let entity = self.spawn_empty();
        self.insert(entity, component);
        entity
    }

    /// Insert a component onto an entity. If the entity already has this
    /// component type, it is replaced.
    pub fn insert<T: Component>(&mut self, entity: Entity, component: T) {
        if !self.entities.is_alive(entity) {
            return;
        }

        let comp_id = self.components.register::<T>();
        let info = self.components.get_info(comp_id).unwrap();
        let size = info.size;
        let align = info.align;

        // Determine new archetype (current components + new one)
        let current_loc = self.entity_locations[entity.id as usize];

        if let Some((arch_id, row)) = current_loc {
            let arch = self.archetypes.get(arch_id).unwrap();
            if arch.has_component(comp_id) {
                // Replace existing component in-place
                let col_idx = arch.column_index(comp_id).unwrap();
                let arch_mut = self.archetypes.get_mut(arch_id).unwrap();
                // SAFETY: row is valid, component is a valid T, dst points to a valid T-sized slot
                unsafe {
                    let dst = arch_mut.columns[col_idx].get_raw_mut(row);
                    core::ptr::copy_nonoverlapping(
                        &component as *const T as *const u8,
                        dst,
                        size,
                    );
                }
                arch_mut.columns[col_idx].set_change_tick(row, self.current_tick);
                core::mem::forget(component);
                return;
            }
        }

        // Build the new component set
        let new_comp_ids: Vec<ComponentId>;
        let new_sizes: Vec<(usize, usize)>;

        if let Some((arch_id, _row)) = current_loc {
            let arch = self.archetypes.get(arch_id).unwrap();
            let mut ids = arch.component_ids.clone();
            ids.push(comp_id);
            let mut sizes_vec = Vec::new();
            for &cid in &ids {
                let ci = self.components.get_info(cid).unwrap();
                sizes_vec.push((ci.size, ci.align));
            }
            new_comp_ids = ids;
            new_sizes = sizes_vec;
        } else {
            new_comp_ids = alloc::vec![comp_id];
            new_sizes = alloc::vec![(size, align)];
        }

        // Get or create target archetype
        // Note: get_or_create sorts new_comp_ids internally, so we need the
        // sorted version to look up column indices in the new archetype.
        let target_arch_id = self.archetypes.get_or_create(new_comp_ids, &new_sizes);

        // Move entity data from old archetype to new.
        // We must save existing component data before removing from the old
        // archetype, so it can be copied into the new one.
        let mut saved_data: Vec<(ComponentId, Vec<u8>)> = Vec::new();
        if let Some((old_arch_id, old_row)) = current_loc {
            let old_arch = self.archetypes.get_mut(old_arch_id).unwrap();
            // Save existing component data before removal
            for (i, &cid) in old_arch.component_ids.clone().iter().enumerate() {
                let col = &old_arch.columns[i];
                let item_size = col.item_size;
                let mut buf = alloc::vec![0u8; item_size];
                // SAFETY: old_row is in bounds, column stores valid data
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        col.get_raw(old_row),
                        buf.as_mut_ptr(),
                        item_size,
                    );
                }
                saved_data.push((cid, buf));
            }
            // Remove from old archetype (swap-remove)
            old_arch.entities.swap_remove(old_row);
            for col in &mut old_arch.columns {
                col.swap_remove(old_row);
            }
            // If the swap moved the last entity to old_row, update its location
            if old_row < old_arch.entities.len() {
                let moved_entity = old_arch.entities[old_row];
                self.entity_locations[moved_entity.id as usize] = Some((old_arch_id, old_row));
            }
        }

        // Add to new archetype
        let new_arch = self.archetypes.get_mut(target_arch_id).unwrap();
        let new_row = new_arch.len();
        new_arch.entities.push(entity);

        // Push the new component data into the correct column
        let col_idx = new_arch.column_index(comp_id).unwrap();

        // Push zero-initialized data for all columns, then overwrite
        for col in &mut new_arch.columns {
            let zeros = alloc::vec![0u8; col.item_size];
            // SAFETY: zeros is a valid byte buffer of the correct size
            unsafe {
                col.push_raw(zeros.as_ptr(), self.current_tick);
            }
        }

        // Copy saved component data from old archetype into new archetype
        for (cid, data) in &saved_data {
            if let Some(new_col_idx) = new_arch.column_index(*cid) {
                // SAFETY: new_row is valid, data length matches column item_size
                unsafe {
                    let dst = new_arch.columns[new_col_idx].get_raw_mut(new_row);
                    core::ptr::copy_nonoverlapping(data.as_ptr(), dst, data.len());
                }
            }
        }

        // Overwrite the target column with the actual new component data
        // SAFETY: new_row is valid (we just pushed), component is a valid T
        unsafe {
            let dst = new_arch.columns[col_idx].get_raw_mut(new_row);
            core::ptr::copy_nonoverlapping(
                &component as *const T as *const u8,
                dst,
                size,
            );
        }

        core::mem::forget(component);
        self.entity_locations[entity.id as usize] = Some((target_arch_id, new_row));
    }

    /// Despawn an entity.
    pub fn despawn(&mut self, entity: Entity) -> bool {
        if !self.entities.is_alive(entity) {
            return false;
        }

        if let Some((arch_id, row)) = self.entity_locations[entity.id as usize] {
            let arch = self.archetypes.get_mut(arch_id).unwrap();
            if row < arch.entities.len() {
                arch.entities.swap_remove(row);
                for col in &mut arch.columns {
                    col.swap_remove(row);
                }
                // If the swap moved the last entity to `row`, update its location
                if row < arch.entities.len() {
                    let moved_entity = arch.entities[row];
                    self.entity_locations[moved_entity.id as usize] = Some((arch_id, row));
                }
            }
        }

        self.entity_locations[entity.id as usize] = None;
        self.entities.free(entity);
        true
    }

    /// Get a component reference for an entity.
    pub fn get<T: Component>(&self, entity: Entity) -> Option<&T> {
        if !self.entities.is_alive(entity) {
            return None;
        }
        let comp_id = self.components.get_id::<T>()?;
        let (arch_id, row) = self.entity_locations[entity.id as usize]?;
        let arch = self.archetypes.get(arch_id)?;
        let col_idx = arch.column_index(comp_id)?;
        // SAFETY: row is valid, column stores T-typed data at this index
        unsafe {
            let ptr = arch.columns[col_idx].get_raw(row);
            Some(&*(ptr as *const T))
        }
    }

    /// Get a mutable component reference for an entity.
    pub fn get_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        if !self.entities.is_alive(entity) {
            return None;
        }
        let comp_id = self.components.get_id::<T>()?;
        let (arch_id, row) = self.entity_locations[entity.id as usize]?;
        let arch = self.archetypes.get_mut(arch_id)?;
        let col_idx = arch.column_index(comp_id)?;
        arch.columns[col_idx].set_change_tick(row, self.current_tick);
        // SAFETY: row is valid, column stores T-typed data at this index
        unsafe {
            let ptr = arch.columns[col_idx].get_raw_mut(row);
            Some(&mut *(ptr as *mut T))
        }
    }

    /// Check if entity is alive.
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.entities.is_alive(entity)
    }

    /// Insert a singleton resource into the world.
    pub fn insert_resource<R: Resource>(&mut self, resource: R) {
        self.resources.insert(resource);
    }

    /// Get a shared reference to a resource.
    pub fn get_resource<R: Resource>(&self) -> Option<&R> {
        self.resources.get::<R>()
    }

    /// Get a mutable reference to a resource.
    pub fn get_resource_mut<R: Resource>(&mut self) -> Option<&mut R> {
        self.resources.get_mut::<R>()
    }

    /// Increment the world tick (call once per frame).
    pub fn increment_tick(&mut self) {
        self.current_tick += 1;
    }

    pub fn entity_count(&self) -> u32 {
        self.entities.alive_count()
    }

    pub fn current_tick(&self) -> u32 {
        self.current_tick
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use super::*;
    use crate::ecs::component::Component;

    #[derive(Debug, PartialEq, Clone, Copy)]
    struct Position {
        x: f32,
        y: f32,
    }
    impl Component for Position {}

    #[derive(Debug, PartialEq, Clone, Copy)]
    struct Velocity {
        dx: f32,
        dy: f32,
    }
    impl Component for Velocity {}

    #[derive(Debug, PartialEq, Clone, Copy)]
    struct Health {
        hp: u32,
    }
    impl Component for Health {}

    #[test]
    fn spawn_empty_creates_entity() {
        let mut world = World::new();
        let e = world.spawn_empty();
        assert!(world.is_alive(e));
        assert_eq!(world.entity_count(), 1);
    }

    #[test]
    fn spawn_with_component() {
        let mut world = World::new();
        world.register_component::<Position>();
        let e = world.spawn(Position { x: 1.0, y: 2.0 });
        assert!(world.is_alive(e));
        let pos = world.get::<Position>(e).unwrap();
        assert_eq!(pos.x, 1.0);
        assert_eq!(pos.y, 2.0);
    }

    #[test]
    fn get_returns_none_for_missing_component() {
        let mut world = World::new();
        world.register_component::<Position>();
        world.register_component::<Velocity>();
        let e = world.spawn(Position { x: 0.0, y: 0.0 });
        assert!(world.get::<Velocity>(e).is_none());
    }

    #[test]
    fn get_mut_modifies_component() {
        let mut world = World::new();
        world.register_component::<Position>();
        let e = world.spawn(Position { x: 1.0, y: 2.0 });
        {
            let pos = world.get_mut::<Position>(e).unwrap();
            pos.x = 99.0;
            pos.y = 88.0;
        }
        let pos = world.get::<Position>(e).unwrap();
        assert_eq!(pos.x, 99.0);
        assert_eq!(pos.y, 88.0);
    }

    #[test]
    fn insert_replaces_existing_component() {
        let mut world = World::new();
        world.register_component::<Position>();
        let e = world.spawn(Position { x: 1.0, y: 2.0 });
        world.insert(e, Position { x: 10.0, y: 20.0 });
        let pos = world.get::<Position>(e).unwrap();
        assert_eq!(pos.x, 10.0);
        assert_eq!(pos.y, 20.0);
    }

    #[test]
    fn despawn_removes_entity() {
        let mut world = World::new();
        world.register_component::<Position>();
        let e = world.spawn(Position { x: 1.0, y: 2.0 });
        assert!(world.despawn(e));
        assert!(!world.is_alive(e));
        assert_eq!(world.entity_count(), 0);
    }

    #[test]
    fn despawn_invalidates_get() {
        let mut world = World::new();
        world.register_component::<Position>();
        let e = world.spawn(Position { x: 1.0, y: 2.0 });
        world.despawn(e);
        assert!(world.get::<Position>(e).is_none());
    }

    #[test]
    fn despawn_returns_false_for_dead_entity() {
        let mut world = World::new();
        let e = world.spawn_empty();
        world.despawn(e);
        assert!(!world.despawn(e));
    }

    #[test]
    fn entity_count_tracking() {
        let mut world = World::new();
        assert_eq!(world.entity_count(), 0);
        let e0 = world.spawn_empty();
        let _e1 = world.spawn_empty();
        let _e2 = world.spawn_empty();
        assert_eq!(world.entity_count(), 3);
        world.despawn(e0);
        assert_eq!(world.entity_count(), 2);
    }

    #[test]
    fn tick_increment() {
        let mut world = World::new();
        assert_eq!(world.current_tick(), 0);
        world.increment_tick();
        assert_eq!(world.current_tick(), 1);
        world.increment_tick();
        world.increment_tick();
        assert_eq!(world.current_tick(), 3);
    }

    #[test]
    fn multiple_entities_same_component() {
        let mut world = World::new();
        world.register_component::<Position>();
        let e0 = world.spawn(Position { x: 1.0, y: 2.0 });
        let e1 = world.spawn(Position { x: 3.0, y: 4.0 });
        let e2 = world.spawn(Position { x: 5.0, y: 6.0 });
        assert_eq!(world.get::<Position>(e0).unwrap().x, 1.0);
        assert_eq!(world.get::<Position>(e1).unwrap().x, 3.0);
        assert_eq!(world.get::<Position>(e2).unwrap().x, 5.0);
    }

    #[test]
    fn despawn_middle_entity_preserves_others() {
        let mut world = World::new();
        world.register_component::<Health>();
        let e0 = world.spawn(Health { hp: 100 });
        let e1 = world.spawn(Health { hp: 200 });
        let e2 = world.spawn(Health { hp: 300 });
        world.despawn(e1);
        assert!(!world.is_alive(e1));
        // e0 and e2 should still be accessible
        assert_eq!(world.get::<Health>(e0).unwrap().hp, 100);
        assert_eq!(world.get::<Health>(e2).unwrap().hp, 300);
    }

    #[test]
    fn get_on_empty_entity_returns_none() {
        let mut world = World::new();
        world.register_component::<Position>();
        let e = world.spawn_empty();
        assert!(world.get::<Position>(e).is_none());
    }

    #[test]
    fn insert_on_dead_entity_is_noop() {
        let mut world = World::new();
        world.register_component::<Position>();
        let e = world.spawn_empty();
        world.despawn(e);
        world.insert(e, Position { x: 1.0, y: 2.0 });
        assert!(!world.is_alive(e));
    }

    #[test]
    fn get_mut_updates_change_tick() {
        let mut world = World::new();
        world.register_component::<Position>();
        let e = world.spawn(Position { x: 0.0, y: 0.0 });
        world.increment_tick(); // tick = 1
        world.increment_tick(); // tick = 2
        let _pos = world.get_mut::<Position>(e).unwrap();
        // The change tick for this component should be the current tick (2)
        let comp_id = world.components.get_id::<Position>().unwrap();
        let (arch_id, row) = world.entity_locations[e.id as usize].unwrap();
        let arch = world.archetypes.get(arch_id).unwrap();
        let col_idx = arch.column_index(comp_id).unwrap();
        assert_eq!(arch.columns[col_idx].get_change_tick(row), 2);
    }

    #[test]
    fn insert_components_in_reverse_id_order() {
        // Regression test: inserting components in non-ascending ComponentId
        // order previously caused column size mismatches because
        // get_or_create sorted component_ids but not sizes.
        let mut world = World::new();

        // Register in order: Position gets ID 0, then Velocity gets ID 1.
        // But a different component registered first would shift IDs.
        // Use a third type to ensure non-trivial ordering.
        #[derive(Debug, PartialEq, Clone, Copy)]
        struct LargeComponent {
            data: [u8; 64],
        }
        impl Component for LargeComponent {}

        world.register_component::<LargeComponent>(); // ID 0 (64 bytes)
        world.register_component::<Position>();         // ID 1 (8 bytes)
        world.register_component::<Velocity>();         // ID 2 (8 bytes)

        let e = world.spawn_empty();

        // Insert in REVERSE ID order: Velocity (ID 2), Position (ID 1), LargeComponent (ID 0)
        world.insert(e, Velocity { dx: 3.0, dy: 4.0 });
        world.insert(e, Position { x: 1.0, y: 2.0 });
        world.insert(e, LargeComponent { data: [42u8; 64] });

        // Verify all components readable without corruption
        let pos = world.get::<Position>(e).unwrap();
        assert_eq!(*pos, Position { x: 1.0, y: 2.0 });

        let vel = world.get::<Velocity>(e).unwrap();
        assert_eq!(*vel, Velocity { dx: 3.0, dy: 4.0 });

        let large = world.get::<LargeComponent>(e).unwrap();
        assert_eq!(large.data[0], 42);
        assert_eq!(large.data[63], 42);
    }
}
