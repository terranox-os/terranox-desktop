use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::ecs::component::ComponentId;
use crate::ecs::entity::Entity;
use crate::ecs::storage::ComponentColumn;

/// Unique archetype identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArchetypeId(pub u32);

/// An archetype stores all entities with the same set of components.
/// Components are stored in parallel columns for cache efficiency.
pub struct Archetype {
    pub id: ArchetypeId,
    /// Component IDs this archetype contains (sorted).
    pub component_ids: Vec<ComponentId>,
    /// Parallel columns -- one per component, indexed by position in component_ids.
    pub columns: Vec<ComponentColumn>,
    /// Entity stored at each row.
    pub entities: Vec<Entity>,
}

impl Archetype {
    pub fn new(
        id: ArchetypeId,
        component_ids: Vec<ComponentId>,
        sizes: &[(usize, usize)],
    ) -> Self {
        let columns = sizes
            .iter()
            .map(|&(size, align)| ComponentColumn::new(size, align))
            .collect();
        Self {
            id,
            component_ids,
            columns,
            entities: Vec::new(),
        }
    }

    /// Get the column index for a component ID within this archetype.
    pub fn column_index(&self, component_id: ComponentId) -> Option<usize> {
        self.component_ids
            .iter()
            .position(|&id| id == component_id)
    }

    /// Check if this archetype has a specific component.
    pub fn has_component(&self, component_id: ComponentId) -> bool {
        self.component_ids.contains(&component_id)
    }

    /// Number of entities in this archetype.
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }
}

/// Stores all archetypes and provides lookup by component set.
pub struct Archetypes {
    archetypes: Vec<Archetype>,
    /// Maps sorted component ID sets to archetype index.
    index: BTreeMap<Vec<ComponentId>, ArchetypeId>,
}

impl Default for Archetypes {
    fn default() -> Self {
        Self::new()
    }
}

impl Archetypes {
    pub fn new() -> Self {
        Self {
            archetypes: Vec::new(),
            index: BTreeMap::new(),
        }
    }

    /// Get or create an archetype for the given component set.
    ///
    /// `component_ids` and `sizes` must be parallel arrays (same length,
    /// sizes[i] corresponds to component_ids[i]). Both are sorted by
    /// component ID before creating the archetype, so callers do not
    /// need to pre-sort.
    pub fn get_or_create(
        &mut self,
        component_ids: Vec<ComponentId>,
        sizes: &[(usize, usize)],
    ) -> ArchetypeId {
        // Sort component_ids AND reorder sizes to match.
        // Create index permutation, sort by component ID, then apply.
        let mut indices: Vec<usize> = (0..component_ids.len()).collect();
        indices.sort_by_key(|&i| component_ids[i]);
        let sorted_ids: Vec<ComponentId> = indices.iter().map(|&i| component_ids[i]).collect();
        let sorted_sizes: Vec<(usize, usize)> = indices.iter().map(|&i| sizes[i]).collect();

        if let Some(&id) = self.index.get(&sorted_ids) {
            return id;
        }
        let id = ArchetypeId(self.archetypes.len() as u32);
        self.archetypes
            .push(Archetype::new(id, sorted_ids.clone(), &sorted_sizes));
        self.index.insert(sorted_ids, id);
        id
    }

    pub fn get(&self, id: ArchetypeId) -> Option<&Archetype> {
        self.archetypes.get(id.0 as usize)
    }

    pub fn get_mut(&mut self, id: ArchetypeId) -> Option<&mut Archetype> {
        self.archetypes.get_mut(id.0 as usize)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Archetype> {
        self.archetypes.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Archetype> {
        self.archetypes.iter_mut()
    }

    pub fn count(&self) -> usize {
        self.archetypes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_archetype() {
        let mut archetypes = Archetypes::new();
        let ids = alloc::vec![ComponentId(0), ComponentId(1)];
        let sizes = &[(4, 4), (8, 8)];
        let arch_id = archetypes.get_or_create(ids, sizes);
        assert_eq!(arch_id, ArchetypeId(0));
        assert_eq!(archetypes.count(), 1);
    }

    #[test]
    fn get_or_create_is_idempotent() {
        let mut archetypes = Archetypes::new();
        let ids1 = alloc::vec![ComponentId(0), ComponentId(1)];
        let ids2 = alloc::vec![ComponentId(1), ComponentId(0)]; // reversed order
        let sizes = &[(4, 4), (8, 8)];
        let a = archetypes.get_or_create(ids1, sizes);
        let b = archetypes.get_or_create(ids2, sizes);
        assert_eq!(a, b);
        assert_eq!(archetypes.count(), 1);
    }

    #[test]
    fn different_component_sets_create_different_archetypes() {
        let mut archetypes = Archetypes::new();
        let a = archetypes.get_or_create(alloc::vec![ComponentId(0)], &[(4, 4)]);
        let b = archetypes.get_or_create(
            alloc::vec![ComponentId(0), ComponentId(1)],
            &[(4, 4), (8, 8)],
        );
        assert_ne!(a, b);
        assert_eq!(archetypes.count(), 2);
    }

    #[test]
    fn archetype_column_index() {
        let mut archetypes = Archetypes::new();
        let ids = alloc::vec![ComponentId(5), ComponentId(2)];
        let sizes = &[(4, 4), (8, 8)];
        let arch_id = archetypes.get_or_create(ids, sizes);
        let arch = archetypes.get(arch_id).unwrap();
        // After sorting: [ComponentId(2), ComponentId(5)]
        assert_eq!(arch.column_index(ComponentId(2)), Some(0));
        assert_eq!(arch.column_index(ComponentId(5)), Some(1));
        assert_eq!(arch.column_index(ComponentId(99)), None);
    }

    #[test]
    fn get_or_create_reorders_sizes_with_component_ids() {
        // Regression test: sizes must be reordered alongside component_ids
        // when get_or_create sorts. Previously, sizes stayed in original
        // order causing column size mismatches (SIGSEGV on access).
        let mut archetypes = Archetypes::new();
        // Component 10 has size 16, Component 3 has size 4
        // Passed in descending ID order (10, 3)
        let ids = alloc::vec![ComponentId(10), ComponentId(3)];
        let sizes = &[(16, 8), (4, 4)]; // sizes[0]=16 for ID 10, sizes[1]=4 for ID 3
        let arch_id = archetypes.get_or_create(ids, sizes);
        let arch = archetypes.get(arch_id).unwrap();

        // After sorting: [ComponentId(3), ComponentId(10)]
        // Column 0 should be ComponentId(3) with size 4
        // Column 1 should be ComponentId(10) with size 16
        assert_eq!(arch.component_ids[0], ComponentId(3));
        assert_eq!(arch.component_ids[1], ComponentId(10));
        assert_eq!(arch.columns[0].item_size(), 4);
        assert_eq!(arch.columns[1].item_size(), 16);
    }

    #[test]
    fn archetype_has_component() {
        let mut archetypes = Archetypes::new();
        let ids = alloc::vec![ComponentId(3)];
        let sizes = &[(4, 4)];
        let arch_id = archetypes.get_or_create(ids, sizes);
        let arch = archetypes.get(arch_id).unwrap();
        assert!(arch.has_component(ComponentId(3)));
        assert!(!arch.has_component(ComponentId(7)));
    }

    #[test]
    fn archetype_starts_empty() {
        let mut archetypes = Archetypes::new();
        let arch_id = archetypes.get_or_create(alloc::vec![ComponentId(0)], &[(4, 4)]);
        let arch = archetypes.get(arch_id).unwrap();
        assert!(arch.is_empty());
        assert_eq!(arch.len(), 0);
    }
}
