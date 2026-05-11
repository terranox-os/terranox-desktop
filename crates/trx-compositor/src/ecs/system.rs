use alloc::boxed::Box;

use crate::ecs::world::World;

/// A system is a unit of logic that operates on the World.
pub trait System {
    /// Execute the system against the given world.
    fn run(&mut self, world: &mut World);

    /// Human-readable name for debugging and scheduling.
    fn name(&self) -> &str;
}

/// Adapter: wraps a function `fn(&mut World)` as a System.
pub struct FunctionSystem<F: FnMut(&mut World)> {
    func: F,
    name: &'static str,
}

impl<F: FnMut(&mut World)> FunctionSystem<F> {
    pub fn new(name: &'static str, func: F) -> Self {
        Self { func, name }
    }
}

impl<F: FnMut(&mut World)> System for FunctionSystem<F> {
    fn run(&mut self, world: &mut World) {
        (self.func)(world);
    }

    fn name(&self) -> &str {
        self.name
    }
}

/// Helper to create a boxed System from a closure.
pub fn into_system<F: FnMut(&mut World) + 'static>(name: &'static str, f: F) -> Box<dyn System> {
    Box::new(FunctionSystem::new(name, f))
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use super::*;
    use crate::ecs::component::Component;
    use crate::ecs::resource::Resource;

    #[derive(Debug, PartialEq)]
    struct Counter {
        value: u32,
    }
    impl Resource for Counter {}

    #[derive(Debug, PartialEq, Clone, Copy)]
    struct Position {
        x: f32,
        y: f32,
    }
    impl Component for Position {}

    #[test]
    fn function_system_runs() {
        let mut world = World::new();
        world.insert_resource(Counter { value: 0 });

        let mut system = FunctionSystem::new("increment", |w: &mut World| {
            let counter = w.get_resource_mut::<Counter>().unwrap();
            counter.value += 1;
        });

        system.run(&mut world);
        assert_eq!(world.get_resource::<Counter>().unwrap().value, 1);

        system.run(&mut world);
        assert_eq!(world.get_resource::<Counter>().unwrap().value, 2);
    }

    #[test]
    fn function_system_name() {
        let system = FunctionSystem::new("my_system", |_: &mut World| {});
        assert_eq!(system.name(), "my_system");
    }

    #[test]
    fn into_system_creates_boxed_system() {
        let mut world = World::new();
        world.insert_resource(Counter { value: 10 });

        let mut sys = into_system("double", |w: &mut World| {
            let counter = w.get_resource_mut::<Counter>().unwrap();
            counter.value *= 2;
        });

        sys.run(&mut world);
        assert_eq!(world.get_resource::<Counter>().unwrap().value, 20);
    }

    #[test]
    fn system_modifies_entities() {
        let mut world = World::new();
        world.register_component::<Position>();
        let e = world.spawn(Position { x: 0.0, y: 0.0 });

        let mut sys = into_system("move_right", move |w: &mut World| {
            let pos = w.get_mut::<Position>(e).unwrap();
            pos.x += 1.0;
        });

        sys.run(&mut world);
        sys.run(&mut world);
        assert_eq!(world.get::<Position>(e).unwrap().x, 2.0);
    }

    #[test]
    fn system_trait_object_dispatches() {
        let mut world = World::new();
        world.insert_resource(Counter { value: 0 });

        let mut sys: Box<dyn System> = into_system("inc", |w: &mut World| {
            w.get_resource_mut::<Counter>().unwrap().value += 5;
        });

        sys.run(&mut world);
        assert_eq!(world.get_resource::<Counter>().unwrap().value, 5);
    }
}
