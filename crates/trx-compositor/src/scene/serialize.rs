use alloc::vec::Vec;

use crate::scene::format::{ComponentTag, NO_PARENT, SCENE_MAGIC, SCENE_VERSION};

/// A single component in the scene description, with its data.
#[derive(Debug, Clone)]
pub enum SceneComponent {
    Position(f32, f32),
    Size(f32, f32),
    ZIndex(i32),
    BackgroundColor(u8, u8, u8, u8),
    BorderColor(u8, u8, u8, u8),
    BorderWidth(f32),
    BorderRadius(f32),
    Opacity(f32),
    Visible(bool),
    FlexboxLayout {
        direction: u8,
        align: u8,
        justify: u8,
        gap: f32,
        padding: [f32; 4],
        margin: [f32; 4],
    },
    TextContent(Vec<u8>),
    FontSize(u16),
    TextColor(u8, u8, u8, u8),
    Focusable,
    Window,
    WindowTitle(Vec<u8>),
}

/// A node in the scene graph, holding components and hierarchy info.
#[derive(Debug, Clone)]
pub struct SceneNode {
    pub components: Vec<SceneComponent>,
    pub parent_index: Option<u32>,
    pub child_indices: Vec<u32>,
}

/// Top-level scene data: an ordered list of nodes.
#[derive(Debug, Clone)]
pub struct SceneData {
    pub nodes: Vec<SceneNode>,
}

impl SceneData {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }
}

impl Default for SceneData {
    fn default() -> Self {
        Self::new()
    }
}

/// Serialize a `SceneData` to the TRXS v1 binary format.
pub fn serialize_scene(scene: &SceneData) -> Vec<u8> {
    let mut buf = Vec::new();

    // Header: magic (4) + version (4) + entity count (4)
    buf.extend_from_slice(&SCENE_MAGIC);
    buf.extend_from_slice(&SCENE_VERSION.to_le_bytes());
    buf.extend_from_slice(&(scene.nodes.len() as u32).to_le_bytes());

    // Entities
    for node in &scene.nodes {
        // Component count (u8)
        buf.push(node.components.len() as u8);

        // Parent index (u32, NO_PARENT sentinel if none)
        buf.extend_from_slice(&node.parent_index.unwrap_or(NO_PARENT).to_le_bytes());

        // Child count (u8) + child indices (u32 each)
        buf.push(node.child_indices.len() as u8);
        for &idx in &node.child_indices {
            buf.extend_from_slice(&idx.to_le_bytes());
        }

        // Components
        for comp in &node.components {
            match comp {
                SceneComponent::Position(x, y) => {
                    buf.push(ComponentTag::Position as u8);
                    buf.extend_from_slice(&x.to_le_bytes());
                    buf.extend_from_slice(&y.to_le_bytes());
                }
                SceneComponent::Size(w, h) => {
                    buf.push(ComponentTag::Size as u8);
                    buf.extend_from_slice(&w.to_le_bytes());
                    buf.extend_from_slice(&h.to_le_bytes());
                }
                SceneComponent::ZIndex(z) => {
                    buf.push(ComponentTag::ZIndex as u8);
                    buf.extend_from_slice(&z.to_le_bytes());
                }
                SceneComponent::BackgroundColor(r, g, b, a)
                | SceneComponent::BorderColor(r, g, b, a)
                | SceneComponent::TextColor(r, g, b, a) => {
                    let tag = match comp {
                        SceneComponent::BackgroundColor(..) => ComponentTag::BackgroundColor,
                        SceneComponent::BorderColor(..) => ComponentTag::BorderColor,
                        SceneComponent::TextColor(..) => ComponentTag::TextColor,
                        _ => unreachable!(),
                    };
                    buf.push(tag as u8);
                    buf.extend_from_slice(&[*r, *g, *b, *a]);
                }
                SceneComponent::BorderWidth(v)
                | SceneComponent::BorderRadius(v)
                | SceneComponent::Opacity(v) => {
                    let tag = match comp {
                        SceneComponent::BorderWidth(..) => ComponentTag::BorderWidth,
                        SceneComponent::BorderRadius(..) => ComponentTag::BorderRadius,
                        SceneComponent::Opacity(..) => ComponentTag::Opacity,
                        _ => unreachable!(),
                    };
                    buf.push(tag as u8);
                    buf.extend_from_slice(&v.to_le_bytes());
                }
                SceneComponent::Visible(v) => {
                    buf.push(ComponentTag::Visible as u8);
                    buf.push(*v as u8);
                }
                SceneComponent::FlexboxLayout {
                    direction,
                    align,
                    justify,
                    gap,
                    padding,
                    margin,
                } => {
                    buf.push(ComponentTag::FlexboxLayout as u8);
                    buf.push(*direction);
                    buf.push(*align);
                    buf.push(*justify);
                    buf.extend_from_slice(&gap.to_le_bytes());
                    for p in padding {
                        buf.extend_from_slice(&p.to_le_bytes());
                    }
                    for m in margin {
                        buf.extend_from_slice(&m.to_le_bytes());
                    }
                }
                SceneComponent::TextContent(text) => {
                    buf.push(ComponentTag::TextContent as u8);
                    let len = text.len().min(256) as u16;
                    buf.extend_from_slice(&len.to_le_bytes());
                    buf.extend_from_slice(&text[..len as usize]);
                }
                SceneComponent::FontSize(s) => {
                    buf.push(ComponentTag::FontSize as u8);
                    buf.extend_from_slice(&s.to_le_bytes());
                }
                SceneComponent::Focusable => {
                    buf.push(ComponentTag::Focusable as u8);
                }
                SceneComponent::Window => {
                    buf.push(ComponentTag::Window as u8);
                }
                SceneComponent::WindowTitle(title) => {
                    buf.push(ComponentTag::WindowTitle as u8);
                    let len = title.len().min(64) as u8;
                    buf.push(len);
                    buf.extend_from_slice(&title[..len as usize]);
                }
            }
        }
    }

    buf
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use alloc::vec;

    use super::*;
    use crate::scene::format::{SCENE_MAGIC, SCENE_VERSION};

    #[test]
    fn serialize_empty_scene() {
        let scene = SceneData::new();
        let bytes = serialize_scene(&scene);
        // Header only: magic(4) + version(4) + count(4) = 12
        assert_eq!(bytes.len(), 12);
        assert_eq!(&bytes[0..4], &SCENE_MAGIC);
        assert_eq!(
            u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            SCENE_VERSION
        );
        assert_eq!(
            u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            0
        );
    }

    #[test]
    fn serialize_single_node_no_components() {
        let scene = SceneData {
            nodes: vec![SceneNode {
                components: vec![],
                parent_index: None,
                child_indices: vec![],
            }],
        };
        let bytes = serialize_scene(&scene);
        // Header(12) + comp_count(1) + parent(4) + child_count(1) = 18
        assert_eq!(bytes.len(), 18);
        // entity count = 1
        assert_eq!(
            u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            1
        );
        // comp_count = 0
        assert_eq!(bytes[12], 0);
        // parent = NO_PARENT
        assert_eq!(
            u32::from_le_bytes([bytes[13], bytes[14], bytes[15], bytes[16]]),
            NO_PARENT
        );
        // child_count = 0
        assert_eq!(bytes[17], 0);
    }

    #[test]
    fn serialize_single_node_with_position() {
        let scene = SceneData {
            nodes: vec![SceneNode {
                components: vec![SceneComponent::Position(10.0, 20.0)],
                parent_index: None,
                child_indices: vec![],
            }],
        };
        let bytes = serialize_scene(&scene);
        // Header(12) + comp_count(1) + parent(4) + child_count(1) + tag(1) + 2*f32(8) = 27
        assert_eq!(bytes.len(), 27);
        // comp_count = 1
        assert_eq!(bytes[12], 1);
        // tag = Position(0)
        assert_eq!(bytes[18], 0);
        let x = f32::from_le_bytes([bytes[19], bytes[20], bytes[21], bytes[22]]);
        let y = f32::from_le_bytes([bytes[23], bytes[24], bytes[25], bytes[26]]);
        assert_eq!(x, 10.0);
        assert_eq!(y, 20.0);
    }

    #[test]
    fn serialize_with_hierarchy() {
        let scene = SceneData {
            nodes: vec![
                SceneNode {
                    components: vec![SceneComponent::Window],
                    parent_index: None,
                    child_indices: vec![1],
                },
                SceneNode {
                    components: vec![SceneComponent::Position(5.0, 5.0)],
                    parent_index: Some(0),
                    child_indices: vec![],
                },
            ],
        };
        let bytes = serialize_scene(&scene);
        // entity count = 2
        assert_eq!(
            u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            2
        );
        // First node: comp_count(1) + parent(4) + child_count(1) + child_idx(4) + tag(1) = 11
        // parent = NO_PARENT
        assert_eq!(
            u32::from_le_bytes([bytes[13], bytes[14], bytes[15], bytes[16]]),
            NO_PARENT
        );
        // child_count = 1
        assert_eq!(bytes[17], 1);
        // child index = 1
        assert_eq!(
            u32::from_le_bytes([bytes[18], bytes[19], bytes[20], bytes[21]]),
            1
        );
    }

    #[test]
    fn serialize_text_content() {
        let scene = SceneData {
            nodes: vec![SceneNode {
                components: vec![SceneComponent::TextContent(b"Hello".to_vec())],
                parent_index: None,
                child_indices: vec![],
            }],
        };
        let bytes = serialize_scene(&scene);
        // Find tag byte after node header
        // Header(12) + comp_count(1) + parent(4) + child_count(1) = offset 18
        assert_eq!(bytes[18], ComponentTag::TextContent as u8);
        // length (u16 LE)
        let len = u16::from_le_bytes([bytes[19], bytes[20]]);
        assert_eq!(len, 5);
        assert_eq!(&bytes[21..26], b"Hello");
    }

    #[test]
    fn serialize_window_title() {
        let scene = SceneData {
            nodes: vec![SceneNode {
                components: vec![SceneComponent::WindowTitle(b"MyWin".to_vec())],
                parent_index: None,
                child_indices: vec![],
            }],
        };
        let bytes = serialize_scene(&scene);
        assert_eq!(bytes[18], ComponentTag::WindowTitle as u8);
        // length (u8)
        assert_eq!(bytes[19], 5);
        assert_eq!(&bytes[20..25], b"MyWin");
    }
}
