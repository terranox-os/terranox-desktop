//! TRX GPU syscall backend.
//!
//! Delegates buffer allocation, mapping, and presentation to a
//! [`PlatformBackend`] implementation backed by TRX display syscalls
//! (`sys_trx_buffer_create`, `sys_trx_surface_create`,
//! `sys_trx_compositor_present`, etc.).
//!
//! Compared to the [`DrmBackend`](super::DrmBackend), the GPU backend
//! supports more concurrent buffers (8 vs 4) and tracks a compositor
//! handle for TRX compositor presentation. A future Phase 3+ may add
//! direct GPU command submission via `sys_trx_gpu_submit`.

use crate::platform::PlatformBackend;
use crate::render::backend::{BufferId, RenderBackend, RenderError};

/// Maximum number of pixel buffers the GPU backend can manage simultaneously.
const MAX_GPU_BUFFERS: usize = 8;

/// Internal representation of a single GPU-managed pixel buffer.
struct GpuBuffer {
    /// Platform handle returned by [`PlatformBackend::create_buffer`].
    handle: i64,
    /// Mapped pixel pointer from [`PlatformBackend::map_buffer`].
    pixels: *mut u32,
    /// Stride in pixels (currently unused; reserved for non-contiguous layouts).
    _stride: u32,
    /// Buffer width in pixels.
    width: u32,
    /// Buffer height in pixels.
    height: u32,
    /// Total number of u32 pixel elements (`width * height`).
    len: usize,
}

/// A rendering backend backed by TRX display/compositor syscalls.
///
/// Uses a [`PlatformBackend`] for buffer creation, mapping, and
/// presentation. Manages up to [`MAX_GPU_BUFFERS`] concurrent pixel
/// buffers and tracks a surface handle and compositor handle for
/// display output.
///
/// # Examples
///
/// ```ignore
/// use trx_compositor::platform::MockPlatform;
/// use trx_compositor::render::gpu::GpuBackend;
/// use trx_compositor::render::backend::RenderBackend;
///
/// let platform = MockPlatform::new();
/// let mut gpu = GpuBackend::new(platform, 1920, 1080).unwrap();
/// let buf = gpu.create_buffer(800, 600).unwrap();
/// gpu.present(buf).unwrap();
/// ```
pub struct GpuBackend<P: PlatformBackend> {
    platform: P,
    buffers: [Option<GpuBuffer>; MAX_GPU_BUFFERS],
    /// Surface handle from the platform layer.
    surface_handle: i64,
    /// Compositor handle for TRX compositor_present. Created lazily
    /// on first present call (value of -1 means not yet created).
    compositor_handle: i64,
    display_width: u32,
    display_height: u32,
    /// Number of successful present calls.
    present_count: u32,
}

impl<P: PlatformBackend> GpuBackend<P> {
    /// Create a new GPU backend with the given display dimensions.
    ///
    /// Calls `platform.create_surface()` immediately. Returns an error
    /// if the platform returns a negative handle.
    pub fn new(mut platform: P, width: u32, height: u32) -> Result<Self, RenderError> {
        let surface_handle = platform.create_surface(width, height);
        if surface_handle < 0 {
            return Err(RenderError::PlatformError(surface_handle as i32));
        }
        const NONE: Option<GpuBuffer> = None;
        Ok(Self {
            platform,
            buffers: [NONE; MAX_GPU_BUFFERS],
            surface_handle,
            compositor_handle: -1,
            display_width: width,
            display_height: height,
            present_count: 0,
        })
    }

    /// Access the underlying platform backend.
    pub fn platform(&self) -> &P {
        &self.platform
    }

    /// Number of successful [`RenderBackend::present`] calls.
    pub fn present_count(&self) -> u32 {
        self.present_count
    }

    /// Compositor handle for TRX `compositor_present`.
    ///
    /// Returns `-1` if not yet created (reserved for Phase 3+ GPU
    /// command submission).
    pub fn compositor_handle(&self) -> i64 {
        self.compositor_handle
    }
}

impl<P: PlatformBackend> RenderBackend for GpuBackend<P> {
    fn create_buffer(&mut self, width: u32, height: u32) -> Result<BufferId, RenderError> {
        let handle = self.platform.create_buffer(width, height);
        if handle < 0 {
            return Err(RenderError::PlatformError(handle as i32));
        }
        let (pixels, stride) = self.platform.map_buffer(handle);
        let len = (width as usize) * (height as usize);

        for (i, slot) in self.buffers.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(GpuBuffer {
                    handle,
                    pixels,
                    _stride: stride,
                    width,
                    height,
                    len,
                });
                return Ok(BufferId(i as u32));
            }
        }
        // No free slot — clean up the platform buffer.
        self.platform.unmap_buffer(handle);
        Err(RenderError::BufferFull)
    }

    fn destroy_buffer(&mut self, id: BufferId) -> Result<(), RenderError> {
        let idx = id.0 as usize;
        if idx >= MAX_GPU_BUFFERS {
            return Err(RenderError::InvalidBuffer);
        }
        if let Some(buf) = self.buffers[idx].take() {
            self.platform.unmap_buffer(buf.handle);
            Ok(())
        } else {
            Err(RenderError::InvalidBuffer)
        }
    }

    fn get_pixels(&mut self, id: BufferId) -> Option<&mut [u32]> {
        let idx = id.0 as usize;
        if idx >= MAX_GPU_BUFFERS {
            return None;
        }
        self.buffers[idx].as_ref().and_then(|buf| {
            if buf.pixels.is_null() {
                None
            } else {
                // SAFETY: The platform backend mapped this buffer and guarantees
                // the pointer is valid for `len` u32 elements while the buffer
                // is alive. We hold a mutable borrow on self, preventing
                // concurrent access.
                Some(unsafe { core::slice::from_raw_parts_mut(buf.pixels, buf.len) })
            }
        })
    }

    fn buffer_size(&self, id: BufferId) -> Option<(u32, u32)> {
        let idx = id.0 as usize;
        if idx >= MAX_GPU_BUFFERS {
            return None;
        }
        self.buffers[idx].as_ref().map(|b| (b.width, b.height))
    }

    fn present(&mut self, id: BufferId) -> Result<(), RenderError> {
        let idx = id.0 as usize;
        if idx >= MAX_GPU_BUFFERS {
            return Err(RenderError::InvalidBuffer);
        }
        if let Some(buf) = &self.buffers[idx] {
            let buf_handle = buf.handle;
            self.platform.present(self.surface_handle, buf_handle);
            self.present_count += 1;
            Ok(())
        } else {
            Err(RenderError::InvalidBuffer)
        }
    }

    fn display_width(&self) -> u32 {
        self.display_width
    }

    fn display_height(&self) -> u32 {
        self.display_height
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use super::*;
    use crate::platform::MockPlatform;
    use crate::render::backend::{BufferId, RenderBackend};

    #[test]
    fn gpu_backend_creates_surface() {
        let mock = MockPlatform::new();
        let backend = GpuBackend::new(mock, 1920, 1080).unwrap();
        assert_eq!(backend.display_width(), 1920);
        assert_eq!(backend.display_height(), 1080);
        // MockPlatform starts at handle 1; surface is handle 1.
        assert_eq!(backend.surface_handle, 1);
    }

    #[test]
    fn gpu_backend_compositor_handle_initially_unset() {
        let mock = MockPlatform::new();
        let backend = GpuBackend::new(mock, 800, 600).unwrap();
        assert_eq!(backend.compositor_handle, -1);
    }

    #[test]
    fn gpu_backend_create_buffer() {
        let mock = MockPlatform::new();
        let mut backend = GpuBackend::new(mock, 800, 600).unwrap();
        let id = backend.create_buffer(100, 50).unwrap();
        assert_eq!(id, BufferId(0));
        assert_eq!(backend.buffer_size(id), Some((100, 50)));
    }

    #[test]
    fn gpu_backend_create_multiple_buffers() {
        let mock = MockPlatform::new();
        let mut backend = GpuBackend::new(mock, 800, 600).unwrap();
        let id0 = backend.create_buffer(100, 50).unwrap();
        let id1 = backend.create_buffer(200, 100).unwrap();
        let id2 = backend.create_buffer(320, 240).unwrap();
        assert_eq!(id0, BufferId(0));
        assert_eq!(id1, BufferId(1));
        assert_eq!(id2, BufferId(2));
        assert_eq!(backend.buffer_size(id0), Some((100, 50)));
        assert_eq!(backend.buffer_size(id1), Some((200, 100)));
        assert_eq!(backend.buffer_size(id2), Some((320, 240)));
    }

    #[test]
    fn gpu_backend_get_pixels_null_returns_none() {
        // MockPlatform returns null from map_buffer.
        let mock = MockPlatform::new();
        let mut backend = GpuBackend::new(mock, 800, 600).unwrap();
        let id = backend.create_buffer(10, 10).unwrap();
        assert!(backend.get_pixels(id).is_none());
    }

    #[test]
    fn gpu_backend_get_pixels_invalid_returns_none() {
        let mock = MockPlatform::new();
        let mut backend = GpuBackend::new(mock, 800, 600).unwrap();
        assert!(backend.get_pixels(BufferId(0)).is_none());
        assert!(backend.get_pixels(BufferId(99)).is_none());
    }

    #[test]
    fn gpu_backend_present_delegates_to_platform() {
        let mock = MockPlatform::new();
        let mut backend = GpuBackend::new(mock, 800, 600).unwrap();
        let id = backend.create_buffer(10, 10).unwrap();

        backend.present(id).unwrap();
        assert_eq!(backend.platform().presented_count(), 1);
        assert_eq!(backend.present_count(), 1);

        backend.present(id).unwrap();
        assert_eq!(backend.platform().presented_count(), 2);
        assert_eq!(backend.present_count(), 2);
    }

    #[test]
    fn gpu_backend_present_invalid_returns_error() {
        let mock = MockPlatform::new();
        let mut backend = GpuBackend::new(mock, 800, 600).unwrap();
        assert!(backend.present(BufferId(0)).is_err());
        assert!(backend.present(BufferId(99)).is_err());
    }

    #[test]
    fn gpu_backend_present_count_starts_at_zero() {
        let mock = MockPlatform::new();
        let backend = GpuBackend::new(mock, 800, 600).unwrap();
        assert_eq!(backend.present_count(), 0);
    }

    #[test]
    fn gpu_backend_destroy_buffer() {
        let mock = MockPlatform::new();
        let mut backend = GpuBackend::new(mock, 800, 600).unwrap();
        let id = backend.create_buffer(10, 10).unwrap();
        backend.destroy_buffer(id).unwrap();
        assert_eq!(backend.buffer_size(id), None);
    }

    #[test]
    fn gpu_backend_destroy_invalid_returns_error() {
        let mock = MockPlatform::new();
        let mut backend = GpuBackend::new(mock, 800, 600).unwrap();
        assert!(backend.destroy_buffer(BufferId(0)).is_err());
        assert!(backend.destroy_buffer(BufferId(99)).is_err());
    }

    #[test]
    fn gpu_backend_max_buffers() {
        let mock = MockPlatform::new();
        let mut backend = GpuBackend::new(mock, 800, 600).unwrap();
        for i in 0..MAX_GPU_BUFFERS {
            let id = backend.create_buffer(1, 1).unwrap();
            assert_eq!(id, BufferId(i as u32));
        }
        // 9th allocation should fail.
        assert!(backend.create_buffer(1, 1).is_err());
    }

    #[test]
    fn gpu_backend_slot_reuse_after_destroy() {
        let mock = MockPlatform::new();
        let mut backend = GpuBackend::new(mock, 800, 600).unwrap();
        let id0 = backend.create_buffer(10, 10).unwrap();
        let _id1 = backend.create_buffer(20, 20).unwrap();
        backend.destroy_buffer(id0).unwrap();

        // Should reuse slot 0.
        let id_reused = backend.create_buffer(30, 30).unwrap();
        assert_eq!(id_reused, BufferId(0));
        assert_eq!(backend.buffer_size(id_reused), Some((30, 30)));
    }

    #[test]
    fn gpu_backend_buffer_size_invalid_returns_none() {
        let mock = MockPlatform::new();
        let backend = GpuBackend::new(mock, 800, 600).unwrap();
        assert!(backend.buffer_size(BufferId(0)).is_none());
        assert!(backend.buffer_size(BufferId(99)).is_none());
    }

    #[test]
    fn gpu_backend_display_dimensions() {
        let mock = MockPlatform::new();
        let backend = GpuBackend::new(mock, 2560, 1440).unwrap();
        assert_eq!(backend.display_width(), 2560);
        assert_eq!(backend.display_height(), 1440);
    }

    #[test]
    fn gpu_backend_present_after_destroy_fails() {
        let mock = MockPlatform::new();
        let mut backend = GpuBackend::new(mock, 800, 600).unwrap();
        let id = backend.create_buffer(10, 10).unwrap();
        backend.destroy_buffer(id).unwrap();
        assert!(backend.present(id).is_err());
    }
}
