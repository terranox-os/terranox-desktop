use alloc::vec::Vec;

/// Fixed-capacity ring buffer for events of one type.
pub struct Events<T> {
    buffer: Vec<T>,
    capacity: usize,
    write_idx: usize,
    /// Generation counter -- incremented on clear.
    generation: u32,
}

impl<T> Events<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            capacity,
            write_idx: 0,
            generation: 0,
        }
    }

    /// Send an event into the channel.
    pub fn send(&mut self, event: T) {
        if self.buffer.len() < self.capacity {
            self.buffer.push(event);
        } else {
            self.buffer[self.write_idx % self.capacity] = event;
        }
        self.write_idx += 1;
    }

    /// Clear all events and increment the generation counter.
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.write_idx = 0;
        self.generation += 1;
    }

    /// Number of events currently in the buffer.
    pub fn len(&self) -> usize {
        self.buffer.len().min(self.write_idx)
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Current generation (incremented on each clear).
    pub fn generation(&self) -> u32 {
        self.generation
    }
}

/// Cursor for reading events. Tracks position per reader.
pub struct EventReader {
    last_read: usize,
    generation: u32,
}

impl Default for EventReader {
    fn default() -> Self {
        Self::new()
    }
}

impl EventReader {
    pub fn new() -> Self {
        Self {
            last_read: 0,
            generation: 0,
        }
    }

    /// Read unread events from the channel. Returns a slice of events
    /// that have been sent since the last read (or since the last clear).
    pub fn read<'a, T>(&mut self, events: &'a Events<T>) -> &'a [T] {
        if self.generation != events.generation {
            self.last_read = 0;
            self.generation = events.generation;
        }
        let start = self.last_read;
        let end = events.buffer.len();
        self.last_read = end;
        if start < end {
            &events.buffer[start..end]
        } else {
            &[]
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct MouseClick {
        x: i32,
        y: i32,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct KeyPress {
        code: u32,
    }

    #[test]
    fn send_and_read_events() {
        let mut events = Events::new(16);
        let mut reader = EventReader::new();

        events.send(MouseClick { x: 10, y: 20 });
        events.send(MouseClick { x: 30, y: 40 });

        let read = reader.read(&events);
        assert_eq!(read.len(), 2);
        assert_eq!(read[0], MouseClick { x: 10, y: 20 });
        assert_eq!(read[1], MouseClick { x: 30, y: 40 });
    }

    #[test]
    fn reader_only_sees_new_events() {
        let mut events = Events::new(16);
        let mut reader = EventReader::new();

        events.send(MouseClick { x: 1, y: 1 });
        let _ = reader.read(&events);

        events.send(MouseClick { x: 2, y: 2 });
        let read = reader.read(&events);
        assert_eq!(read.len(), 1);
        assert_eq!(read[0], MouseClick { x: 2, y: 2 });
    }

    #[test]
    fn multiple_readers_see_same_events() {
        let mut events = Events::new(16);
        let mut reader_a = EventReader::new();
        let mut reader_b = EventReader::new();

        events.send(MouseClick { x: 5, y: 5 });
        events.send(MouseClick { x: 10, y: 10 });

        let read_a = reader_a.read(&events);
        let read_b = reader_b.read(&events);
        assert_eq!(read_a.len(), 2);
        assert_eq!(read_b.len(), 2);
        assert_eq!(read_a[0], read_b[0]);
        assert_eq!(read_a[1], read_b[1]);
    }

    #[test]
    fn clear_resets_and_increments_generation() {
        let mut events = Events::new(16);
        assert_eq!(events.generation(), 0);

        events.send(MouseClick { x: 1, y: 1 });
        assert_eq!(events.len(), 1);

        events.clear();
        assert_eq!(events.len(), 0);
        assert!(events.is_empty());
        assert_eq!(events.generation(), 1);
    }

    #[test]
    fn reader_resets_on_generation_change() {
        let mut events = Events::new(16);
        let mut reader = EventReader::new();

        events.send(MouseClick { x: 1, y: 1 });
        let _ = reader.read(&events);

        events.clear();
        events.send(MouseClick { x: 99, y: 99 });

        let read = reader.read(&events);
        assert_eq!(read.len(), 1);
        assert_eq!(read[0], MouseClick { x: 99, y: 99 });
    }

    #[test]
    fn capacity_limit_overwrites_oldest() {
        let mut events = Events::new(3);

        events.send(KeyPress { code: 1 });
        events.send(KeyPress { code: 2 });
        events.send(KeyPress { code: 3 });
        // Buffer is now full; next send overwrites
        events.send(KeyPress { code: 4 });

        // The buffer now contains [4, 2, 3] (index 0 was overwritten)
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn empty_read_returns_empty_slice() {
        let events: Events<MouseClick> = Events::new(16);
        let mut reader = EventReader::new();
        let read = reader.read(&events);
        assert!(read.is_empty());
    }

    #[test]
    fn repeated_read_without_new_events_returns_empty() {
        let mut events = Events::new(16);
        let mut reader = EventReader::new();

        events.send(MouseClick { x: 1, y: 1 });
        let _ = reader.read(&events);
        let read = reader.read(&events);
        assert!(read.is_empty());
    }

    #[test]
    fn is_empty_and_len() {
        let mut events: Events<MouseClick> = Events::new(16);
        assert!(events.is_empty());
        assert_eq!(events.len(), 0);

        events.send(MouseClick { x: 0, y: 0 });
        assert!(!events.is_empty());
        assert_eq!(events.len(), 1);
    }
}
