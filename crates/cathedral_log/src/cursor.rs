//! Cursor for navigating event streams.

use cathedral_core::{EventId, CoreResult};

/// Cursor position in an event stream
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    pub position: u64,
    pub direction: Direction,
}

/// Direction for cursor movement
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Backward,
}

impl Cursor {
    #[must_use]
    pub fn new() -> Self {
        Self {
            position: 0,
            direction: Direction::Forward,
        }
    }

    #[must_use]
    pub fn at(position: u64) -> Self {
        Self {
            position,
            direction: Direction::Forward,
        }
    }

    pub fn move_forward(&mut self, count: u64) {
        self.position = self.position.saturating_add(count);
        self.direction = Direction::Forward;
    }

    pub fn move_backward(&mut self, count: u64) {
        self.position = self.position.saturating_sub(count);
        self.direction = Direction::Backward;
    }

    pub fn seek(&mut self, position: u64) {
        self.position = position;
    }

    #[must_use]
    pub const fn pos(&self) -> u64 {
        self.position
    }

    pub fn reset(&mut self) {
        self.position = 0;
        self.direction = Direction::Forward;
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_new() {
        let cursor = Cursor::new();
        assert_eq!(cursor.pos(), 0);
    }

    #[test]
    fn test_cursor_at() {
        let cursor = Cursor::at(42);
        assert_eq!(cursor.pos(), 42);
    }

    #[test]
    fn test_cursor_move_forward() {
        let mut cursor = Cursor::new();
        cursor.move_forward(5);
        assert_eq!(cursor.pos(), 5);
    }

    #[test]
    fn test_cursor_move_backward() {
        let mut cursor = Cursor::at(10);
        cursor.move_backward(3);
        assert_eq!(cursor.pos(), 7);
    }

    #[test]
    fn test_cursor_seek() {
        let mut cursor = Cursor::new();
        cursor.seek(100);
        assert_eq!(cursor.pos(), 100);
    }

    #[test]
    fn test_cursor_reset() {
        let mut cursor = Cursor::at(50);
        cursor.reset();
        assert_eq!(cursor.pos(), 0);
    }

    #[test]
    fn test_direction() {
        let cursor = Cursor::new();
        assert!(matches!(cursor.direction, Direction::Forward));
    }
}
