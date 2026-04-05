/// Binary scene format magic bytes: "TRXS".
pub const SCENE_MAGIC: [u8; 4] = *b"TRXS";

/// Binary scene format version.
pub const SCENE_VERSION: u32 = 1;

/// Sentinel value indicating an entity has no parent.
pub const NO_PARENT: u32 = 0xFFFF_FFFF;

/// Tag byte identifying a component type in the binary scene format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ComponentTag {
    Position = 0,
    Size = 1,
    ZIndex = 2,
    BackgroundColor = 3,
    BorderColor = 4,
    BorderWidth = 5,
    BorderRadius = 6,
    Opacity = 7,
    Visible = 8,
    FlexboxLayout = 9,
    TextContent = 10,
    FontSize = 11,
    TextColor = 12,
    Focusable = 13,
    Window = 14,
    WindowTitle = 15,
}

impl ComponentTag {
    /// Convert a raw byte to a `ComponentTag`, returning `None` for unknown values.
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Position),
            1 => Some(Self::Size),
            2 => Some(Self::ZIndex),
            3 => Some(Self::BackgroundColor),
            4 => Some(Self::BorderColor),
            5 => Some(Self::BorderWidth),
            6 => Some(Self::BorderRadius),
            7 => Some(Self::Opacity),
            8 => Some(Self::Visible),
            9 => Some(Self::FlexboxLayout),
            10 => Some(Self::TextContent),
            11 => Some(Self::FontSize),
            12 => Some(Self::TextColor),
            13 => Some(Self::Focusable),
            14 => Some(Self::Window),
            15 => Some(Self::WindowTitle),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn magic_bytes() {
        assert_eq!(&SCENE_MAGIC, b"TRXS");
    }

    #[test]
    fn version_is_one() {
        assert_eq!(SCENE_VERSION, 1);
    }

    #[test]
    fn no_parent_sentinel() {
        assert_eq!(NO_PARENT, 0xFFFF_FFFF);
    }

    #[test]
    fn from_u8_all_variants() {
        assert_eq!(ComponentTag::from_u8(0), Some(ComponentTag::Position));
        assert_eq!(ComponentTag::from_u8(1), Some(ComponentTag::Size));
        assert_eq!(ComponentTag::from_u8(2), Some(ComponentTag::ZIndex));
        assert_eq!(ComponentTag::from_u8(3), Some(ComponentTag::BackgroundColor));
        assert_eq!(ComponentTag::from_u8(4), Some(ComponentTag::BorderColor));
        assert_eq!(ComponentTag::from_u8(5), Some(ComponentTag::BorderWidth));
        assert_eq!(ComponentTag::from_u8(6), Some(ComponentTag::BorderRadius));
        assert_eq!(ComponentTag::from_u8(7), Some(ComponentTag::Opacity));
        assert_eq!(ComponentTag::from_u8(8), Some(ComponentTag::Visible));
        assert_eq!(ComponentTag::from_u8(9), Some(ComponentTag::FlexboxLayout));
        assert_eq!(ComponentTag::from_u8(10), Some(ComponentTag::TextContent));
        assert_eq!(ComponentTag::from_u8(11), Some(ComponentTag::FontSize));
        assert_eq!(ComponentTag::from_u8(12), Some(ComponentTag::TextColor));
        assert_eq!(ComponentTag::from_u8(13), Some(ComponentTag::Focusable));
        assert_eq!(ComponentTag::from_u8(14), Some(ComponentTag::Window));
        assert_eq!(ComponentTag::from_u8(15), Some(ComponentTag::WindowTitle));
    }

    #[test]
    fn from_u8_unknown_returns_none() {
        assert_eq!(ComponentTag::from_u8(16), None);
        assert_eq!(ComponentTag::from_u8(255), None);
    }

    #[test]
    fn round_trip_all_tags() {
        for v in 0..=15u8 {
            let tag = ComponentTag::from_u8(v).unwrap();
            assert_eq!(tag as u8, v);
        }
    }
}
