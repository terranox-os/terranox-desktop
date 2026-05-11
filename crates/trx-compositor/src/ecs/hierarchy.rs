use alloc::vec::Vec;

use crate::ecs::component::Component;
use crate::ecs::entity::Entity;

/// Parent component -- points to parent entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Parent(pub Entity);
impl Component for Parent {}

/// Children component -- list of child entities.
#[derive(Debug, Clone)]
pub struct Children {
    pub entities: Vec<Entity>,
}

impl Default for Children {
    fn default() -> Self {
        Self::new()
    }
}

impl Children {
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
        }
    }

    pub fn with(entities: Vec<Entity>) -> Self {
        Self { entities }
    }

    pub fn add(&mut self, child: Entity) {
        self.entities.push(child);
    }

    pub fn remove(&mut self, child: Entity) {
        self.entities.retain(|&e| e != child);
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Entity> {
        self.entities.iter()
    }
}

impl Component for Children {}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use alloc::vec;

    use super::*;
    use crate::ecs::entity::Entity;

    fn entity(id: u32) -> Entity {
        Entity { id, generation: 0 }
    }

    #[test]
    fn parent_component_stores_entity() {
        let parent_entity = entity(0);
        let p = Parent(parent_entity);
        assert_eq!(p.0, parent_entity);
    }

    #[test]
    fn children_new_is_empty() {
        let children = Children::new();
        assert!(children.is_empty());
        assert_eq!(children.len(), 0);
    }

    #[test]
    fn children_add() {
        let mut children = Children::new();
        children.add(entity(1));
        children.add(entity(2));
        assert_eq!(children.len(), 2);
        assert!(!children.is_empty());
    }

    #[test]
    fn children_remove() {
        let mut children = Children::new();
        let e1 = entity(1);
        let e2 = entity(2);
        let e3 = entity(3);
        children.add(e1);
        children.add(e2);
        children.add(e3);

        children.remove(e2);
        assert_eq!(children.len(), 2);
        let ids: Vec<Entity> = children.iter().copied().collect();
        assert!(ids.contains(&e1));
        assert!(!ids.contains(&e2));
        assert!(ids.contains(&e3));
    }

    #[test]
    fn children_remove_nonexistent_is_noop() {
        let mut children = Children::new();
        children.add(entity(1));
        children.remove(entity(99));
        assert_eq!(children.len(), 1);
    }

    #[test]
    fn children_with_constructor() {
        let children = Children::with(vec![entity(10), entity(20), entity(30)]);
        assert_eq!(children.len(), 3);
    }

    #[test]
    fn children_iter() {
        let mut children = Children::new();
        children.add(entity(5));
        children.add(entity(10));
        let collected: Vec<&Entity> = children.iter().collect();
        assert_eq!(collected.len(), 2);
        assert_eq!(collected[0].id, 5);
        assert_eq!(collected[1].id, 10);
    }

    #[test]
    fn parent_and_children_as_components() {
        use crate::ecs::world::World;

        let mut world = World::new();
        world.register_component::<Parent>();
        world.register_component::<Children>();

        let parent_entity = world.spawn_empty();
        let child1 = world.spawn_empty();
        let child2 = world.spawn_empty();

        // Set parent on children
        world.insert(child1, Parent(parent_entity));
        world.insert(child2, Parent(parent_entity));

        // Set children on parent
        let mut ch = Children::new();
        ch.add(child1);
        ch.add(child2);
        world.insert(parent_entity, ch);

        // Verify
        assert_eq!(
            world.get::<Parent>(child1).unwrap().0,
            parent_entity
        );
        assert_eq!(
            world.get::<Parent>(child2).unwrap().0,
            parent_entity
        );
        assert_eq!(world.get::<Children>(parent_entity).unwrap().len(), 2);
    }
}
