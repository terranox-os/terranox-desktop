//! Software rendering backend.
//!
//! Allocates pixel buffers in heap memory via `alloc::vec::Vec<u32>`.
//! Suitable for testing and fallback when no GPU or DRM is available.

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use crate::render::backend::{BufferId, RenderBackend, RenderError};

const MAX_BUFFERS: usize = 16;

struct SoftBuffer {
    pixels: Vec<u32>,
    width: u32,
    height: u32,
}

/// A pure-software rendering backend backed by heap-allocated pixel buffers.
pub struct SoftwareBackend {
    buffers: [Option<SoftBuffer>; MAX_BUFFERS],
    display_width: u32,
    display_height: u32,
    present_count: u32,
}

impl SoftwareBackend {
    /// Create a new software backend for the given display dimensions.
    pub fn new(width: u32, height: u32) -> Self {
        // Initialize the array without requiring SoftBuffer: Copy.
        const NONE: Option<SoftBuffer> = None;
        Self {
            buffers: [NONE; MAX_BUFFERS],
            display_width: width,
            display_height: height,
            present_count: 0,
        }
    }

    /// Number of times [`RenderBackend::present`] has been called successfully.
    pub fn present_count(&self) -> u32 {
        self.present_count
    }
}

impl RenderBackend for SoftwareBackend {
    fn create_buffer(&mut self, width: u32, height: u32) -> Result<BufferId, RenderError> {
        for (i, slot) in self.buffers.iter_mut().enumerate() {
            if slot.is_none() {
                let pixels = vec![0u32; (width as usize) * (height as usize)];
                *slot = Some(SoftBuffer {
                    pixels,
                    width,
                    height,
                });
                return Ok(BufferId(i as u32));
            }
        }
        Err(RenderError::BufferFull)
    }

    fn destroy_buffer(&mut self, id: BufferId) -> Result<(), RenderError> {
        let idx = id.0 as usize;
        if idx >= MAX_BUFFERS {
            return Err(RenderError::InvalidBuffer);
        }
        if self.buffers[idx].is_none() {
            return Err(RenderError::InvalidBuffer);
        }
        self.buffers[idx] = None;
        Ok(())
    }

    fn get_pixels(&mut self, id: BufferId) -> Option<&mut [u32]> {
        let idx = id.0 as usize;
        if idx >= MAX_BUFFERS {
            return None;
        }
        self.buffers[idx].as_mut().map(|b| b.pixels.as_mut_slice())
    }

    fn buffer_size(&self, id: BufferId) -> Option<(u32, u32)> {
        let idx = id.0 as usize;
        if idx >= MAX_BUFFERS {
            return None;
        }
        self.buffers[idx].as_ref().map(|b| (b.width, b.height))
    }

    fn present(&mut self, id: BufferId) -> Result<(), RenderError> {
        let idx = id.0 as usize;
        if idx >= MAX_BUFFERS || self.buffers[idx].is_none() {
            return Err(RenderError::InvalidBuffer);
        }
        self.present_count += 1;
        Ok(())
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
    use crate::render::backend::{BufferId, RenderBackend};

    #[test]
    fn create_and_destroy_buffer() {
        let mut backend = SoftwareBackend::new(800, 600);
        let id = backend.create_buffer(100, 50).unwrap();
        assert_eq!(id, BufferId(0));
        assert_eq!(backend.buffer_size(id), Some((100, 50)));

        backend.destroy_buffer(id).unwrap();
        assert_eq!(backend.buffer_size(id), None);
    }

    #[test]
    fn destroy_invalid_buffer_returns_error() {
        let mut backend = SoftwareBackend::new(800, 600);
        assert!(backend.destroy_buffer(BufferId(0)).is_err());
        assert!(backend.destroy_buffer(BufferId(99)).is_err());
    }

    #[test]
    fn get_pixels_writes_correctly() {
        let mut backend = SoftwareBackend::new(800, 600);
        let id = backend.create_buffer(4, 2).unwrap();

        let pixels = backend.get_pixels(id).unwrap();
        assert_eq!(pixels.len(), 8);
        pixels[0] = 0xFFFF_FFFF;
        pixels[7] = 0xAABB_CCDD;

        let pixels = backend.get_pixels(id).unwrap();
        assert_eq!(pixels[0], 0xFFFF_FFFF);
        assert_eq!(pixels[7], 0xAABB_CCDD);
    }

    #[test]
    fn get_pixels_invalid_returns_none() {
        let mut backend = SoftwareBackend::new(800, 600);
        assert!(backend.get_pixels(BufferId(0)).is_none());
        assert!(backend.get_pixels(BufferId(99)).is_none());
    }

    #[test]
    fn present_increments_count() {
        let mut backend = SoftwareBackend::new(800, 600);
        let id = backend.create_buffer(10, 10).unwrap();
        assert_eq!(backend.present_count(), 0);

        backend.present(id).unwrap();
        assert_eq!(backend.present_count(), 1);

        backend.present(id).unwrap();
        backend.present(id).unwrap();
        assert_eq!(backend.present_count(), 3);
    }

    #[test]
    fn present_invalid_buffer_returns_error() {
        let mut backend = SoftwareBackend::new(800, 600);
        assert!(backend.present(BufferId(0)).is_err());
    }

    #[test]
    fn max_buffers_returns_buffer_full() {
        let mut backend = SoftwareBackend::new(800, 600);
        for i in 0..MAX_BUFFERS {
            let id = backend.create_buffer(1, 1).unwrap();
            assert_eq!(id, BufferId(i as u32));
        }
        // 17th allocation should fail
        assert!(backend.create_buffer(1, 1).is_err());
    }

    #[test]
    fn slot_reuse_after_destroy() {
        let mut backend = SoftwareBackend::new(800, 600);
        let id0 = backend.create_buffer(1, 1).unwrap();
        let _id1 = backend.create_buffer(1, 1).unwrap();
        backend.destroy_buffer(id0).unwrap();

        // Should reuse slot 0
        let id_reused = backend.create_buffer(2, 2).unwrap();
        assert_eq!(id_reused, BufferId(0));
        assert_eq!(backend.buffer_size(id_reused), Some((2, 2)));
    }

    #[test]
    fn display_dimensions() {
        let backend = SoftwareBackend::new(1920, 1080);
        assert_eq!(backend.display_width(), 1920);
        assert_eq!(backend.display_height(), 1080);
    }

    #[test]
    fn buffers_initialized_zeroed() {
        let mut backend = SoftwareBackend::new(800, 600);
        let id = backend.create_buffer(4, 4).unwrap();
        let pixels = backend.get_pixels(id).unwrap();
        assert!(pixels.iter().all(|&p| p == 0));
    }
}
