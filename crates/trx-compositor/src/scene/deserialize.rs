use alloc::vec::Vec;

use crate::scene::format::{ComponentTag, NO_PARENT, SCENE_MAGIC, SCENE_VERSION};
use crate::scene::serialize::{SceneComponent, SceneData, SceneNode};

/// Errors that can occur when parsing a binary scene.
#[derive(Debug)]
pub enum SceneError {
    /// Magic bytes do not match "TRXS".
    BadMagic,
    /// Unsupported format version.
    BadVersion(u32),
    /// Input data ended before a complete scene could be read.
    UnexpectedEof,
    /// Unknown component tag byte encountered.
    BadComponentTag(u8),
}

/// Helper to read exactly `n` bytes from `data` at `pos`, advancing `pos`.
/// Returns `Err(UnexpectedEof)` if not enough data is available.
macro_rules! need {
    ($data:expr, $pos:expr, $n:expr) => {
        if $pos + $n > $data.len() {
            return Err(SceneError::UnexpectedEof);
        }
    };
}

/// Read a `u32` from little-endian bytes.
fn read_u32(data: &[u8], pos: &mut usize) -> Result<u32, SceneError> {
    need!(data, *pos, 4);
    let v = u32::from_le_bytes([data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3]]);
    *pos += 4;
    Ok(v)
}

/// Read an `i32` from little-endian bytes.
fn read_i32(data: &[u8], pos: &mut usize) -> Result<i32, SceneError> {
    need!(data, *pos, 4);
    let v = i32::from_le_bytes([data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3]]);
    *pos += 4;
    Ok(v)
}

/// Read an `f32` from little-endian bytes.
fn read_f32(data: &[u8], pos: &mut usize) -> Result<f32, SceneError> {
    need!(data, *pos, 4);
    let v = f32::from_le_bytes([data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3]]);
    *pos += 4;
    Ok(v)
}

/// Read a `u16` from little-endian bytes.
fn read_u16(data: &[u8], pos: &mut usize) -> Result<u16, SceneError> {
    need!(data, *pos, 2);
    let v = u16::from_le_bytes([data[*pos], data[*pos + 1]]);
    *pos += 2;
    Ok(v)
}

/// Read a single byte.
fn read_u8(data: &[u8], pos: &mut usize) -> Result<u8, SceneError> {
    need!(data, *pos, 1);
    let v = data[*pos];
    *pos += 1;
    Ok(v)
}

/// Read a 4-byte color (r, g, b, a).
fn read_color(data: &[u8], pos: &mut usize) -> Result<(u8, u8, u8, u8), SceneError> {
    need!(data, *pos, 4);
    let r = data[*pos];
    let g = data[*pos + 1];
    let b = data[*pos + 2];
    let a = data[*pos + 3];
    *pos += 4;
    Ok((r, g, b, a))
}

/// Deserialize a TRXS v1 binary scene into a `SceneData`.
pub fn deserialize_scene(data: &[u8]) -> Result<SceneData, SceneError> {
    let mut pos = 0;

    // Header
    need!(data, pos, 12);
    if data[0..4] != SCENE_MAGIC {
        return Err(SceneError::BadMagic);
    }
    pos = 4;
    let version = read_u32(data, &mut pos)?;
    if version != SCENE_VERSION {
        return Err(SceneError::BadVersion(version));
    }
    let entity_count = read_u32(data, &mut pos)? as usize;

    let mut nodes = Vec::with_capacity(entity_count);

    for _ in 0..entity_count {
        // Component count
        let comp_count = read_u8(data, &mut pos)? as usize;

        // Parent index
        let parent_raw = read_u32(data, &mut pos)?;
        let parent_index = if parent_raw == NO_PARENT {
            None
        } else {
            Some(parent_raw)
        };

        // Child count + child indices
        let child_count = read_u8(data, &mut pos)? as usize;
        let mut child_indices = Vec::with_capacity(child_count);
        for _ in 0..child_count {
            child_indices.push(read_u32(data, &mut pos)?);
        }

        // Components
        let mut components = Vec::with_capacity(comp_count);
        for _ in 0..comp_count {
            let tag_byte = read_u8(data, &mut pos)?;
            let tag =
                ComponentTag::from_u8(tag_byte).ok_or(SceneError::BadComponentTag(tag_byte))?;

            let comp = match tag {
                ComponentTag::Position => {
                    let x = read_f32(data, &mut pos)?;
                    let y = read_f32(data, &mut pos)?;
                    SceneComponent::Position(x, y)
                }
                ComponentTag::Size => {
                    let w = read_f32(data, &mut pos)?;
                    let h = read_f32(data, &mut pos)?;
                    SceneComponent::Size(w, h)
                }
                ComponentTag::ZIndex => {
                    let z = read_i32(data, &mut pos)?;
                    SceneComponent::ZIndex(z)
                }
                ComponentTag::BackgroundColor => {
                    let (r, g, b, a) = read_color(data, &mut pos)?;
                    SceneComponent::BackgroundColor(r, g, b, a)
                }
                ComponentTag::BorderColor => {
                    let (r, g, b, a) = read_color(data, &mut pos)?;
                    SceneComponent::BorderColor(r, g, b, a)
                }
                ComponentTag::TextColor => {
                    let (r, g, b, a) = read_color(data, &mut pos)?;
                    SceneComponent::TextColor(r, g, b, a)
                }
                ComponentTag::BorderWidth => {
                    let v = read_f32(data, &mut pos)?;
                    SceneComponent::BorderWidth(v)
                }
                ComponentTag::BorderRadius => {
                    let v = read_f32(data, &mut pos)?;
                    SceneComponent::BorderRadius(v)
                }
                ComponentTag::Opacity => {
                    let v = read_f32(data, &mut pos)?;
                    SceneComponent::Opacity(v)
                }
                ComponentTag::Visible => {
                    let v = read_u8(data, &mut pos)?;
                    SceneComponent::Visible(v != 0)
                }
                ComponentTag::FlexboxLayout => {
                    let direction = read_u8(data, &mut pos)?;
                    let align = read_u8(data, &mut pos)?;
                    let justify = read_u8(data, &mut pos)?;
                    let gap = read_f32(data, &mut pos)?;
                    let mut padding = [0.0f32; 4];
                    for p in &mut padding {
                        *p = read_f32(data, &mut pos)?;
                    }
                    let mut margin = [0.0f32; 4];
                    for m in &mut margin {
                        *m = read_f32(data, &mut pos)?;
                    }
                    SceneComponent::FlexboxLayout {
                        direction,
                        align,
                        justify,
                        gap,
                        padding,
                        margin,
                    }
                }
                ComponentTag::TextContent => {
                    let len = read_u16(data, &mut pos)? as usize;
                    need!(data, pos, len);
                    let text = data[pos..pos + len].to_vec();
                    pos += len;
                    SceneComponent::TextContent(text)
                }
                ComponentTag::FontSize => {
                    let s = read_u16(data, &mut pos)?;
                    SceneComponent::FontSize(s)
                }
                ComponentTag::Focusable => SceneComponent::Focusable,
                ComponentTag::Window => SceneComponent::Window,
                ComponentTag::WindowTitle => {
                    let len = read_u8(data, &mut pos)? as usize;
                    need!(data, pos, len);
                    let title = data[pos..pos + len].to_vec();
                    pos += len;
                    SceneComponent::WindowTitle(title)
                }
            };
            components.push(comp);
        }

        nodes.push(SceneNode {
            components,
            parent_index,
            child_indices,
        });
    }

    Ok(SceneData { nodes })
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use alloc::vec;
    use alloc::vec::Vec;

    use super::*;
    use crate::scene::serialize::{serialize_scene, SceneComponent, SceneData, SceneNode};

    #[test]
    fn round_trip_empty_scene() {
        let scene = SceneData::new();
        let bytes = serialize_scene(&scene);
        let result = deserialize_scene(&bytes).unwrap();
        assert_eq!(result.nodes.len(), 0);
    }

    #[test]
    fn round_trip_single_node_all_components() {
        let scene = SceneData {
            nodes: vec![SceneNode {
                components: vec![
                    SceneComponent::Position(1.5, 2.5),
                    SceneComponent::Size(100.0, 200.0),
                    SceneComponent::ZIndex(-3),
                    SceneComponent::BackgroundColor(255, 0, 128, 200),
                    SceneComponent::BorderColor(10, 20, 30, 40),
                    SceneComponent::BorderWidth(2.0),
                    SceneComponent::BorderRadius(8.0),
                    SceneComponent::Opacity(0.75),
                    SceneComponent::Visible(true),
                    SceneComponent::FlexboxLayout {
                        direction: 1,
                        align: 2,
                        justify: 3,
                        gap: 4.0,
                        padding: [1.0, 2.0, 3.0, 4.0],
                        margin: [5.0, 6.0, 7.0, 8.0],
                    },
                    SceneComponent::TextContent(b"Hello World".to_vec()),
                    SceneComponent::FontSize(16),
                    SceneComponent::TextColor(50, 60, 70, 255),
                    SceneComponent::Focusable,
                    SceneComponent::Window,
                    SceneComponent::WindowTitle(b"Test".to_vec()),
                ],
                parent_index: None,
                child_indices: vec![],
            }],
        };

        let bytes = serialize_scene(&scene);
        let result = deserialize_scene(&bytes).unwrap();
        assert_eq!(result.nodes.len(), 1);

        let node = &result.nodes[0];
        assert_eq!(node.components.len(), 16);
        assert!(node.parent_index.is_none());
        assert!(node.child_indices.is_empty());

        // Verify each component
        match &node.components[0] {
            SceneComponent::Position(x, y) => {
                assert_eq!(*x, 1.5);
                assert_eq!(*y, 2.5);
            }
            _ => panic!("expected Position"),
        }
        match &node.components[1] {
            SceneComponent::Size(w, h) => {
                assert_eq!(*w, 100.0);
                assert_eq!(*h, 200.0);
            }
            _ => panic!("expected Size"),
        }
        match &node.components[2] {
            SceneComponent::ZIndex(z) => assert_eq!(*z, -3),
            _ => panic!("expected ZIndex"),
        }
        match &node.components[3] {
            SceneComponent::BackgroundColor(r, g, b, a) => {
                assert_eq!((*r, *g, *b, *a), (255, 0, 128, 200));
            }
            _ => panic!("expected BackgroundColor"),
        }
        match &node.components[4] {
            SceneComponent::BorderColor(r, g, b, a) => {
                assert_eq!((*r, *g, *b, *a), (10, 20, 30, 40));
            }
            _ => panic!("expected BorderColor"),
        }
        match &node.components[5] {
            SceneComponent::BorderWidth(v) => assert_eq!(*v, 2.0),
            _ => panic!("expected BorderWidth"),
        }
        match &node.components[6] {
            SceneComponent::BorderRadius(v) => assert_eq!(*v, 8.0),
            _ => panic!("expected BorderRadius"),
        }
        match &node.components[7] {
            SceneComponent::Opacity(v) => assert_eq!(*v, 0.75),
            _ => panic!("expected Opacity"),
        }
        match &node.components[8] {
            SceneComponent::Visible(v) => assert!(*v),
            _ => panic!("expected Visible"),
        }
        match &node.components[9] {
            SceneComponent::FlexboxLayout {
                direction,
                align,
                justify,
                gap,
                padding,
                margin,
            } => {
                assert_eq!(*direction, 1);
                assert_eq!(*align, 2);
                assert_eq!(*justify, 3);
                assert_eq!(*gap, 4.0);
                assert_eq!(*padding, [1.0, 2.0, 3.0, 4.0]);
                assert_eq!(*margin, [5.0, 6.0, 7.0, 8.0]);
            }
            _ => panic!("expected FlexboxLayout"),
        }
        match &node.components[10] {
            SceneComponent::TextContent(text) => assert_eq!(text.as_slice(), b"Hello World"),
            _ => panic!("expected TextContent"),
        }
        match &node.components[11] {
            SceneComponent::FontSize(s) => assert_eq!(*s, 16),
            _ => panic!("expected FontSize"),
        }
        match &node.components[12] {
            SceneComponent::TextColor(r, g, b, a) => {
                assert_eq!((*r, *g, *b, *a), (50, 60, 70, 255));
            }
            _ => panic!("expected TextColor"),
        }
        assert!(matches!(&node.components[13], SceneComponent::Focusable));
        assert!(matches!(&node.components[14], SceneComponent::Window));
        match &node.components[15] {
            SceneComponent::WindowTitle(title) => assert_eq!(title.as_slice(), b"Test"),
            _ => panic!("expected WindowTitle"),
        }
    }

    #[test]
    fn round_trip_with_hierarchy() {
        let scene = SceneData {
            nodes: vec![
                SceneNode {
                    components: vec![SceneComponent::Window],
                    parent_index: None,
                    child_indices: vec![1, 2],
                },
                SceneNode {
                    components: vec![SceneComponent::Position(10.0, 20.0)],
                    parent_index: Some(0),
                    child_indices: vec![],
                },
                SceneNode {
                    components: vec![SceneComponent::Position(30.0, 40.0)],
                    parent_index: Some(0),
                    child_indices: vec![],
                },
            ],
        };

        let bytes = serialize_scene(&scene);
        let result = deserialize_scene(&bytes).unwrap();
        assert_eq!(result.nodes.len(), 3);

        assert!(result.nodes[0].parent_index.is_none());
        assert_eq!(result.nodes[0].child_indices, vec![1, 2]);
        assert_eq!(result.nodes[1].parent_index, Some(0));
        assert!(result.nodes[1].child_indices.is_empty());
        assert_eq!(result.nodes[2].parent_index, Some(0));
        assert!(result.nodes[2].child_indices.is_empty());
    }

    #[test]
    fn bad_magic() {
        let data = b"BADSxxxxxxxx";
        let err = deserialize_scene(data).unwrap_err();
        assert!(matches!(err, SceneError::BadMagic));
    }

    #[test]
    fn bad_version() {
        let mut data = Vec::new();
        data.extend_from_slice(b"TRXS");
        data.extend_from_slice(&99u32.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        let err = deserialize_scene(&data).unwrap_err();
        assert!(matches!(err, SceneError::BadVersion(99)));
    }

    #[test]
    fn truncated_header() {
        let data = b"TRXS\x01";
        let err = deserialize_scene(data).unwrap_err();
        assert!(matches!(err, SceneError::UnexpectedEof));
    }

    #[test]
    fn truncated_entity() {
        let mut data = Vec::new();
        data.extend_from_slice(b"TRXS");
        data.extend_from_slice(&1u32.to_le_bytes()); // version
        data.extend_from_slice(&1u32.to_le_bytes()); // 1 entity
        // No entity data follows
        let err = deserialize_scene(&data).unwrap_err();
        assert!(matches!(err, SceneError::UnexpectedEof));
    }

    #[test]
    fn unknown_tag() {
        let mut data = Vec::new();
        data.extend_from_slice(b"TRXS");
        data.extend_from_slice(&1u32.to_le_bytes()); // version
        data.extend_from_slice(&1u32.to_le_bytes()); // 1 entity
        data.push(1); // 1 component
        data.extend_from_slice(&NO_PARENT.to_le_bytes());
        data.push(0); // 0 children
        data.push(0xFE); // bad tag
        let err = deserialize_scene(&data).unwrap_err();
        assert!(matches!(err, SceneError::BadComponentTag(0xFE)));
    }

    #[test]
    fn round_trip_visible_false() {
        let scene = SceneData {
            nodes: vec![SceneNode {
                components: vec![SceneComponent::Visible(false)],
                parent_index: None,
                child_indices: vec![],
            }],
        };
        let bytes = serialize_scene(&scene);
        let result = deserialize_scene(&bytes).unwrap();
        match &result.nodes[0].components[0] {
            SceneComponent::Visible(v) => assert!(!v),
            _ => panic!("expected Visible"),
        }
    }
}
