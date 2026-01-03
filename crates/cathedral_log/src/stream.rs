//! Event stream for sequential event access.

use cathedral_core::{RunId, NodeId, LogicalTime, CoreResult};
use crate::event::EventKind;

/// Simplified Event for stream testing
pub struct Event {
    pub logical_time: LogicalTime,
}

/// Event stream for reading events sequentially
pub struct EventStream {
    events: Vec<Event>,
    position: usize,
}

impl EventStream {
    pub fn new(events: Vec<Event>) -> Self {
        Self {
            events,
            position: 0,
        }
    }

    pub fn next(&mut self) -> Option<&Event> {
        if self.position < self.events.len() {
            let event = &self.events[self.position];
            self.position += 1;
            Some(event)
        } else {
            None
        }
    }

    pub fn peek(&self) -> Option<&Event> {
        if self.position < self.events.len() {
            Some(&self.events[self.position])
        } else {
            None
        }
    }

    pub fn remaining(&self) -> usize {
        self.events.len().saturating_sub(self.position)
    }

    pub fn is_end(&self) -> bool {
        self.position >= self.events.len()
    }

    pub fn reset(&mut self) {
        self.position = 0;
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }
}

/// Stream writer for appending events
pub struct StreamWriter {
    events: Vec<Event>,
}

impl StreamWriter {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn write(&mut self, event: Event) {
        self.events.push(event);
    }

    pub fn events(&self) -> &[Event] {
        &self.events
    }

    pub fn finalize(self) -> Vec<Event> {
        self.events
    }
}

impl Default for StreamWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Stream errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamError {
    InvalidPosition { position: usize },
    EventNotFound,
}

impl std::fmt::Display for StreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPosition { position } => {
                write!(f, "Invalid position: {}", position)
            }
            Self::EventNotFound => write!(f, "Event not found"),
        }
    }
}

impl std::error::Error for StreamError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_event(time: u64) -> Event {
        Event { logical_time: LogicalTime::from_raw(time) }
    }

    #[test]
    fn test_stream_next() {
        let events = vec![make_test_event(0), make_test_event(1)];
        let mut stream = EventStream::new(events);
        assert_eq!(stream.next().unwrap().logical_time.as_u64(), 0);
        assert_eq!(stream.next().unwrap().logical_time.as_u64(), 1);
        assert!(stream.next().is_none());
    }

    #[test]
    fn test_stream_peek() {
        let events = vec![make_test_event(0)];
        let stream = EventStream::new(events);
        assert_eq!(stream.peek().unwrap().logical_time.as_u64(), 0);
        assert_eq!(stream.peek().unwrap().logical_time.as_u64(), 0);
    }

    #[test]
    fn test_stream_remaining() {
        let events = vec![make_test_event(0), make_test_event(1)];
        let mut stream = EventStream::new(events);
        assert_eq!(stream.remaining(), 2);
        stream.next();
        assert_eq!(stream.remaining(), 1);
    }

    #[test]
    fn test_writer() {
        let mut writer = StreamWriter::new();
        writer.write(make_test_event(0));
        assert_eq!(writer.events.len(), 1);
    }
}
