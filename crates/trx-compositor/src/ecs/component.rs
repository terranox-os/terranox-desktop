use alloc::vec::Vec;
use core::any::TypeId;

/// Marker trait for ECS components. All components must be 'static + Sized.
pub trait Component: 'static + Sized {}

/// Dense component ID assigned at registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ComponentId(pub u32);

/// Registry mapping TypeId to ComponentId.
pub struct ComponentRegistry {
    /// Registered component info entries.
    entries: Vec<ComponentInfo>,
}

#[derive(Clone)]
pub struct ComponentInfo {
    pub type_id: TypeId,
    pub size: usize,
    pub align: usize,
    pub name: &'static str,
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentRegistry {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Register a component type. Returns its ComponentId.
    /// Idempotent -- returns existing ID if already registered.
    pub fn register<T: Component>(&mut self) -> ComponentId {
        let type_id = TypeId::of::<T>();
        for (i, entry) in self.entries.iter().enumerate() {
            if entry.type_id == type_id {
                return ComponentId(i as u32);
            }
        }
        let id = ComponentId(self.entries.len() as u32);
        self.entries.push(ComponentInfo {
            type_id,
            size: core::mem::size_of::<T>(),
            align: core::mem::align_of::<T>(),
            name: core::any::type_name::<T>(),
        });
        id
    }

    /// Look up a component ID by type.
    pub fn get_id<T: Component>(&self) -> Option<ComponentId> {
        let type_id = TypeId::of::<T>();
        self.entries
            .iter()
            .position(|e| e.type_id == type_id)
            .map(|i| ComponentId(i as u32))
    }

    pub fn get_info(&self, id: ComponentId) -> Option<&ComponentInfo> {
        self.entries.get(id.0 as usize)
    }

    pub fn count(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Position {
        _x: f32,
        _y: f32,
    }
    impl Component for Position {}

    struct Velocity {
        _dx: f32,
        _dy: f32,
    }
    impl Component for Velocity {}

    struct Health {
        _hp: u32,
    }
    impl Component for Health {}

    #[test]
    fn register_returns_sequential_ids() {
        let mut reg = ComponentRegistry::new();
        let id0 = reg.register::<Position>();
        let id1 = reg.register::<Velocity>();
        assert_eq!(id0, ComponentId(0));
        assert_eq!(id1, ComponentId(1));
        assert_eq!(reg.count(), 2);
    }

    #[test]
    fn register_is_idempotent() {
        let mut reg = ComponentRegistry::new();
        let id0 = reg.register::<Position>();
        let id1 = reg.register::<Position>();
        assert_eq!(id0, id1);
        assert_eq!(reg.count(), 1);
    }

    #[test]
    fn get_id_returns_none_for_unregistered() {
        let reg = ComponentRegistry::new();
        assert_eq!(reg.get_id::<Position>(), None);
    }

    #[test]
    fn get_id_returns_correct_id() {
        let mut reg = ComponentRegistry::new();
        let id = reg.register::<Velocity>();
        assert_eq!(reg.get_id::<Velocity>(), Some(id));
    }

    #[test]
    fn different_types_get_different_ids() {
        let mut reg = ComponentRegistry::new();
        let id_pos = reg.register::<Position>();
        let id_vel = reg.register::<Velocity>();
        let id_hp = reg.register::<Health>();
        assert_ne!(id_pos, id_vel);
        assert_ne!(id_vel, id_hp);
        assert_ne!(id_pos, id_hp);
    }

    #[test]
    fn get_info_returns_correct_metadata() {
        let mut reg = ComponentRegistry::new();
        let id = reg.register::<Position>();
        let info = reg.get_info(id).unwrap();
        assert_eq!(info.size, core::mem::size_of::<Position>());
        assert_eq!(info.align, core::mem::align_of::<Position>());
    }

    #[test]
    fn get_info_out_of_bounds() {
        let reg = ComponentRegistry::new();
        assert!(reg.get_info(ComponentId(99)).is_none());
    }
}
