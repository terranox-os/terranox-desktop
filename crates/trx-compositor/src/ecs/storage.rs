use alloc::vec::Vec;

/// Type-erased column of component data for one archetype.
/// Stores components as raw bytes in a Vec<u8>, with stride = component size.
pub struct ComponentColumn {
    data: Vec<u8>,
    pub(crate) item_size: usize,
    #[allow(dead_code)]
    item_align: usize,
    len: usize,
    /// Change tick for each item (indexed parallel to data).
    change_ticks: Vec<u32>,
}

impl ComponentColumn {
    pub fn new(item_size: usize, item_align: usize) -> Self {
        Self {
            data: Vec::new(),
            item_size,
            item_align,
            len: 0,
            change_ticks: Vec::new(),
        }
    }

    /// Push a component value (as raw bytes).
    ///
    /// # Safety
    ///
    /// `src` must point to a valid instance of the component type with correct size.
    /// The pointed-to memory must be at least `self.item_size` bytes readable.
    pub unsafe fn push_raw(&mut self, src: *const u8, tick: u32) {
        let offset = self.len * self.item_size;
        self.data.resize(offset + self.item_size, 0);
        // SAFETY: caller guarantees src is valid for item_size bytes,
        // and we just resized data to have room at offset.
        unsafe {
            core::ptr::copy_nonoverlapping(src, self.data.as_mut_ptr().add(offset), self.item_size);
        }
        self.change_ticks.push(tick);
        self.len += 1;
    }

    /// Get a pointer to the component at `index`.
    ///
    /// # Safety
    ///
    /// Index must be in bounds. Caller must ensure correct type interpretation.
    pub unsafe fn get_raw(&self, index: usize) -> *const u8 {
        // SAFETY: caller guarantees index is in bounds
        unsafe { self.data.as_ptr().add(index * self.item_size) }
    }

    /// Get a mutable pointer to the component at `index`.
    ///
    /// # Safety
    ///
    /// Index must be in bounds. Caller must ensure correct type interpretation
    /// and that no other references to this component exist.
    pub unsafe fn get_raw_mut(&mut self, index: usize) -> *mut u8 {
        // SAFETY: caller guarantees index is in bounds
        unsafe { self.data.as_mut_ptr().add(index * self.item_size) }
    }

    /// Remove the component at `index` by swapping with the last element.
    /// Returns true if a swap occurred (caller must update entity-to-row mapping).
    pub fn swap_remove(&mut self, index: usize) -> bool {
        let last = self.len - 1;
        if index != last {
            let src_offset = last * self.item_size;
            let dst_offset = index * self.item_size;
            // SAFETY: both offsets are in bounds since index < len and last < len
            unsafe {
                core::ptr::copy(
                    self.data.as_ptr().add(src_offset),
                    self.data.as_mut_ptr().add(dst_offset),
                    self.item_size,
                );
            }
            self.change_ticks[index] = self.change_ticks[last];
        }
        self.data.truncate(last * self.item_size);
        self.change_ticks.pop();
        self.len -= 1;
        index != last
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn get_change_tick(&self, index: usize) -> u32 {
        self.change_ticks[index]
    }

    pub fn set_change_tick(&mut self, index: usize, tick: u32) {
        self.change_ticks[index] = tick;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_read_back() {
        let mut col = ComponentColumn::new(4, 4);
        let val: u32 = 42;
        unsafe {
            col.push_raw(&val as *const u32 as *const u8, 0);
        }
        assert_eq!(col.len(), 1);
        unsafe {
            let ptr = col.get_raw(0) as *const u32;
            assert_eq!(*ptr, 42);
        }
    }

    #[test]
    fn push_multiple_and_read() {
        let mut col = ComponentColumn::new(4, 4);
        for i in 0u32..5 {
            unsafe {
                col.push_raw(&i as *const u32 as *const u8, i);
            }
        }
        assert_eq!(col.len(), 5);
        for i in 0u32..5 {
            unsafe {
                let ptr = col.get_raw(i as usize) as *const u32;
                assert_eq!(*ptr, i);
            }
            assert_eq!(col.get_change_tick(i as usize), i);
        }
    }

    #[test]
    fn swap_remove_middle() {
        let mut col = ComponentColumn::new(4, 4);
        for i in 0u32..3 {
            unsafe {
                col.push_raw(&i as *const u32 as *const u8, 0);
            }
        }
        // Remove index 0 -- last element (2) swaps into position 0
        let swapped = col.swap_remove(0);
        assert!(swapped);
        assert_eq!(col.len(), 2);
        unsafe {
            let ptr = col.get_raw(0) as *const u32;
            assert_eq!(*ptr, 2); // was last, now at index 0
            let ptr = col.get_raw(1) as *const u32;
            assert_eq!(*ptr, 1); // unchanged
        }
    }

    #[test]
    fn swap_remove_last() {
        let mut col = ComponentColumn::new(4, 4);
        for i in 0u32..3 {
            unsafe {
                col.push_raw(&i as *const u32 as *const u8, 0);
            }
        }
        // Remove last -- no swap needed
        let swapped = col.swap_remove(2);
        assert!(!swapped);
        assert_eq!(col.len(), 2);
    }

    #[test]
    fn get_raw_mut_modifies_value() {
        let mut col = ComponentColumn::new(4, 4);
        let val: u32 = 10;
        unsafe {
            col.push_raw(&val as *const u32 as *const u8, 0);
            let ptr = col.get_raw_mut(0) as *mut u32;
            *ptr = 99;
            let read_ptr = col.get_raw(0) as *const u32;
            assert_eq!(*read_ptr, 99);
        }
    }

    #[test]
    fn change_tick_tracking() {
        let mut col = ComponentColumn::new(4, 4);
        let val: u32 = 0;
        unsafe {
            col.push_raw(&val as *const u32 as *const u8, 5);
        }
        assert_eq!(col.get_change_tick(0), 5);
        col.set_change_tick(0, 10);
        assert_eq!(col.get_change_tick(0), 10);
    }

    #[test]
    fn is_empty() {
        let col = ComponentColumn::new(4, 4);
        assert!(col.is_empty());
    }
}
