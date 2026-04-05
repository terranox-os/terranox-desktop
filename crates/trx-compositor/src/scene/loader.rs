use crate::components::{
    BackgroundColor, BorderColor, BorderRadius, BorderWidth, Color, FlexboxLayout, FontSize,
    Focusable, GlobalTransform, Interaction, Opacity, Position, Size, TextColor, TextContent,
    Visible, Window, WindowTitle, ZIndex,
};
use crate::ecs::entity::Entity;
use crate::ecs::hierarchy::{Children, Parent};
use crate::ecs::plugin::{App, Plugin};
use crate::ecs::resource::Resource;
use crate::ecs::system::into_system;
use crate::ecs::world::World;
use crate::scene::format::ComponentTag;
use crate::scene::serialize::{SceneComponent, SceneData};

use alloc::vec::Vec;

use crate::components::{AlignItems, Edges, FlexDirection, JustifyContent};

/// Resource holding the scene data to be loaded on startup.
pub struct PendingScene {
    pub data: SceneData,
}
impl Resource for PendingScene {}

/// Plugin that loads a binary scene into the ECS World during the startup phase.
pub struct SceneLoaderPlugin {
    pub scene: SceneData,
}

impl Plugin for SceneLoaderPlugin {
    fn build(&self, app: &mut App) {
        // Register all component types that scenes can use
        app.world.register_component::<Position>();
        app.world.register_component::<Size>();
        app.world.register_component::<ZIndex>();
        app.world.register_component::<BackgroundColor>();
        app.world.register_component::<BorderColor>();
        app.world.register_component::<BorderWidth>();
        app.world.register_component::<BorderRadius>();
        app.world.register_component::<Opacity>();
        app.world.register_component::<Visible>();
        app.world.register_component::<FlexboxLayout>();
        app.world.register_component::<TextContent>();
        app.world.register_component::<FontSize>();
        app.world.register_component::<TextColor>();
        app.world.register_component::<Focusable>();
        app.world.register_component::<Window>();
        app.world.register_component::<WindowTitle>();
        app.world.register_component::<GlobalTransform>();
        app.world.register_component::<Interaction>();
        app.world.register_component::<Parent>();
        app.world.register_component::<Children>();

        // Store scene data as resource for the startup system
        app.insert_resource(PendingScene {
            data: self.scene.clone(),
        });

        // Add startup system
        app.add_startup_system(into_system("load_scene", load_scene_system));
    }

    fn name(&self) -> &str {
        "SceneLoaderPlugin"
    }
}

fn direction_from_u8(v: u8) -> FlexDirection {
    match v {
        1 => FlexDirection::Column,
        _ => FlexDirection::Row,
    }
}

fn align_from_u8(v: u8) -> AlignItems {
    match v {
        1 => AlignItems::End,
        2 => AlignItems::Center,
        3 => AlignItems::Stretch,
        _ => AlignItems::Start,
    }
}

fn justify_from_u8(v: u8) -> JustifyContent {
    match v {
        1 => JustifyContent::End,
        2 => JustifyContent::Center,
        3 => JustifyContent::SpaceBetween,
        4 => JustifyContent::SpaceAround,
        _ => JustifyContent::Start,
    }
}

/// Return the ComponentTag value for a SceneComponent, used to sort
/// components into ascending registration order before ECS insertion.
fn scene_component_tag(comp: &SceneComponent) -> u8 {
    match comp {
        SceneComponent::Position(..) => ComponentTag::Position as u8,
        SceneComponent::Size(..) => ComponentTag::Size as u8,
        SceneComponent::ZIndex(..) => ComponentTag::ZIndex as u8,
        SceneComponent::BackgroundColor(..) => ComponentTag::BackgroundColor as u8,
        SceneComponent::BorderColor(..) => ComponentTag::BorderColor as u8,
        SceneComponent::BorderWidth(..) => ComponentTag::BorderWidth as u8,
        SceneComponent::BorderRadius(..) => ComponentTag::BorderRadius as u8,
        SceneComponent::Opacity(..) => ComponentTag::Opacity as u8,
        SceneComponent::Visible(..) => ComponentTag::Visible as u8,
        SceneComponent::FlexboxLayout { .. } => ComponentTag::FlexboxLayout as u8,
        SceneComponent::TextContent(..) => ComponentTag::TextContent as u8,
        SceneComponent::FontSize(..) => ComponentTag::FontSize as u8,
        SceneComponent::TextColor(..) => ComponentTag::TextColor as u8,
        SceneComponent::Focusable => ComponentTag::Focusable as u8,
        SceneComponent::Window => ComponentTag::Window as u8,
        SceneComponent::WindowTitle(..) => ComponentTag::WindowTitle as u8,
    }
}

fn load_scene_system(world: &mut World) {
    // Take the pending scene resource
    let scene = match world.get_resource::<PendingScene>() {
        Some(s) => s.data.clone(),
        None => return,
    };

    // Spawn entities and collect handles.
    //
    // IMPORTANT: Components must be inserted in ascending ComponentId order
    // because the ECS archetype storage sorts component IDs but not the
    // corresponding column sizes. Inserting out of order leads to column
    // size mismatches and memory corruption. Scene components (IDs 0..15)
    // are inserted first, then runtime components (GlobalTransform=16,
    // Interaction=17).
    let mut entity_map: Vec<Entity> = Vec::with_capacity(scene.nodes.len());
    for node in &scene.nodes {
        let entity = world.spawn_empty();

        // Sort scene components by tag value (ascending) to match
        // the ascending ComponentId registration order.
        let mut sorted_comps = node.components.clone();
        sorted_comps.sort_by_key(scene_component_tag);

        // Insert scene components first (lower component IDs)
        for comp in &sorted_comps {
            match comp {
                SceneComponent::Position(x, y) => {
                    world.insert(entity, Position { x: *x, y: *y });
                }
                SceneComponent::Size(w, h) => {
                    world.insert(
                        entity,
                        Size {
                            width: *w,
                            height: *h,
                        },
                    );
                }
                SceneComponent::ZIndex(z) => {
                    world.insert(entity, ZIndex(*z));
                }
                SceneComponent::BackgroundColor(r, g, b, a) => {
                    world.insert(
                        entity,
                        BackgroundColor(Color {
                            r: *r,
                            g: *g,
                            b: *b,
                            a: *a,
                        }),
                    );
                }
                SceneComponent::BorderColor(r, g, b, a) => {
                    world.insert(
                        entity,
                        BorderColor(Color {
                            r: *r,
                            g: *g,
                            b: *b,
                            a: *a,
                        }),
                    );
                }
                SceneComponent::BorderWidth(v) => {
                    world.insert(entity, BorderWidth(*v));
                }
                SceneComponent::BorderRadius(v) => {
                    world.insert(entity, BorderRadius(*v));
                }
                SceneComponent::Opacity(v) => {
                    world.insert(entity, Opacity(*v));
                }
                SceneComponent::Visible(v) => {
                    world.insert(entity, Visible(*v));
                }
                SceneComponent::FlexboxLayout {
                    direction,
                    align,
                    justify,
                    gap,
                    padding,
                    margin,
                } => {
                    world.insert(
                        entity,
                        FlexboxLayout {
                            direction: direction_from_u8(*direction),
                            align_items: align_from_u8(*align),
                            justify_content: justify_from_u8(*justify),
                            gap: *gap,
                            padding: Edges {
                                top: padding[0],
                                right: padding[1],
                                bottom: padding[2],
                                left: padding[3],
                            },
                            margin: Edges {
                                top: margin[0],
                                right: margin[1],
                                bottom: margin[2],
                                left: margin[3],
                            },
                        },
                    );
                }
                SceneComponent::TextContent(text) => {
                    let mut bytes = [0u8; 256];
                    let len = text.len().min(256);
                    bytes[..len].copy_from_slice(&text[..len]);
                    world.insert(entity, TextContent { bytes, len });
                }
                SceneComponent::FontSize(s) => {
                    world.insert(entity, FontSize(*s));
                }
                SceneComponent::TextColor(r, g, b, a) => {
                    world.insert(
                        entity,
                        TextColor(Color {
                            r: *r,
                            g: *g,
                            b: *b,
                            a: *a,
                        }),
                    );
                }
                SceneComponent::Focusable => {
                    world.insert(entity, Focusable);
                }
                SceneComponent::Window => {
                    world.insert(entity, Window);
                }
                SceneComponent::WindowTitle(title) => {
                    let mut bytes = [0u8; 64];
                    let len = title.len().min(64);
                    bytes[..len].copy_from_slice(&title[..len]);
                    world.insert(entity, WindowTitle { bytes, len });
                }
            }
        }

        // Add runtime components AFTER scene components (higher component IDs)
        world.insert(entity, GlobalTransform::default());
        world.insert(entity, Interaction::default());

        entity_map.push(entity);
    }

    // Set up hierarchy (Parent + Children)
    for (i, node) in scene.nodes.iter().enumerate() {
        if let Some(parent_idx) = node.parent_index {
            if (parent_idx as usize) < entity_map.len() {
                let child = entity_map[i];
                let parent = entity_map[parent_idx as usize];
                world.insert(child, Parent(parent));

                // Add to parent's Children
                if let Some(children) = world.get_mut::<Children>(parent) {
                    children.add(child);
                } else {
                    let mut c = Children::new();
                    c.add(child);
                    world.insert(parent, c);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use alloc::vec;

    use super::*;
    use crate::ecs::plugin::App;
    use crate::scene::serialize::SceneNode;

    #[test]
    fn load_empty_scene() {
        let mut app = App::new();
        app.add_plugin(SceneLoaderPlugin {
            scene: SceneData::new(),
        });
        app.update();
        assert_eq!(app.world.entity_count(), 0);
    }

    #[test]
    fn load_single_entity_with_position() {
        let scene = SceneData {
            nodes: vec![SceneNode {
                components: vec![SceneComponent::Position(10.0, 20.0)],
                parent_index: None,
                child_indices: vec![],
            }],
        };

        let mut app = App::new();
        app.add_plugin(SceneLoaderPlugin { scene });
        app.update();

        assert_eq!(app.world.entity_count(), 1);
        // Entity 0 should have Position
        let entity = Entity { id: 0, generation: 0 };
        let pos = app.world.get::<Position>(entity).unwrap();
        assert_eq!(pos.x, 10.0);
        assert_eq!(pos.y, 20.0);
        // Should also have runtime components
        assert!(app.world.get::<GlobalTransform>(entity).is_some());
        assert!(app.world.get::<Interaction>(entity).is_some());
    }

    #[test]
    fn load_entity_with_all_component_types() {
        let scene = SceneData {
            nodes: vec![SceneNode {
                components: vec![
                    SceneComponent::Position(1.0, 2.0),
                    SceneComponent::Size(100.0, 50.0),
                    SceneComponent::ZIndex(5),
                    SceneComponent::BackgroundColor(255, 0, 0, 255),
                    SceneComponent::BorderColor(0, 255, 0, 128),
                    SceneComponent::BorderWidth(2.0),
                    SceneComponent::BorderRadius(4.0),
                    SceneComponent::Opacity(0.5),
                    SceneComponent::Visible(true),
                    SceneComponent::FontSize(14),
                    SceneComponent::TextColor(0, 0, 0, 255),
                    SceneComponent::Focusable,
                    SceneComponent::Window,
                    SceneComponent::WindowTitle(b"Hello".to_vec()),
                    SceneComponent::TextContent(b"World".to_vec()),
                ],
                parent_index: None,
                child_indices: vec![],
            }],
        };

        let mut app = App::new();
        app.add_plugin(SceneLoaderPlugin { scene });
        app.update();

        let entity = Entity { id: 0, generation: 0 };
        assert!(app.world.get::<Position>(entity).is_some());
        assert!(app.world.get::<Size>(entity).is_some());
        assert_eq!(app.world.get::<ZIndex>(entity).unwrap().0, 5);
        assert_eq!(
            app.world
                .get::<BackgroundColor>(entity)
                .unwrap()
                .0
                .r,
            255
        );
        assert_eq!(app.world.get::<BorderWidth>(entity).unwrap().0, 2.0);
        assert_eq!(app.world.get::<BorderRadius>(entity).unwrap().0, 4.0);
        assert_eq!(app.world.get::<Opacity>(entity).unwrap().0, 0.5);
        assert!(app.world.get::<Visible>(entity).unwrap().0);
        assert_eq!(app.world.get::<FontSize>(entity).unwrap().0, 14);
        assert!(app.world.get::<Focusable>(entity).is_some());
        assert!(app.world.get::<Window>(entity).is_some());

        let title = app.world.get::<WindowTitle>(entity).unwrap();
        assert_eq!(title.as_str(), "Hello");

        let text = app.world.get::<TextContent>(entity).unwrap();
        assert_eq!(text.as_str(), "World");
    }

    #[test]
    fn load_hierarchy() {
        let scene = SceneData {
            nodes: vec![
                SceneNode {
                    components: vec![SceneComponent::Window],
                    parent_index: None,
                    child_indices: vec![1, 2],
                },
                SceneNode {
                    components: vec![SceneComponent::Position(10.0, 0.0)],
                    parent_index: Some(0),
                    child_indices: vec![],
                },
                SceneNode {
                    components: vec![SceneComponent::Position(20.0, 0.0)],
                    parent_index: Some(0),
                    child_indices: vec![],
                },
            ],
        };

        let mut app = App::new();
        app.add_plugin(SceneLoaderPlugin { scene });
        app.update();

        assert_eq!(app.world.entity_count(), 3);

        let parent_entity = Entity { id: 0, generation: 0 };
        let child1 = Entity { id: 1, generation: 0 };
        let child2 = Entity { id: 2, generation: 0 };

        // Children of parent
        let children = app.world.get::<Children>(parent_entity).unwrap();
        assert_eq!(children.len(), 2);

        // Parent of children
        assert_eq!(app.world.get::<Parent>(child1).unwrap().0, parent_entity);
        assert_eq!(app.world.get::<Parent>(child2).unwrap().0, parent_entity);
    }

    #[test]
    fn load_flexbox_layout() {
        let scene = SceneData {
            nodes: vec![SceneNode {
                components: vec![SceneComponent::FlexboxLayout {
                    direction: 1,  // Column
                    align: 2,      // Center
                    justify: 3,    // SpaceBetween
                    gap: 8.0,
                    padding: [1.0, 2.0, 3.0, 4.0],
                    margin: [5.0, 6.0, 7.0, 8.0],
                }],
                parent_index: None,
                child_indices: vec![],
            }],
        };

        let mut app = App::new();
        app.add_plugin(SceneLoaderPlugin { scene });
        app.update();

        let entity = Entity { id: 0, generation: 0 };
        let layout = app.world.get::<FlexboxLayout>(entity).unwrap();
        assert_eq!(layout.direction, FlexDirection::Column);
        assert_eq!(layout.align_items, AlignItems::Center);
        assert_eq!(layout.justify_content, JustifyContent::SpaceBetween);
        assert_eq!(layout.gap, 8.0);
        assert_eq!(layout.padding.top, 1.0);
        assert_eq!(layout.padding.right, 2.0);
        assert_eq!(layout.padding.bottom, 3.0);
        assert_eq!(layout.padding.left, 4.0);
        assert_eq!(layout.margin.top, 5.0);
        assert_eq!(layout.margin.right, 6.0);
        assert_eq!(layout.margin.bottom, 7.0);
        assert_eq!(layout.margin.left, 8.0);
    }

    #[test]
    fn plugin_name() {
        let plugin = SceneLoaderPlugin {
            scene: SceneData::new(),
        };
        assert_eq!(plugin.name(), "SceneLoaderPlugin");
    }
}
