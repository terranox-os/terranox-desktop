use alloc::boxed::Box;

use crate::ecs::resource::Resource;
use crate::ecs::schedule::Schedule;
use crate::ecs::system::System;
use crate::ecs::world::World;

/// A plugin adds systems, resources, and components to an App.
pub trait Plugin {
    fn build(&self, app: &mut App);
    fn name(&self) -> &str;
}

/// The App: owns the World and Schedules.
pub struct App {
    pub world: World,
    startup: Schedule,
    update: Schedule,
    has_run_startup: bool,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            world: World::new(),
            startup: Schedule::new(),
            update: Schedule::new(),
            has_run_startup: false,
        }
    }

    /// Install a plugin into the app.
    pub fn add_plugin<P: Plugin>(&mut self, plugin: P) -> &mut Self {
        plugin.build(self);
        self
    }

    /// Add a system that runs once during startup.
    pub fn add_startup_system(&mut self, system: Box<dyn System>) -> &mut Self {
        self.startup.add_system(system);
        self
    }

    /// Add a system that runs every frame.
    pub fn add_system(&mut self, system: Box<dyn System>) -> &mut Self {
        self.update.add_system(system);
        self
    }

    /// Insert a resource into the world.
    pub fn insert_resource<R: Resource>(&mut self, resource: R) -> &mut Self {
        self.world.insert_resource(resource);
        self
    }

    /// Run one frame: startup (once) + update.
    pub fn update(&mut self) {
        if !self.has_run_startup {
            self.startup.run(&mut self.world);
            self.has_run_startup = true;
        }
        self.update.run(&mut self.world);
        self.world.increment_tick();
    }

    /// Run the app for N frames (useful for testing).
    pub fn run_frames(&mut self, n: u32) {
        for _ in 0..n {
            self.update();
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use alloc::vec::Vec;

    use super::*;
    use crate::ecs::component::Component;
    use crate::ecs::resource::Resource;
    use crate::ecs::system::into_system;

    #[derive(Debug)]
    struct FrameCount {
        value: u32,
    }
    impl Resource for FrameCount {}

    #[derive(Debug)]
    struct StartupFlag {
        initialized: bool,
    }
    impl Resource for StartupFlag {}

    struct ExecutionLog {
        entries: Vec<&'static str>,
    }
    impl Resource for ExecutionLog {}

    #[derive(Debug, PartialEq, Clone, Copy)]
    struct Position {
        x: f32,
        y: f32,
    }
    impl Component for Position {}

    #[test]
    fn app_new_creates_empty() {
        let app = App::new();
        assert_eq!(app.world.entity_count(), 0);
        assert_eq!(app.world.current_tick(), 0);
    }

    #[test]
    fn insert_resource_accessible_from_world() {
        let mut app = App::new();
        app.insert_resource(FrameCount { value: 42 });
        assert_eq!(app.world.get_resource::<FrameCount>().unwrap().value, 42);
    }

    #[test]
    fn startup_runs_once() {
        let mut app = App::new();
        app.insert_resource(StartupFlag { initialized: false });

        app.add_startup_system(into_system("init", |w: &mut World| {
            w.get_resource_mut::<StartupFlag>().unwrap().initialized = true;
        }));

        app.update();
        assert!(app.world.get_resource::<StartupFlag>().unwrap().initialized);

        // Mutate it back
        app.world.get_resource_mut::<StartupFlag>().unwrap().initialized = false;

        // Second update should NOT run startup again
        app.update();
        assert!(!app.world.get_resource::<StartupFlag>().unwrap().initialized);
    }

    #[test]
    fn update_runs_each_frame() {
        let mut app = App::new();
        app.insert_resource(FrameCount { value: 0 });

        app.add_system(into_system("count_frames", |w: &mut World| {
            w.get_resource_mut::<FrameCount>().unwrap().value += 1;
        }));

        app.update();
        app.update();
        app.update();

        assert_eq!(app.world.get_resource::<FrameCount>().unwrap().value, 3);
    }

    #[test]
    fn run_frames_helper() {
        let mut app = App::new();
        app.insert_resource(FrameCount { value: 0 });

        app.add_system(into_system("count", |w: &mut World| {
            w.get_resource_mut::<FrameCount>().unwrap().value += 1;
        }));

        app.run_frames(5);
        assert_eq!(app.world.get_resource::<FrameCount>().unwrap().value, 5);
    }

    #[test]
    fn tick_increments_each_frame() {
        let mut app = App::new();
        app.run_frames(3);
        assert_eq!(app.world.current_tick(), 3);
    }

    #[test]
    fn add_plugin_installs_systems_and_resources() {
        struct MyPlugin;

        impl Plugin for MyPlugin {
            fn build(&self, app: &mut App) {
                app.insert_resource(FrameCount { value: 100 });
                app.add_system(into_system("plugin_sys", |w: &mut World| {
                    w.get_resource_mut::<FrameCount>().unwrap().value += 1;
                }));
            }

            fn name(&self) -> &str {
                "MyPlugin"
            }
        }

        let mut app = App::new();
        app.add_plugin(MyPlugin);

        assert_eq!(app.world.get_resource::<FrameCount>().unwrap().value, 100);

        app.update();
        assert_eq!(app.world.get_resource::<FrameCount>().unwrap().value, 101);
    }

    #[test]
    fn multiple_plugins() {
        struct PluginA;
        impl Plugin for PluginA {
            fn build(&self, app: &mut App) {
                app.add_system(into_system("plugin_a", |w: &mut World| {
                    w.get_resource_mut::<ExecutionLog>()
                        .unwrap()
                        .entries
                        .push("a");
                }));
            }
            fn name(&self) -> &str {
                "PluginA"
            }
        }

        struct PluginB;
        impl Plugin for PluginB {
            fn build(&self, app: &mut App) {
                app.add_system(into_system("plugin_b", |w: &mut World| {
                    w.get_resource_mut::<ExecutionLog>()
                        .unwrap()
                        .entries
                        .push("b");
                }));
            }
            fn name(&self) -> &str {
                "PluginB"
            }
        }

        let mut app = App::new();
        app.insert_resource(ExecutionLog {
            entries: Vec::new(),
        });
        app.add_plugin(PluginA);
        app.add_plugin(PluginB);

        app.update();

        let log = app.world.get_resource::<ExecutionLog>().unwrap();
        assert_eq!(log.entries.len(), 2);
        assert_eq!(log.entries[0], "a");
        assert_eq!(log.entries[1], "b");
    }

    #[test]
    fn startup_and_update_combined() {
        let mut app = App::new();
        app.insert_resource(ExecutionLog {
            entries: Vec::new(),
        });

        app.add_startup_system(into_system("startup_sys", |w: &mut World| {
            w.get_resource_mut::<ExecutionLog>()
                .unwrap()
                .entries
                .push("startup");
        }));
        app.add_system(into_system("update_sys", |w: &mut World| {
            w.get_resource_mut::<ExecutionLog>()
                .unwrap()
                .entries
                .push("update");
        }));

        app.update(); // startup + update
        app.update(); // update only

        let log = app.world.get_resource::<ExecutionLog>().unwrap();
        assert_eq!(log.entries, alloc::vec!["startup", "update", "update"]);
    }

    #[test]
    fn app_systems_can_spawn_entities() {
        let mut app = App::new();
        app.world.register_component::<Position>();

        app.add_system(into_system("spawner", |w: &mut World| {
            if w.entity_count() == 0 {
                w.spawn(Position { x: 1.0, y: 2.0 });
            }
        }));

        app.update();
        assert_eq!(app.world.entity_count(), 1);

        // Second update should not spawn again
        app.update();
        assert_eq!(app.world.entity_count(), 1);
    }
}
