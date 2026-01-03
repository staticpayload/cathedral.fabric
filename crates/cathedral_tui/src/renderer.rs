//! TUI renderer for drawing views and widgets.

use ratatui::{
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use serde::{Deserialize, Serialize};

/// Renderer for TUI frames
pub struct Renderer {
    /// Render configuration
    config: RenderConfig,
    /// Frame counter
    frame_count: usize,
}

impl Renderer {
    /// Create a new renderer
    #[must_use]
    pub fn new(config: RenderConfig) -> Self {
        Self {
            config,
            frame_count: 0,
        }
    }

    /// Get the configuration
    #[must_use]
    pub fn config(&self) -> &RenderConfig {
        &self.config
    }

    /// Set the configuration
    #[must_use]
    pub fn with_config(mut self, config: RenderConfig) -> Self {
        self.config = config;
        self
    }

    /// Render a border with title
    pub fn render_border(&self, f: &mut Frame, area: Rect, title: &str) {
        let block = Block::default()
            .title(format!(" {} ", title))
            .borders(Borders::ALL)
            .border_style(self.config.border_style());

        f.render_widget(block, area);
    }

    /// Render a paragraph with wrapping
    pub fn render_paragraph(&self, f: &mut Frame, area: Rect, text: &str) {
        let paragraph = Paragraph::new(text)
            .block(Block::default())
            .wrap(Wrap { trim: false });

        f.render_widget(paragraph, area);
    }

    /// Render a paragraph with style
    pub fn render_styled_paragraph(&self, f: &mut Frame, area: Rect, text: &str, style: Style) {
        let paragraph = Paragraph::new(text)
            .style(style)
            .wrap(Wrap { trim: false });

        f.render_widget(paragraph, area);
    }

    /// Render a list of lines
    pub fn render_lines(&self, f: &mut Frame, area: Rect, lines: &[Line]) {
        let paragraph = Paragraph::new(lines.to_vec())
            .wrap(Wrap { trim: false });

        f.render_widget(paragraph, area);
    }

    /// Render a status message
    pub fn render_status(&self, f: &mut Frame, area: Rect, message: &str) {
        let style = Style::default()
            .fg(self.config.status_color())
            .add_modifier(Modifier::BOLD);

        let paragraph = Paragraph::new(message)
            .style(style)
            .wrap(Wrap { trim: false });

        f.render_widget(paragraph, area);
    }

    /// Render an error message
    pub fn render_error(&self, f: &mut Frame, area: Rect, message: &str) {
        let style = Style::default()
            .fg(Color::Red)
            .add_modifier(Modifier::BOLD);

        let paragraph = Paragraph::new(message)
            .style(style)
            .wrap(Wrap { trim: false });

        f.render_widget(paragraph, area);
    }

    /// Render a warning message
    pub fn render_warning(&self, f: &mut Frame, area: Rect, message: &str) {
        let style = Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD);

        let paragraph = Paragraph::new(message)
            .style(style)
            .wrap(Wrap { trim: false });

        f.render_widget(paragraph, area);
    }

    /// Render a success message
    pub fn render_success(&self, f: &mut Frame, area: Rect, message: &str) {
        let style = Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD);

        let paragraph = Paragraph::new(message)
            .style(style)
            .wrap(Wrap { trim: false });

        f.render_widget(paragraph, area);
    }

    /// Get color for a level (0-255)
    #[must_use]
    pub fn level_color(&self, level: u8) -> Color {
        match level {
            0 => Color::DarkGray,
            1..=50 => Color::Blue,
            51..=100 => Color::Cyan,
            101..=150 => Color::Green,
            151..=200 => Color::Yellow,
            _ => Color::Red,
        }
    }

    /// Increment frame counter
    pub fn tick(&mut self) {
        self.frame_count += 1;
    }

    /// Get frame count
    #[must_use]
    pub fn frame_count(&self) -> usize {
        self.frame_count
    }
}

/// Render configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderConfig {
    /// Border style
    pub border_style: BorderStyle,
    /// Status color
    pub status_color: StatusColor,
    /// Enable colors
    pub enable_colors: bool,
    /// Enable bold text
    pub enable_bold: bool,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            border_style: BorderStyle::default(),
            status_color: StatusColor::default(),
            enable_colors: true,
            enable_bold: true,
        }
    }
}

impl RenderConfig {
    /// Create a new render config
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a minimal config (no colors, no bold)
    #[must_use]
    pub fn minimal() -> Self {
        Self {
            border_style: BorderStyle::Plain,
            status_color: StatusColor::White,
            enable_colors: false,
            enable_bold: false,
        }
    }

    /// Create a high-contrast config
    #[must_use]
    pub fn high_contrast() -> Self {
        Self {
            border_style: BorderStyle::Double,
            status_color: StatusColor::Yellow,
            enable_colors: true,
            enable_bold: true,
        }
    }

    /// Get the border style
    #[must_use]
    pub fn border_style(&self) -> Style {
        let color = match self.border_style {
            BorderStyle::Plain => Color::White,
            BorderStyle::Rounded => Color::Cyan,
            BorderStyle::Double => Color::Blue,
            BorderStyle::Thick => Color::Green,
        };

        let mut style = Style::default().fg(color);

        if self.enable_bold {
            style = style.add_modifier(Modifier::BOLD);
        }

        style
    }

    /// Get the status color
    #[must_use]
    pub fn status_color(&self) -> Color {
        if !self.enable_colors {
            return Color::White;
        }

        match self.status_color {
            StatusColor::White => Color::White,
            StatusColor::Cyan => Color::Cyan,
            StatusColor::Yellow => Color::Yellow,
            StatusColor::Green => Color::Green,
            StatusColor::Magenta => Color::Magenta,
        }
    }
}

/// Border style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BorderStyle {
    /// Plain borders
    Plain,
    /// Rounded borders
    Rounded,
    /// Double borders
    Double,
    /// Thick borders
    Thick,
}

impl Default for BorderStyle {
    fn default() -> Self {
        Self::Rounded
    }
}

/// Status color
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatusColor {
    /// White
    White,
    /// Cyan
    Cyan,
    /// Yellow
    Yellow,
    /// Green
    Green,
    /// Magenta
    Magenta,
}

impl Default for StatusColor {
    fn default() -> Self {
        Self::Cyan
    }
}

/// Render-related errors
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RenderError {
    /// IO error
    #[error("IO error: {0}")]
    Io(String),
    /// Terminal error
    #[error("terminal error")]
    Terminal,
    /// Invalid area
    #[error("invalid render area")]
    InvalidArea,
}

impl From<io::Error> for RenderError {
    fn from(err: io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

/// Create a new terminal
///
/// # Errors
///
/// Returns error if terminal creation fails
pub fn create_terminal() -> Result<Terminal<CrosstermBackend<std::io::Stdout>>, RenderError> {
    let backend = CrosstermBackend::new(std::io::stdout());
    Terminal::new(backend).map_err(|e| RenderError::Io(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_new() {
        let config = RenderConfig::default();
        let renderer = Renderer::new(config.clone());
        assert_eq!(renderer.config(), &config);
        assert_eq!(renderer.frame_count(), 0);
    }

    #[test]
    fn test_renderer_with_config() {
        let config = RenderConfig::minimal();
        let renderer = Renderer::new(RenderConfig::default()).with_config(config.clone());
        assert_eq!(renderer.config(), &config);
    }

    #[test]
    fn test_renderer_tick() {
        let renderer = Renderer::new(RenderConfig::default());
        assert_eq!(renderer.frame_count(), 0);

        let mut renderer = renderer;
        renderer.tick();
        assert_eq!(renderer.frame_count(), 1);

        renderer.tick();
        assert_eq!(renderer.frame_count(), 2);
    }

    #[test]
    fn test_level_color() {
        let renderer = Renderer::new(RenderConfig::default());

        assert_eq!(renderer.level_color(0), Color::DarkGray);
        assert_eq!(renderer.level_color(25), Color::Blue);
        assert_eq!(renderer.level_color(75), Color::Cyan);
        assert_eq!(renderer.level_color(125), Color::Green);
        assert_eq!(renderer.level_color(175), Color::Yellow);
        assert_eq!(renderer.level_color(250), Color::Red);
    }

    #[test]
    fn test_render_config_default() {
        let config = RenderConfig::default();
        assert_eq!(config.border_style, BorderStyle::Rounded);
        assert_eq!(config.status_color, StatusColor::Cyan);
        assert!(config.enable_colors);
        assert!(config.enable_bold);
    }

    #[test]
    fn test_render_config_minimal() {
        let config = RenderConfig::minimal();
        assert_eq!(config.border_style, BorderStyle::Plain);
        assert_eq!(config.status_color, StatusColor::White);
        assert!(!config.enable_colors);
        assert!(!config.enable_bold);
    }

    #[test]
    fn test_render_config_high_contrast() {
        let config = RenderConfig::high_contrast();
        assert_eq!(config.border_style, BorderStyle::Double);
        assert_eq!(config.status_color, StatusColor::Yellow);
        assert!(config.enable_colors);
        assert!(config.enable_bold);
    }

    #[test]
    fn test_render_config_border_style() {
        let config = RenderConfig::default();
        let style = config.border_style();
        // Default is Rounded which maps to Cyan
        assert_eq!(style.fg.unwrap(), Color::Cyan);
    }

    #[test]
    fn test_render_config_status_color() {
        let config = RenderConfig::default();
        // Default is Cyan
        assert_eq!(config.status_color(), Color::Cyan);
    }

    #[test]
    fn test_render_config_status_color_no_colors() {
        let config = RenderConfig::minimal();
        // Minimal config has no colors
        assert_eq!(config.status_color(), Color::White);
    }

    #[test]
    fn test_border_style_default() {
        assert_eq!(BorderStyle::default(), BorderStyle::Rounded);
    }

    #[test]
    fn test_status_color_default() {
        assert_eq!(StatusColor::default(), StatusColor::Cyan);
    }

    #[test]
    fn test_border_style_copy() {
        let style = BorderStyle::Double;
        assert_eq!(style, BorderStyle::Double);
    }

    #[test]
    fn test_status_color_copy() {
        let color = StatusColor::Yellow;
        assert_eq!(color, StatusColor::Yellow);
    }

    #[test]
    fn test_render_error_io() {
        let error = RenderError::Io("test error".to_string());
        assert!(error.to_string().contains("test error"));
    }

    #[test]
    fn test_render_error_terminal() {
        let error = RenderError::Terminal;
        assert!(error.to_string().contains("terminal"));
    }

    #[test]
    fn test_render_error_invalid_area() {
        let error = RenderError::InvalidArea;
        assert!(error.to_string().contains("invalid"));
    }

    #[test]
    fn test_render_error_from_io() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "test");
        let render_error: RenderError = io_error.into();
        assert!(matches!(render_error, RenderError::Io(_)));
    }
}
