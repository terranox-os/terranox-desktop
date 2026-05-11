use core::any::{Any, TypeId};

use alloc::boxed::Box;
use alloc::collections::BTreeMap;

/// Marker trait for resources. Resources are singleton data stored in the World
/// (e.g., Framebuffer, Time, InputEvents).
pub trait Resource: 'static {}

/// Type-erased resource storage.
pub struct Resources {
    data: BTreeMap<TypeId, Box<dyn Any>>,
}

impl Default for Resources {
    fn default() -> Self {
        Self::new()
    }
}

impl Resources {
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
        }
    }

    /// Insert a resource. If a resource of this type already exists, it is replaced.
    pub fn insert<R: Resource>(&mut self, resource: R) {
        self.data.insert(TypeId::of::<R>(), Box::new(resource));
    }

    /// Get a shared reference to a resource.
    pub fn get<R: Resource>(&self) -> Option<&R> {
        self.data
            .get(&TypeId::of::<R>())
            .and_then(|b| b.downcast_ref::<R>())
    }

    /// Get a mutable reference to a resource.
    pub fn get_mut<R: Resource>(&mut self) -> Option<&mut R> {
        self.data
            .get_mut(&TypeId::of::<R>())
            .and_then(|b| b.downcast_mut::<R>())
    }

    /// Check if a resource of this type exists.
    pub fn contains<R: Resource>(&self) -> bool {
        self.data.contains_key(&TypeId::of::<R>())
    }

    /// Remove a resource. Returns true if it existed.
    pub fn remove<R: Resource>(&mut self) -> bool {
        self.data.remove(&TypeId::of::<R>()).is_some()
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use super::*;

    struct Time {
        elapsed: f64,
    }
    impl Resource for Time {}

    struct Framebuffer {
        width: u32,
        height: u32,
    }
    impl Resource for Framebuffer {}

    struct InputEvents {
        count: u32,
    }
    impl Resource for InputEvents {}

    #[test]
    fn insert_and_get_roundtrip() {
        let mut resources = Resources::new();
        resources.insert(Time { elapsed: 1.5 });
        let time = resources.get::<Time>().unwrap();
        assert_eq!(time.elapsed, 1.5);
    }

    #[test]
    fn get_mut_modifies_resource() {
        let mut resources = Resources::new();
        resources.insert(Time { elapsed: 0.0 });
        {
            let time = resources.get_mut::<Time>().unwrap();
            time.elapsed = 42.0;
        }
        let time = resources.get::<Time>().unwrap();
        assert_eq!(time.elapsed, 42.0);
    }

    #[test]
    fn contains_returns_true_when_present() {
        let mut resources = Resources::new();
        assert!(!resources.contains::<Time>());
        resources.insert(Time { elapsed: 0.0 });
        assert!(resources.contains::<Time>());
    }

    #[test]
    fn contains_returns_false_for_absent() {
        let resources = Resources::new();
        assert!(!resources.contains::<Time>());
    }

    #[test]
    fn remove_returns_true_and_removes() {
        let mut resources = Resources::new();
        resources.insert(Time { elapsed: 1.0 });
        assert!(resources.remove::<Time>());
        assert!(!resources.contains::<Time>());
        assert!(resources.get::<Time>().is_none());
    }

    #[test]
    fn remove_returns_false_for_absent() {
        let mut resources = Resources::new();
        assert!(!resources.remove::<Time>());
    }

    #[test]
    fn different_types_coexist() {
        let mut resources = Resources::new();
        resources.insert(Time { elapsed: 1.0 });
        resources.insert(Framebuffer {
            width: 1920,
            height: 1080,
        });
        resources.insert(InputEvents { count: 5 });

        assert_eq!(resources.get::<Time>().unwrap().elapsed, 1.0);
        assert_eq!(resources.get::<Framebuffer>().unwrap().width, 1920);
        assert_eq!(resources.get::<Framebuffer>().unwrap().height, 1080);
        assert_eq!(resources.get::<InputEvents>().unwrap().count, 5);
    }

    #[test]
    fn insert_replaces_existing() {
        let mut resources = Resources::new();
        resources.insert(Time { elapsed: 1.0 });
        resources.insert(Time { elapsed: 99.0 });
        assert_eq!(resources.get::<Time>().unwrap().elapsed, 99.0);
    }

    #[test]
    fn get_returns_none_for_absent() {
        let resources = Resources::new();
        assert!(resources.get::<Time>().is_none());
    }

    #[test]
    fn get_mut_returns_none_for_absent() {
        let mut resources = Resources::new();
        assert!(resources.get_mut::<Time>().is_none());
    }
}
