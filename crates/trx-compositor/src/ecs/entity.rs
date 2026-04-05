use alloc::vec::Vec;

/// Generational entity identifier.
/// The `id` field indexes into entity storage. The `generation` field
/// prevents use-after-free: when an entity is despawned, its generation
/// increments, invalidating all existing Entity handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity {
    pub id: u32,
    pub generation: u32,
}

impl Entity {
    pub const PLACEHOLDER: Self = Self {
        id: u32::MAX,
        generation: 0,
    };
}

/// Allocator for entity IDs with generation tracking and recycling.
pub struct EntityAllocator {
    /// Current generation per slot. Index = entity id.
    generations: Vec<u32>,
    /// Free list of recycled entity IDs.
    free_list: Vec<u32>,
    /// Number of alive entities.
    alive_count: u32,
}

impl Default for EntityAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl EntityAllocator {
    pub fn new() -> Self {
        Self {
            generations: Vec::new(),
            free_list: Vec::new(),
            alive_count: 0,
        }
    }

    /// Allocate a new entity.
    pub fn allocate(&mut self) -> Entity {
        self.alive_count += 1;
        if let Some(id) = self.free_list.pop() {
            Entity {
                id,
                generation: self.generations[id as usize],
            }
        } else {
            let id = self.generations.len() as u32;
            self.generations.push(0);
            Entity { id, generation: 0 }
        }
    }

    /// Free an entity. Increments generation to invalidate existing handles.
    pub fn free(&mut self, entity: Entity) -> bool {
        if !self.is_alive(entity) {
            return false;
        }
        self.generations[entity.id as usize] += 1;
        self.free_list.push(entity.id);
        self.alive_count -= 1;
        true
    }

    /// Check if an entity handle is still valid.
    pub fn is_alive(&self, entity: Entity) -> bool {
        (entity.id as usize) < self.generations.len()
            && self.generations[entity.id as usize] == entity.generation
    }

    pub fn alive_count(&self) -> u32 {
        self.alive_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_returns_sequential_ids() {
        let mut alloc = EntityAllocator::new();
        let e0 = alloc.allocate();
        let e1 = alloc.allocate();
        let e2 = alloc.allocate();
        assert_eq!(e0.id, 0);
        assert_eq!(e1.id, 1);
        assert_eq!(e2.id, 2);
        assert_eq!(e0.generation, 0);
        assert_eq!(e1.generation, 0);
        assert_eq!(e2.generation, 0);
    }

    #[test]
    fn is_alive_for_live_entity() {
        let mut alloc = EntityAllocator::new();
        let e = alloc.allocate();
        assert!(alloc.is_alive(e));
    }

    #[test]
    fn free_increments_generation() {
        let mut alloc = EntityAllocator::new();
        let e = alloc.allocate();
        assert!(alloc.free(e));
        assert!(!alloc.is_alive(e));
    }

    #[test]
    fn double_free_returns_false() {
        let mut alloc = EntityAllocator::new();
        let e = alloc.allocate();
        assert!(alloc.free(e));
        assert!(!alloc.free(e));
    }

    #[test]
    fn recycled_id_has_incremented_generation() {
        let mut alloc = EntityAllocator::new();
        let e0 = alloc.allocate();
        alloc.free(e0);
        let e1 = alloc.allocate();
        assert_eq!(e1.id, e0.id);
        assert_eq!(e1.generation, 1);
        assert!(!alloc.is_alive(e0));
        assert!(alloc.is_alive(e1));
    }

    #[test]
    fn alive_count_tracking() {
        let mut alloc = EntityAllocator::new();
        assert_eq!(alloc.alive_count(), 0);
        let e0 = alloc.allocate();
        let _e1 = alloc.allocate();
        assert_eq!(alloc.alive_count(), 2);
        alloc.free(e0);
        assert_eq!(alloc.alive_count(), 1);
    }

    #[test]
    fn placeholder_is_not_alive() {
        let alloc = EntityAllocator::new();
        assert!(!alloc.is_alive(Entity::PLACEHOLDER));
    }

    #[test]
    fn multiple_recycles() {
        let mut alloc = EntityAllocator::new();
        let e0 = alloc.allocate();
        alloc.free(e0);
        let e1 = alloc.allocate(); // reuses id 0, gen 1
        alloc.free(e1);
        let e2 = alloc.allocate(); // reuses id 0, gen 2
        assert_eq!(e2.id, 0);
        assert_eq!(e2.generation, 2);
        assert!(!alloc.is_alive(e0));
        assert!(!alloc.is_alive(e1));
        assert!(alloc.is_alive(e2));
    }
}
