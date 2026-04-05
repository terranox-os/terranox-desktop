//! Platform abstraction layer for compositor backends.
//!
//! Provides the `PlatformBackend` trait for real hardware/display servers
//! and a `MockPlatform` for testing.

/// Platform abstraction for surface, buffer, and input management.
pub trait PlatformBackend {
    /// Create a compositor surface. Returns an opaque handle.
    fn create_surface(&mut self, width: u32, height: u32) -> i64;

    /// Destroy a compositor surface.
    fn destroy_surface(&mut self, handle: i64);

    /// Create a pixel buffer. Returns an opaque handle.
    fn create_buffer(&mut self, width: u32, height: u32) -> i64;

    /// Map a buffer for CPU access. Returns (pixel_ptr, stride).
    fn map_buffer(&mut self, handle: i64) -> (*mut u32, u32);

    /// Unmap a previously mapped buffer.
    fn unmap_buffer(&mut self, handle: i64);

    /// Present a buffer to a surface (display it).
    fn present(&mut self, surface: i64, buffer: i64);

    /// Poll for input events. Returns number of events written.
    /// Each event is (timestamp, type, code, value, device).
    fn poll_input(&mut self, events: &mut [(u64, u32, u32, i32, u32)]) -> usize;
}

/// Mock platform backend for testing.
pub struct MockPlatform {
    next_handle: i64,
    presented_count: u32,
}

impl MockPlatform {
    pub fn new() -> Self {
        Self {
            next_handle: 1,
            presented_count: 0,
        }
    }

    /// How many times `present` has been called.
    pub fn presented_count(&self) -> u32 {
        self.presented_count
    }
}

impl Default for MockPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformBackend for MockPlatform {
    fn create_surface(&mut self, _width: u32, _height: u32) -> i64 {
        let h = self.next_handle;
        self.next_handle += 1;
        h
    }

    fn destroy_surface(&mut self, _handle: i64) {}

    fn create_buffer(&mut self, _width: u32, _height: u32) -> i64 {
        let h = self.next_handle;
        self.next_handle += 1;
        h
    }

    fn map_buffer(&mut self, _handle: i64) -> (*mut u32, u32) {
        (core::ptr::null_mut(), 0)
    }

    fn unmap_buffer(&mut self, _handle: i64) {}

    fn present(&mut self, _surface: i64, _buffer: i64) {
        self.presented_count += 1;
    }

    fn poll_input(&mut self, _events: &mut [(u64, u32, u32, i32, u32)]) -> usize {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_creates_sequential_handles() {
        let mut mock = MockPlatform::new();
        let s1 = mock.create_surface(800, 600);
        let s2 = mock.create_surface(1920, 1080);
        let b1 = mock.create_buffer(800, 600);
        assert_eq!(s1, 1);
        assert_eq!(s2, 2);
        assert_eq!(b1, 3);
    }

    #[test]
    fn mock_tracks_present_count() {
        let mut mock = MockPlatform::new();
        assert_eq!(mock.presented_count(), 0);
        mock.present(1, 2);
        mock.present(1, 3);
        mock.present(1, 4);
        assert_eq!(mock.presented_count(), 3);
    }

    #[test]
    fn mock_map_returns_null() {
        let mut mock = MockPlatform::new();
        let b = mock.create_buffer(100, 100);
        let (ptr, stride) = mock.map_buffer(b);
        assert!(ptr.is_null());
        assert_eq!(stride, 0);
    }

    #[test]
    fn mock_poll_input_returns_zero() {
        let mut mock = MockPlatform::new();
        let mut buf = [(0u64, 0u32, 0u32, 0i32, 0u32); 8];
        let count = mock.poll_input(&mut buf);
        assert_eq!(count, 0);
    }

    #[test]
    fn mock_destroy_surface_is_noop() {
        let mut mock = MockPlatform::new();
        let s = mock.create_surface(100, 100);
        mock.destroy_surface(s); // should not panic
    }

    #[test]
    fn mock_unmap_buffer_is_noop() {
        let mut mock = MockPlatform::new();
        let b = mock.create_buffer(100, 100);
        mock.unmap_buffer(b); // should not panic
    }

    #[test]
    fn mock_default() {
        let mock = MockPlatform::default();
        assert_eq!(mock.presented_count(), 0);
    }
}
