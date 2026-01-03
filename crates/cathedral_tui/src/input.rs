//! TUI input handling for keyboard events and key bindings.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;
use std::io;
use std::time::Duration;

/// Input event from the terminal
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputEvent {
    /// Quit the application
    Quit,
    /// Show help
    Help,
    /// Switch to timeline view
    ViewTimeline,
    /// Switch to DAG view
    ViewDag,
    /// Switch to worker view
    ViewWorker,
    /// Switch to provenance view
    ViewProvenance,
    /// Move down
    Down,
    /// Move up
    Up,
    /// Move left
    Left,
    /// Move right
    Right,
    /// Go to top
    GoTop,
    /// Go to bottom
    GoBottom,
    /// Select current item
    Select,
    /// Search
    Search,
    /// Next search result
    SearchNext,
    /// Previous search result
    SearchPrev,
    /// Refresh
    Refresh,
    /// Unknown key
    Unknown,
}

/// Key binding configuration
#[derive(Debug, Clone)]
pub struct KeyBinding {
    /// Key to action mappings
    bindings: HashMap<KeyCombo, InputEvent>,
}

/// Key combination (key + modifiers)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyCombo {
    /// The key code
    pub code: KeyCode,
    /// Modifiers (ctrl, alt, shift)
    pub modifiers: KeyModifiers,
}

impl KeyCombo {
    /// Create a new key combination
    #[must_use]
    pub fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    /// Create a plain key without modifiers
    #[must_use]
    pub fn key(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::empty(),
        }
    }

    /// Create a Ctrl+key combination
    #[must_use]
    pub fn ctrl(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::CONTROL,
        }
    }
}

impl Default for KeyBinding {
    fn default() -> Self {
        let mut bindings = HashMap::new();

        // Navigation
        bindings.insert(KeyCombo::key(KeyCode::Down), InputEvent::Down);
        bindings.insert(KeyCombo::key(KeyCode::Char('j')), InputEvent::Down);
        bindings.insert(KeyCombo::key(KeyCode::Up), InputEvent::Up);
        bindings.insert(KeyCombo::key(KeyCode::Char('k')), InputEvent::Up);
        bindings.insert(KeyCombo::key(KeyCode::Left), InputEvent::Left);
        bindings.insert(KeyCombo::key(KeyCode::Char('h')), InputEvent::Left);
        bindings.insert(KeyCombo::key(KeyCode::Right), InputEvent::Right);
        bindings.insert(KeyCombo::key(KeyCode::Char('l')), InputEvent::Right);
        bindings.insert(KeyCombo::key(KeyCode::Char('g')), InputEvent::GoTop);
        bindings.insert(KeyCombo::key(KeyCode::Char('G')), InputEvent::GoBottom);

        // View switching
        bindings.insert(KeyCombo::key(KeyCode::Char('1')), InputEvent::ViewTimeline);
        bindings.insert(KeyCombo::key(KeyCode::Char('2')), InputEvent::ViewDag);
        bindings.insert(KeyCombo::key(KeyCode::Char('3')), InputEvent::ViewWorker);
        bindings.insert(KeyCombo::key(KeyCode::Char('4')), InputEvent::ViewProvenance);

        // Actions
        bindings.insert(KeyCombo::key(KeyCode::Enter), InputEvent::Select);
        bindings.insert(KeyCombo::key(KeyCode::Char('/')), InputEvent::Search);
        bindings.insert(KeyCombo::key(KeyCode::Char('n')), InputEvent::SearchNext);
        bindings.insert(KeyCombo::key(KeyCode::Char('p')), InputEvent::SearchPrev);
        bindings.insert(KeyCombo::key(KeyCode::Char('r')), InputEvent::Refresh);
        bindings.insert(KeyCombo::key(KeyCode::Char('?')), InputEvent::Help);

        // Quit
        bindings.insert(KeyCombo::key(KeyCode::Char('q')), InputEvent::Quit);
        bindings.insert(KeyCombo::ctrl(KeyCode::Char('c')), InputEvent::Quit);
        bindings.insert(KeyCombo::ctrl(KeyCode::Char('d')), InputEvent::Quit);

        Self { bindings }
    }
}

/// Input handler for terminal events
pub struct InputHandler {
    /// Key bindings
    bindings: KeyBinding,
    /// Poll timeout
    timeout: Duration,
}

impl InputHandler {
    /// Create a new input handler
    #[must_use]
    pub fn new() -> Self {
        Self {
            bindings: KeyBinding::default(),
            timeout: Duration::from_millis(100),
        }
    }

    /// Create with custom key bindings
    #[must_use]
    pub fn with_bindings(bindings: KeyBinding) -> Self {
        Self {
            bindings,
            timeout: Duration::from_millis(100),
        }
    }

    /// Set poll timeout
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Get the next input event
    ///
    /// # Errors
    ///
    /// Returns error if reading from terminal fails
    pub fn next_event(&self) -> Result<Option<InputEvent>, InputError> {
        if crossterm::event::poll(self.timeout)? {
            if let Event::Key(key) = crossterm::event::read()? {
                return Ok(Some(self.map_key(key)));
            }
        }
        Ok(None)
    }

    /// Map a KeyEvent to an InputEvent using key bindings
    fn map_key(&self, key: KeyEvent) -> InputEvent {
        let combo = KeyCombo::new(key.code, key.modifiers);
        self.bindings.bindings.get(&combo).cloned().unwrap_or(InputEvent::Unknown)
    }

    /// Check if a key press should be treated as quit
    #[must_use]
    pub fn is_quit(&self, event: &InputEvent) -> bool {
        matches!(event, InputEvent::Quit)
    }
}

impl Default for InputHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Input-related errors
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InputError {
    /// IO error
    #[error("IO error: {0}")]
    Io(String),
    /// Terminal error
    #[error("terminal error")]
    Terminal,
}

impl From<io::Error> for InputError {
    fn from(err: io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_combo_new() {
        let combo = KeyCombo::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
        assert_eq!(combo.code, KeyCode::Char('a'));
        assert_eq!(combo.modifiers, KeyModifiers::CONTROL);
    }

    #[test]
    fn test_key_combo_key() {
        let combo = KeyCombo::key(KeyCode::Char('j'));
        assert_eq!(combo.code, KeyCode::Char('j'));
        assert!(combo.modifiers.is_empty());
    }

    #[test]
    fn test_key_combo_ctrl() {
        let combo = KeyCombo::ctrl(KeyCode::Char('c'));
        assert_eq!(combo.code, KeyCode::Char('c'));
        assert_eq!(combo.modifiers, KeyModifiers::CONTROL);
    }

    #[test]
    fn test_key_combo_hashable() {
        let combo1 = KeyCombo::key(KeyCode::Char('j'));
        let combo2 = KeyCombo::key(KeyCode::Char('j'));
        assert_eq!(combo1, combo2);

        let mut map = HashMap::new();
        map.insert(combo1, "down");
        assert_eq!(map.get(&combo2), Some(&"down"));
    }

    #[test]
    fn test_key_binding_default() {
        let binding = KeyBinding::default();
        // Check that 'q' is mapped to quit
        let quit_combo = KeyCombo::key(KeyCode::Char('q'));
        assert_eq!(binding.bindings.get(&quit_combo), Some(&InputEvent::Quit));
    }

    #[test]
    fn test_key_binding_navigation() {
        let binding = KeyBinding::default();
        // Down arrow and 'j' should map to Down
        let down_arrow = KeyCombo::key(KeyCode::Down);
        let j_key = KeyCombo::key(KeyCode::Char('j'));
        assert_eq!(binding.bindings.get(&down_arrow), Some(&InputEvent::Down));
        assert_eq!(binding.bindings.get(&j_key), Some(&InputEvent::Down));
    }

    #[test]
    fn test_input_handler_new() {
        let handler = InputHandler::new();
        assert_eq!(handler.timeout, Duration::from_millis(100));
    }

    #[test]
    fn test_input_handler_with_timeout() {
        let handler = InputHandler::new().with_timeout(Duration::from_millis(250));
        assert_eq!(handler.timeout, Duration::from_millis(250));
    }

    #[test]
    fn test_input_handler_map_key() {
        let handler = InputHandler::new();

        // Test 'q' maps to quit
        let quit_key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty());
        assert_eq!(handler.map_key(quit_key), InputEvent::Quit);

        // Test Ctrl+c maps to quit
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(handler.map_key(ctrl_c), InputEvent::Quit);
    }

    #[test]
    fn test_input_handler_map_unknown_key() {
        let handler = InputHandler::new();

        // Unknown key should map to Unknown
        let unknown_key = KeyEvent::new(KeyCode::Null, KeyModifiers::empty());
        assert_eq!(handler.map_key(unknown_key), InputEvent::Unknown);
    }

    #[test]
    fn test_is_quit() {
        let handler = InputHandler::new();
        assert!(handler.is_quit(&InputEvent::Quit));
        assert!(!handler.is_quit(&InputEvent::Down));
        assert!(!handler.is_quit(&InputEvent::Help));
    }

    #[test]
    fn test_input_event_copy() {
        let event = InputEvent::Quit;
        assert_eq!(event, InputEvent::Quit);
    }

    #[test]
    fn test_input_event_variants() {
        // Test that all variants can be created
        let _ = InputEvent::Quit;
        let _ = InputEvent::Help;
        let _ = InputEvent::ViewTimeline;
        let _ = InputEvent::ViewDag;
        let _ = InputEvent::ViewWorker;
        let _ = InputEvent::ViewProvenance;
        let _ = InputEvent::Down;
        let _ = InputEvent::Up;
        let _ = InputEvent::Left;
        let _ = InputEvent::Right;
        let _ = InputEvent::GoTop;
        let _ = InputEvent::GoBottom;
        let _ = InputEvent::Select;
        let _ = InputEvent::Search;
        let _ = InputEvent::SearchNext;
        let _ = InputEvent::SearchPrev;
        let _ = InputEvent::Refresh;
        let _ = InputEvent::Unknown;
    }
}
