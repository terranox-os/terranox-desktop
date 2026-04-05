//! GPU render backend abstraction trait.
//!
//! Defines `RenderBackend`, a common interface for pixel-buffer
//! creation, mapping, and presentation across software, DRM, and
//! future GPU syscall backends.

/// Opaque identifier for a pixel buffer managed by a [`RenderBackend`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferId(pub u32);

/// Errors returned by [`RenderBackend`] operations.
#[derive(Debug)]
pub enum RenderError {
    /// No free buffer slots available.
    BufferFull,
    /// The supplied [`BufferId`] does not refer to a live buffer.
    InvalidBuffer,
    /// The operation is not implemented by this backend.
    NotSupported,
    /// A platform-specific error (negative errno or DRM error code).
    PlatformError(i32),
}

/// Abstraction over different rendering backends.
///
/// Implementations manage pixel buffers and display presentation.
/// The trait is object-safe so it can be used as `Box<dyn RenderBackend>`.
pub trait RenderBackend {
    /// Create a pixel buffer with the given dimensions.
    /// Returns a handle for later operations.
    fn create_buffer(&mut self, width: u32, height: u32) -> Result<BufferId, RenderError>;

    /// Destroy a pixel buffer, freeing its resources.
    fn destroy_buffer(&mut self, id: BufferId) -> Result<(), RenderError>;

    /// Get mutable access to buffer pixels (ARGB8888 format).
    fn get_pixels(&mut self, id: BufferId) -> Option<&mut [u32]>;

    /// Get buffer dimensions as `(width, height)`.
    fn buffer_size(&self, id: BufferId) -> Option<(u32, u32)>;

    /// Submit the buffer to the display (page flip / present).
    fn present(&mut self, id: BufferId) -> Result<(), RenderError>;

    /// Display width in pixels.
    fn display_width(&self) -> u32;

    /// Display height in pixels.
    fn display_height(&self) -> u32;
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use super::*;
    use alloc::boxed::Box;

    // Minimal backend to prove object safety.
    struct DummyBackend;

    impl RenderBackend for DummyBackend {
        fn create_buffer(&mut self, _w: u32, _h: u32) -> Result<BufferId, RenderError> {
            Err(RenderError::NotSupported)
        }
        fn destroy_buffer(&mut self, _id: BufferId) -> Result<(), RenderError> {
            Err(RenderError::NotSupported)
        }
        fn get_pixels(&mut self, _id: BufferId) -> Option<&mut [u32]> {
            None
        }
        fn buffer_size(&self, _id: BufferId) -> Option<(u32, u32)> {
            None
        }
        fn present(&mut self, _id: BufferId) -> Result<(), RenderError> {
            Err(RenderError::NotSupported)
        }
        fn display_width(&self) -> u32 {
            0
        }
        fn display_height(&self) -> u32 {
            0
        }
    }

    #[test]
    fn trait_is_object_safe() {
        let backend: Box<dyn RenderBackend> = Box::new(DummyBackend);
        assert_eq!(backend.display_width(), 0);
        assert_eq!(backend.display_height(), 0);
    }

    #[test]
    fn buffer_id_equality() {
        let a = BufferId(1);
        let b = BufferId(1);
        let c = BufferId(2);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn buffer_id_copy() {
        let a = BufferId(42);
        let b = a;
        assert_eq!(a, b);
    }
}
