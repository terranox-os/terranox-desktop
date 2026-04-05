//! DRM (Direct Rendering Manager) backend.
//!
//! Delegates buffer allocation, mapping, and page-flip to a
//! [`PlatformBackend`] implementation. Suitable for bare-metal
//! display output via kernel DRM/KMS ioctls.

use crate::platform::PlatformBackend;
use crate::render::backend::{BufferId, RenderBackend, RenderError};

const MAX_DRM_BUFFERS: usize = 4;

struct DrmBuffer {
    handle: i64,
    pixels: *mut u32,
    width: u32,
    height: u32,
    len: usize,
}

/// A rendering backend that delegates to a [`PlatformBackend`] for
/// DRM surface and buffer management.
pub struct DrmBackend<P: PlatformBackend> {
    platform: P,
    buffers: [Option<DrmBuffer>; MAX_DRM_BUFFERS],
    display_width: u32,
    display_height: u32,
    surface_handle: i64,
}

impl<P: PlatformBackend> DrmBackend<P> {
    /// Create a new DRM backend.
    ///
    /// Calls `platform.create_surface()` immediately. Returns an error
    /// if the platform returns a negative handle.
    pub fn new(mut platform: P, width: u32, height: u32) -> Result<Self, RenderError> {
        let surface_handle = platform.create_surface(width, height);
        if surface_handle < 0 {
            return Err(RenderError::PlatformError(surface_handle as i32));
        }
        const NONE: Option<DrmBuffer> = None;
        Ok(Self {
            platform,
            buffers: [NONE; MAX_DRM_BUFFERS],
            display_width: width,
            display_height: height,
            surface_handle,
        })
    }

    /// Access the underlying platform backend.
    pub fn platform(&self) -> &P {
        &self.platform
    }
}

impl<P: PlatformBackend> RenderBackend for DrmBackend<P> {
    fn create_buffer(&mut self, width: u32, height: u32) -> Result<BufferId, RenderError> {
        let handle = self.platform.create_buffer(width, height);
        if handle < 0 {
            return Err(RenderError::PlatformError(handle as i32));
        }
        let (pixels, _stride) = self.platform.map_buffer(handle);
        let len = (width as usize) * (height as usize);

        for (i, slot) in self.buffers.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(DrmBuffer {
                    handle,
                    pixels,
                    width,
                    height,
                    len,
                });
                return Ok(BufferId(i as u32));
            }
        }
        // No free slot — clean up the platform buffer.
        self.platform.unmap_buffer(handle);
        self.platform.destroy_surface(handle);
        Err(RenderError::BufferFull)
    }

    fn destroy_buffer(&mut self, id: BufferId) -> Result<(), RenderError> {
        let idx = id.0 as usize;
        if idx >= MAX_DRM_BUFFERS {
            return Err(RenderError::InvalidBuffer);
        }
        if let Some(buf) = self.buffers[idx].take() {
            self.platform.unmap_buffer(buf.handle);
            self.platform.destroy_surface(buf.handle);
            Ok(())
        } else {
            Err(RenderError::InvalidBuffer)
        }
    }

    fn get_pixels(&mut self, id: BufferId) -> Option<&mut [u32]> {
        let idx = id.0 as usize;
        if idx >= MAX_DRM_BUFFERS {
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
        if idx >= MAX_DRM_BUFFERS {
            return None;
        }
        self.buffers[idx].as_ref().map(|b| (b.width, b.height))
    }

    fn present(&mut self, id: BufferId) -> Result<(), RenderError> {
        let idx = id.0 as usize;
        if idx >= MAX_DRM_BUFFERS {
            return Err(RenderError::InvalidBuffer);
        }
        if let Some(buf) = &self.buffers[idx] {
            self.platform.present(self.surface_handle, buf.handle);
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
    use super::*;
    use crate::platform::MockPlatform;
    use crate::render::backend::{BufferId, RenderBackend};

    #[test]
    fn drm_backend_creates_surface() {
        let mock = MockPlatform::new();
        let backend = DrmBackend::new(mock, 800, 600).unwrap();
        assert_eq!(backend.display_width(), 800);
        assert_eq!(backend.display_height(), 600);
        // MockPlatform starts at handle 1; surface is handle 1.
        assert_eq!(backend.surface_handle, 1);
    }

    #[test]
    fn drm_backend_create_buffer() {
        let mock = MockPlatform::new();
        let mut backend = DrmBackend::new(mock, 800, 600).unwrap();
        let id = backend.create_buffer(100, 50).unwrap();
        assert_eq!(id, BufferId(0));
        assert_eq!(backend.buffer_size(id), Some((100, 50)));
    }

    #[test]
    fn drm_backend_get_pixels_null_returns_none() {
        // MockPlatform returns null from map_buffer.
        let mock = MockPlatform::new();
        let mut backend = DrmBackend::new(mock, 800, 600).unwrap();
        let id = backend.create_buffer(10, 10).unwrap();
        assert!(backend.get_pixels(id).is_none());
    }

    #[test]
    fn drm_backend_present_delegates_to_platform() {
        let mock = MockPlatform::new();
        let mut backend = DrmBackend::new(mock, 800, 600).unwrap();
        let id = backend.create_buffer(10, 10).unwrap();

        backend.present(id).unwrap();
        assert_eq!(backend.platform().presented_count(), 1);

        backend.present(id).unwrap();
        assert_eq!(backend.platform().presented_count(), 2);
    }

    #[test]
    fn drm_backend_present_invalid_returns_error() {
        let mock = MockPlatform::new();
        let mut backend = DrmBackend::new(mock, 800, 600).unwrap();
        assert!(backend.present(BufferId(0)).is_err());
        assert!(backend.present(BufferId(99)).is_err());
    }

    #[test]
    fn drm_backend_destroy_buffer() {
        let mock = MockPlatform::new();
        let mut backend = DrmBackend::new(mock, 800, 600).unwrap();
        let id = backend.create_buffer(10, 10).unwrap();
        backend.destroy_buffer(id).unwrap();
        assert_eq!(backend.buffer_size(id), None);
    }

    #[test]
    fn drm_backend_destroy_invalid_returns_error() {
        let mock = MockPlatform::new();
        let mut backend = DrmBackend::new(mock, 800, 600).unwrap();
        assert!(backend.destroy_buffer(BufferId(0)).is_err());
        assert!(backend.destroy_buffer(BufferId(99)).is_err());
    }

    #[test]
    fn drm_backend_max_buffers() {
        let mock = MockPlatform::new();
        let mut backend = DrmBackend::new(mock, 800, 600).unwrap();
        for i in 0..MAX_DRM_BUFFERS {
            let id = backend.create_buffer(1, 1).unwrap();
            assert_eq!(id, BufferId(i as u32));
        }
        assert!(backend.create_buffer(1, 1).is_err());
    }
}
