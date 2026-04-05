//! GPU syscall backend stub.
//!
//! Placeholder for future TRX GPU syscall backend.
//! Returns [`RenderError::NotSupported`] for all operations until
//! the kernel implements GPU syscalls (0x0170-0x0179).

use crate::render::backend::{BufferId, RenderBackend, RenderError};

/// Stub GPU backend. All operations return [`RenderError::NotSupported`].
pub struct GpuBackend;

impl GpuBackend {
    /// Create a new GPU backend stub.
    pub fn new() -> Self {
        Self
    }
}

impl Default for GpuBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderBackend for GpuBackend {
    fn create_buffer(&mut self, _width: u32, _height: u32) -> Result<BufferId, RenderError> {
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

#[cfg(test)]
mod tests {
    extern crate alloc;
    use super::*;
    use crate::render::backend::BufferId;

    #[test]
    fn gpu_create_buffer_not_supported() {
        let mut gpu = GpuBackend::new();
        assert!(gpu.create_buffer(100, 100).is_err());
    }

    #[test]
    fn gpu_destroy_buffer_not_supported() {
        let mut gpu = GpuBackend::new();
        assert!(gpu.destroy_buffer(BufferId(0)).is_err());
    }

    #[test]
    fn gpu_get_pixels_returns_none() {
        let mut gpu = GpuBackend::new();
        assert!(gpu.get_pixels(BufferId(0)).is_none());
    }

    #[test]
    fn gpu_buffer_size_returns_none() {
        let gpu = GpuBackend::new();
        assert!(gpu.buffer_size(BufferId(0)).is_none());
    }

    #[test]
    fn gpu_present_not_supported() {
        let mut gpu = GpuBackend::new();
        assert!(gpu.present(BufferId(0)).is_err());
    }

    #[test]
    fn gpu_display_dimensions_zero() {
        let gpu = GpuBackend::new();
        assert_eq!(gpu.display_width(), 0);
        assert_eq!(gpu.display_height(), 0);
    }

    #[test]
    fn gpu_default() {
        let gpu = GpuBackend::default();
        assert_eq!(gpu.display_width(), 0);
    }
}
