//! TUI layout management for screen areas and constraints.

use ratatui::layout::{Constraint, Direction, Rect};
use serde::{Deserialize, Serialize};

/// Layout manager for calculating screen areas
#[derive(Debug, Clone)]
pub struct Layout {
    /// Main area percentage (0-100)
    main_percent: u16,
    /// Status bar height
    status_height: u16,
    /// Help area percentage (0-100)
    help_percent: u16,
}

impl Layout {
    /// Create a new layout
    #[must_use]
    pub fn new() -> Self {
        Self {
            main_percent: 80,
            status_height: 3,
            help_percent: 20,
        }
    }

    /// Set main area percentage
    #[must_use]
    pub fn with_main_percent(mut self, percent: u16) -> Self {
        self.main_percent = percent.min(100);
        self
    }

    /// Set status bar height
    #[must_use]
    pub fn with_status_height(mut self, height: u16) -> Self {
        self.status_height = height;
        self
    }

    /// Calculate layout areas for a given terminal size
    #[must_use]
    pub fn calculate(&self, size: Rect) -> CalculatedLayout {
        // Split vertically: main area + status bar
        let total_height = size.height;
        let status_height = self.status_height.min(total_height.saturating_sub(1));
        let main_height = total_height.saturating_sub(status_height);

        let main_area = Rect {
            x: size.x,
            y: size.y,
            width: size.width,
            height: main_height,
        };

        let status_area = Rect {
            x: size.x,
            y: size.y + main_height,
            width: size.width,
            height: status_height,
        };

        CalculatedLayout {
            main_area,
            status_area,
        }
    }

    /// Calculate split layout with sidebar
    #[must_use]
    pub fn calculate_split(&self, size: Rect, sidebar_percent: u16) -> SplitLayout {
        // Split horizontally: sidebar + main
        let total_width = size.width;
        let sidebar_width = total_width * sidebar_percent.min(100) / 100;

        let sidebar_area = Rect {
            x: size.x,
            y: size.y,
            width: sidebar_width,
            height: size.height,
        };

        let main_area = Rect {
            x: size.x + sidebar_width,
            y: size.y,
            width: total_width.saturating_sub(sidebar_width),
            height: size.height,
        };

        SplitLayout {
            sidebar_area,
            main_area,
        }
    }

    /// Calculate three-column layout
    #[must_use]
    pub fn calculate_triple(&self, size: Rect) -> TripleLayout {
        let total_width = size.width;
        let col_width = total_width / 3;

        let left_area = Rect {
            x: size.x,
            y: size.y,
            width: col_width,
            height: size.height,
        };

        let center_area = Rect {
            x: size.x + col_width,
            y: size.y,
            width: col_width,
            height: size.height,
        };

        let right_area = Rect {
            x: size.x + 2 * col_width,
            y: size.y,
            width: total_width.saturating_sub(2 * col_width),
            height: size.height,
        };

        TripleLayout {
            left_area,
            center_area,
            right_area,
        }
    }
}

impl Default for Layout {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculated layout with main and status areas
#[derive(Debug, Clone, Copy)]
pub struct CalculatedLayout {
    /// Main content area
    pub main_area: Rect,
    /// Status bar area
    pub status_area: Rect,
}

impl CalculatedLayout {
    /// Get the main area
    #[must_use]
    pub fn main_area(&self) -> Rect {
        self.main_area
    }

    /// Get the status area
    #[must_use]
    pub fn status_area(&self) -> Rect {
        self.status_area
    }
}

/// Split layout with sidebar
#[derive(Debug, Clone, Copy)]
pub struct SplitLayout {
    /// Sidebar area
    pub sidebar_area: Rect,
    /// Main content area
    pub main_area: Rect,
}

/// Three-column layout
#[derive(Debug, Clone, Copy)]
pub struct TripleLayout {
    /// Left column area
    pub left_area: Rect,
    /// Center column area
    pub center_area: Rect,
    /// Right column area
    pub right_area: Rect,
}

/// A layout area with metadata
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutArea {
    /// X position
    pub x: u16,
    /// Y position
    pub y: u16,
    /// Width
    pub width: u16,
    /// Height
    pub height: u16,
}

impl LayoutArea {
    /// Create a new layout area
    #[must_use]
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self { x, y, width, height }
    }

    /// Get area as a percentage of terminal size
    #[must_use]
    pub fn as_percent(&self, term_width: u16, term_height: u16) -> AreaPercent {
        AreaPercent {
            x_percent: if term_width > 0 {
                (self.x as u32 * 100 / term_width as u32) as u16
            } else {
                0
            },
            y_percent: if term_height > 0 {
                (self.y as u32 * 100 / term_height as u32) as u16
            } else {
                0
            },
            width_percent: if term_width > 0 {
                (self.width as u32 * 100 / term_width as u32) as u16
            } else {
                0
            },
            height_percent: if term_height > 0 {
                (self.height as u32 * 100 / term_height as u32) as u16
            } else {
                0
            },
        }
    }

    /// Check if area is valid (non-zero width and height)
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.width > 0 && self.height > 0
    }

    /// Get area size in characters
    #[must_use]
    pub fn size(&self) -> usize {
        self.width as usize * self.height as usize
    }
}

impl From<Rect> for LayoutArea {
    fn from(rect: Rect) -> Self {
        Self {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
        }
    }
}

impl From<LayoutArea> for Rect {
    fn from(area: LayoutArea) -> Self {
        Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: area.height,
        }
    }
}

/// Area expressed as percentages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AreaPercent {
    /// X position as percentage
    pub x_percent: u16,
    /// Y position as percentage
    pub y_percent: u16,
    /// Width as percentage
    pub width_percent: u16,
    /// Height as percentage
    pub height_percent: u16,
}

/// Layout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    /// Main area percentage
    pub main_percent: u16,
    /// Status bar height
    pub status_height: u16,
    /// Minimum width for sidebar
    pub min_sidebar_width: u16,
    /// Minimum height for content
    pub min_content_height: u16,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            main_percent: 80,
            status_height: 3,
            min_sidebar_width: 20,
            min_content_height: 10,
        }
    }
}

impl LayoutConfig {
    /// Create from percent values
    #[must_use]
    pub fn from_percent(main: u16, status: u16) -> Self {
        Self {
            main_percent: main.min(100),
            status_height: status,
            ..Self::default()
        }
    }

    /// Validate configuration
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.main_percent <= 100
            && self.status_height > 0
            && self.min_sidebar_width > 0
            && self.min_content_height > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_new() {
        let layout = Layout::new();
        assert_eq!(layout.main_percent, 80);
        assert_eq!(layout.status_height, 3);
    }

    #[test]
    fn test_layout_with_main_percent() {
        let layout = Layout::new().with_main_percent(90);
        assert_eq!(layout.main_percent, 90);
    }

    #[test]
    fn test_layout_with_main_percent_clamped() {
        let layout = Layout::new().with_main_percent(150);
        assert_eq!(layout.main_percent, 100);
    }

    #[test]
    fn test_layout_with_status_height() {
        let layout = Layout::new().with_status_height(5);
        assert_eq!(layout.status_height, 5);
    }

    #[test]
    fn test_layout_calculate() {
        let layout = Layout::new();
        let size = Rect::new(0, 0, 100, 30);
        let calculated = layout.calculate(size);

        assert_eq!(calculated.main_area.x, 0);
        assert_eq!(calculated.main_area.y, 0);
        assert_eq!(calculated.main_area.width, 100);
        assert_eq!(calculated.main_area.height, 27);

        assert_eq!(calculated.status_area.y, 27);
        assert_eq!(calculated.status_area.height, 3);
    }

    #[test]
    fn test_layout_calculate_split() {
        let layout = Layout::new();
        let size = Rect::new(0, 0, 100, 30);
        let split = layout.calculate_split(size, 25);

        assert_eq!(split.sidebar_area.width, 25);
        assert_eq!(split.main_area.width, 75);
    }

    #[test]
    fn test_layout_calculate_triple() {
        let layout = Layout::new();
        let size = Rect::new(0, 0, 99, 30);
        let triple = layout.calculate_triple(size);

        assert_eq!(triple.left_area.width, 33);
        assert_eq!(triple.center_area.width, 33);
        assert_eq!(triple.right_area.width, 33);
    }

    #[test]
    fn test_layout_area_new() {
        let area = LayoutArea::new(10, 20, 30, 40);
        assert_eq!(area.x, 10);
        assert_eq!(area.y, 20);
        assert_eq!(area.width, 30);
        assert_eq!(area.height, 40);
    }

    #[test]
    fn test_layout_area_as_percent() {
        let area = LayoutArea::new(10, 10, 50, 25);
        let percent = area.as_percent(100, 100);

        assert_eq!(percent.x_percent, 10);
        assert_eq!(percent.y_percent, 10);
        assert_eq!(percent.width_percent, 50);
        assert_eq!(percent.height_percent, 25);
    }

    #[test]
    fn test_layout_area_is_valid() {
        let valid = LayoutArea::new(0, 0, 10, 10);
        assert!(valid.is_valid());

        let invalid_width = LayoutArea::new(0, 0, 0, 10);
        assert!(!invalid_width.is_valid());

        let invalid_height = LayoutArea::new(0, 0, 10, 0);
        assert!(!invalid_height.is_valid());
    }

    #[test]
    fn test_layout_area_size() {
        let area = LayoutArea::new(0, 0, 10, 20);
        assert_eq!(area.size(), 200);
    }

    #[test]
    fn test_layout_area_from_rect() {
        let rect = Rect::new(5, 10, 15, 20);
        let area = LayoutArea::from(rect);

        assert_eq!(area.x, 5);
        assert_eq!(area.y, 10);
        assert_eq!(area.width, 15);
        assert_eq!(area.height, 20);
    }

    #[test]
    fn test_rect_from_layout_area() {
        let area = LayoutArea::new(5, 10, 15, 20);
        let rect: Rect = area.into();

        assert_eq!(rect.x, 5);
        assert_eq!(rect.y, 10);
        assert_eq!(rect.width, 15);
        assert_eq!(rect.height, 20);
    }

    #[test]
    fn test_layout_config_default() {
        let config = LayoutConfig::default();
        assert_eq!(config.main_percent, 80);
        assert_eq!(config.status_height, 3);
        assert_eq!(config.min_sidebar_width, 20);
        assert_eq!(config.min_content_height, 10);
    }

    #[test]
    fn test_layout_config_from_percent() {
        let config = LayoutConfig::from_percent(90, 5);
        assert_eq!(config.main_percent, 90);
        assert_eq!(config.status_height, 5);
    }

    #[test]
    fn test_layout_config_is_valid() {
        let config = LayoutConfig::default();
        assert!(config.is_valid());
    }

    #[test]
    fn test_layout_config_invalid_main_percent() {
        let mut config = LayoutConfig::default();
        config.main_percent = 150;
        assert!(!config.is_valid());
    }

    #[test]
    fn test_calculated_layout_main_area() {
        let layout = CalculatedLayout {
            main_area: Rect::new(0, 0, 100, 27),
            status_area: Rect::new(0, 27, 100, 3),
        };
        let main = layout.main_area();
        assert_eq!(main.width, 100);
        assert_eq!(main.height, 27);
    }

    #[test]
    fn test_calculated_layout_status_area() {
        let layout = CalculatedLayout {
            main_area: Rect::new(0, 0, 100, 27),
            status_area: Rect::new(0, 27, 100, 3),
        };
        let status = layout.status_area();
        assert_eq!(status.width, 100);
        assert_eq!(status.height, 3);
    }

    #[test]
    fn test_area_percent_fields() {
        let percent = AreaPercent {
            x_percent: 10,
            y_percent: 20,
            width_percent: 50,
            height_percent: 30,
        };
        assert_eq!(percent.x_percent, 10);
        assert_eq!(percent.y_percent, 20);
        assert_eq!(percent.width_percent, 50);
        assert_eq!(percent.height_percent, 30);
    }
}
