use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::ecs::system::System;
use crate::ecs::world::World;

/// A schedule owns and runs an ordered list of systems.
pub struct Schedule {
    systems: Vec<Box<dyn System>>,
}

impl Default for Schedule {
    fn default() -> Self {
        Self::new()
    }
}

impl Schedule {
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
        }
    }

    /// Add a system to the end of the schedule.
    pub fn add_system(&mut self, system: Box<dyn System>) {
        self.systems.push(system);
    }

    /// Run all systems in order.
    pub fn run(&mut self, world: &mut World) {
        for system in &mut self.systems {
            system.run(world);
        }
    }

    /// Number of systems in this schedule.
    pub fn system_count(&self) -> usize {
        self.systems.len()
    }

    /// Get system names (for debugging).
    pub fn system_names(&self) -> Vec<&str> {
        self.systems.iter().map(|s| s.name()).collect()
    }
}

/// Labels for different schedule stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleLabel {
    Startup,
    Update,
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use alloc::vec;
    use alloc::vec::Vec;
    use core::sync::atomic::{AtomicU32, Ordering};

    use super::*;
    use crate::ecs::resource::Resource;
    use crate::ecs::system::into_system;

    struct ExecutionLog {
        order: Vec<&'static str>,
    }
    impl Resource for ExecutionLog {}

    #[test]
    fn empty_schedule() {
        let mut schedule = Schedule::new();
        let mut world = World::new();
        schedule.run(&mut world); // should not panic
        assert_eq!(schedule.system_count(), 0);
    }

    #[test]
    fn add_and_run_systems() {
        let mut schedule = Schedule::new();
        let mut world = World::new();
        world.insert_resource(ExecutionLog {
            order: Vec::new(),
        });

        schedule.add_system(into_system("sys_a", |w: &mut World| {
            w.get_resource_mut::<ExecutionLog>().unwrap().order.push("a");
        }));
        schedule.add_system(into_system("sys_b", |w: &mut World| {
            w.get_resource_mut::<ExecutionLog>().unwrap().order.push("b");
        }));

        schedule.run(&mut world);

        let log = world.get_resource::<ExecutionLog>().unwrap();
        assert_eq!(log.order, vec!["a", "b"]);
    }

    #[test]
    fn systems_run_in_order() {
        let mut schedule = Schedule::new();
        let mut world = World::new();
        world.insert_resource(ExecutionLog {
            order: Vec::new(),
        });

        schedule.add_system(into_system("first", |w: &mut World| {
            w.get_resource_mut::<ExecutionLog>().unwrap().order.push("first");
        }));
        schedule.add_system(into_system("second", |w: &mut World| {
            w.get_resource_mut::<ExecutionLog>().unwrap().order.push("second");
        }));
        schedule.add_system(into_system("third", |w: &mut World| {
            w.get_resource_mut::<ExecutionLog>().unwrap().order.push("third");
        }));

        schedule.run(&mut world);

        let log = world.get_resource::<ExecutionLog>().unwrap();
        assert_eq!(log.order, vec!["first", "second", "third"]);
    }

    #[test]
    fn system_count_and_names() {
        let mut schedule = Schedule::new();
        schedule.add_system(into_system("alpha", |_: &mut World| {}));
        schedule.add_system(into_system("beta", |_: &mut World| {}));
        schedule.add_system(into_system("gamma", |_: &mut World| {}));

        assert_eq!(schedule.system_count(), 3);
        assert_eq!(schedule.system_names(), vec!["alpha", "beta", "gamma"]);
    }

    #[test]
    fn multiple_runs_accumulate() {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        COUNTER.store(0, Ordering::SeqCst);

        let mut schedule = Schedule::new();
        let mut world = World::new();

        schedule.add_system(into_system("inc", |_: &mut World| {
            COUNTER.fetch_add(1, Ordering::SeqCst);
        }));

        schedule.run(&mut world);
        schedule.run(&mut world);
        schedule.run(&mut world);

        assert_eq!(COUNTER.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn schedule_label_values() {
        assert_ne!(ScheduleLabel::Startup, ScheduleLabel::Update);
        let label = ScheduleLabel::Startup;
        assert_eq!(label, ScheduleLabel::Startup);
    }
}
