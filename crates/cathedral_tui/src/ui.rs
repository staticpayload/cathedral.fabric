//! TUI app for viewing traces and audit logs.

use crate::input::{InputHandler, InputEvent, InputError};
use crate::layout::{Layout, CalculatedLayout};
use crate::renderer::{Renderer, RenderConfig};
use crate::view::{TimelineView, DagView, WorkerView, ProvenanceView, View};
use cathedral_core::{EventId, RunId};
use cathedral_log::EventStream;
use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        event::{DisableMouseCapture, EnableMouseCapture},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    Frame,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// TUI application state
pub struct TuiApp {
    /// Event stream
    stream: Arc<RwLock<EventStream>>,
    /// Current view mode
    view_mode: ViewMode,
    /// Timeline view
    timeline: TimelineView,
    /// DAG view
    dag: DagView,
    /// Worker view
    worker: WorkerView,
    /// Provenance view
    provenance: ProvenanceView,
    /// Input handler
    input: InputHandler,
    /// Renderer
    renderer: Renderer,
    /// Layout
    layout: Layout,
    /// Should quit
    should_quit: bool,
    /// Current selection
    selection: Selection,
    /// Status message
    status: String,
}

/// View mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Timeline view
    Timeline,
    /// DAG view
    Dag,
    /// Worker view
    Worker,
    /// Provenance view
    Provenance,
    /// Help
    Help,
}

/// Selection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    /// Selected event
    pub event_id: Option<EventId>,
    /// Selected run
    pub run_id: Option<RunId>,
    /// Selected line in current view
    pub line: usize,
    /// Scroll offset
    pub scroll: usize,
}

impl Default for Selection {
    fn default() -> Self {
        Self {
            event_id: None,
            run_id: None,
            line: 0,
            scroll: 0,
        }
    }
}

impl Default for TuiApp {
    fn default() -> Self {
        Self {
            stream: Arc::new(RwLock::new(EventStream::new(Vec::new()))),
            view_mode: ViewMode::Timeline,
            timeline: TimelineView::new(),
            dag: DagView::new(),
            worker: WorkerView::new(),
            provenance: ProvenanceView::new(),
            input: InputHandler::new(),
            renderer: Renderer::new(RenderConfig::default()),
            layout: Layout::new(),
            should_quit: false,
            selection: Selection::default(),
            status: "Ready".to_string(),
        }
    }
}

impl TuiApp {
    /// Create new TUI app
    ///
    /// # Errors
    ///
    /// Returns error if log loading fails
    pub fn new(input: &str) -> Result<Self, TuiError> {
        // Try to load the event stream from file
        let stream = if std::path::Path::new(input).exists() {
            // For now, create an empty stream
            // TODO: Implement loading from file
            Arc::new(RwLock::new(EventStream::new(Vec::new())))
        } else {
            Arc::new(RwLock::new(EventStream::new(Vec::new())))
        };

        let event_count = 0; // TODO: Get from stream
        let mut app = Self::default();
        app.stream = stream;
        app.status = format!("Loaded {} events", event_count);

        Ok(app)
    }

    /// Run the TUI
    ///
    /// # Errors
    ///
    /// Returns error if terminal setup or execution fails
    pub fn run(&mut self) -> Result<(), TuiError> {
        enable_raw_mode()
            .map_err(|e| TuiError::Terminal(e.to_string()))?;
        execute!(
            std::io::stdout(),
            EnterAlternateScreen,
            EnableMouseCapture
        ).map_err(|e| TuiError::Terminal(e.to_string()))?;

        let backend = CrosstermBackend::new(std::io::stdout());
        let mut terminal = ratatui::Terminal::new(backend)
            .map_err(|e| TuiError::Terminal(e.to_string()))?;

        let result = self.run_inner(&mut terminal);

        disable_raw_mode()
            .map_err(|e| TuiError::Terminal(e.to_string()))?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        ).map_err(|e| TuiError::Terminal(e.to_string()))?;

        result
    }

    fn run_inner(&mut self, terminal: &mut ratatui::Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<(), TuiError> {
        let mut last_tick = std::time::Instant::now();
        let tick_rate = Duration::from_millis(250);

        loop {
            terminal.draw(|f| self.draw(f))
                .map_err(|e| TuiError::Render(e.to_string()))?;

            let timeout = tick_rate
                .saturating_sub(last_tick.elapsed())
                .max(Duration::from_millis(100));

            if crossterm::event::poll(timeout)
                .map_err(|e| TuiError::Io(e.to_string()))? {
                if let Some(event) = self.input.next_event()
                    .map_err(|e| TuiError::Terminal(e.to_string()))? {
                    self.handle_event(event);
                }
            }

            last_tick = std::time::Instant::now();

            if self.should_quit {
                return Ok(());
            }
        }
    }

    fn draw(&self, f: &mut Frame) {
        let area = f.area();

        let layout = self.layout.calculate(area);
        self.render_view(f, layout);
        self.render_status(f, layout);
        self.render_help(f, layout);
    }

    fn render_view(&self, f: &mut Frame, layout: CalculatedLayout) {
        let main_area = layout.main_area;

        match self.view_mode {
            ViewMode::Timeline => {
                // For now, render empty views since we don't have the event data
                self.render_empty_view(f, main_area, "Timeline");
            }
            ViewMode::Dag => {
                self.render_empty_view(f, main_area, "Execution DAG");
            }
            ViewMode::Worker => {
                self.render_empty_view(f, main_area, "Workers");
            }
            ViewMode::Provenance => {
                self.render_empty_view(f, main_area, "Provenance");
            }
            ViewMode::Help => {
                self.render_help_screen(f, main_area);
            }
        }
    }

    fn render_empty_view(&self, f: &mut Frame, area: ratatui::layout::Rect, title: &str) {
        use ratatui::{widgets::Paragraph, widgets::Wrap};

        let block = ratatui::widgets::Block::default()
            .title(format!(" {} ", title))
            .borders(ratatui::widgets::Borders::ALL);

        let paragraph = Paragraph::new("No data loaded")
            .block(block)
            .wrap(Wrap { trim: false });

        f.render_widget(paragraph, area);
    }

    fn render_status(&self, f: &mut Frame, layout: CalculatedLayout) {
        use ratatui::{widgets::Paragraph, widgets::Wrap};

        let status_area = layout.status_area;
        let status_text = format!(
            " {} | {} | {} | {}",
            self.view_mode_short(),
            self.selection_info(),
            self.status,
            "Press ? for help"
        );

        let status = Paragraph::new(status_text)
            .wrap(Wrap { trim: false });

        f.render_widget(status, status_area);
    }

    fn render_help(&self, _f: &mut Frame, _layout: CalculatedLayout) {
        // Only show key hints in a small area at bottom
    }

    fn render_help_screen(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        use ratatui::{
            layout::{Alignment, Constraint, Direction, Layout},
            style::{Color, Modifier, Style},
            text::{Line, Span},
            widgets::{Block, Borders, Paragraph, Wrap},
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Length(1), Constraint::Min(0)].as_ref())
            .split(area);

        let title = Paragraph::new("Key Bindings")
            .alignment(Alignment::Center)
            .style(Style::default().add_modifier(Modifier::BOLD));

        let help_text = vec![
            Line::from("Navigation:"),
            Line::from("  j/↓    - Move down"),
            Line::from("  k/↑    - Move up"),
            Line::from("  g      - Go to top"),
            Line::from("  G      - Go to bottom"),
            Line::from(""),
            Line::from("Views:"),
            Line::from("  1      - Timeline view"),
            Line::from("  2      - DAG view"),
            Line::from("  3      - Worker view"),
            Line::from("  4      - Provenance view"),
            Line::from(""),
            Line::from("Actions:"),
            Line::from("  Enter  - View details"),
            Line::from("  /      - Search"),
            Line::from("  n      - Next search result"),
            Line::from("  p      - Previous search result"),
            Line::from("  q      - Quit"),
            Line::from("  ?      - Help"),
        ];

        let help = Paragraph::new(help_text)
            .block(Block::default().borders(Borders::ALL).title(" Help "));
        f.render_widget(title, chunks[0]);
        f.render_widget(help, chunks[1]);
    }

    fn view_mode_short(&self) -> &str {
        match self.view_mode {
            ViewMode::Timeline => "Timeline",
            ViewMode::Dag => "DAG",
            ViewMode::Worker => "Worker",
            ViewMode::Provenance => "Provenance",
            ViewMode::Help => "Help",
        }
    }

    fn selection_info(&self) -> String {
        if let Some(event_id) = self.selection.event_id {
            format!("Event:{}", event_id)
        } else if let Some(run_id) = self.selection.run_id {
            format!("Run:{}", run_id)
        } else {
            format!("Line {}", self.selection.line)
        }
    }

    fn handle_event(&mut self, event: InputEvent) {
        match event {
            InputEvent::Quit => {
                self.should_quit = true;
            }
            InputEvent::Help => {
                self.view_mode = ViewMode::Help;
            }
            InputEvent::ViewTimeline => {
                self.view_mode = ViewMode::Timeline;
                self.status = "Timeline view".to_string();
            }
            InputEvent::ViewDag => {
                self.view_mode = ViewMode::Dag;
                self.status = "DAG view".to_string();
            }
            InputEvent::ViewWorker => {
                self.view_mode = ViewMode::Worker;
                self.status = "Worker view".to_string();
            }
            InputEvent::ViewProvenance => {
                self.view_mode = ViewMode::Provenance;
                self.status = "Provenance view".to_string();
            }
            InputEvent::Down => {
                self.selection.line += 1;
                self.update_scroll();
            }
            InputEvent::Up => {
                if self.selection.line > 0 {
                    self.selection.line -= 1;
                    self.update_scroll();
                }
            }
            InputEvent::GoTop => {
                self.selection.line = 0;
                self.selection.scroll = 0;
            }
            InputEvent::GoBottom => {
                self.selection.line = self.max_line();
                self.update_scroll();
            }
            InputEvent::Select => {
                self.status = "Selected details".to_string();
            }
            InputEvent::Search => {
                self.status = "Search not yet implemented".to_string();
            }
            _ => {}
        }
    }

    fn update_scroll(&mut self) {
        let max_scroll = self.selection.line.saturating_sub(10);
        if self.selection.scroll > max_scroll {
            self.selection.scroll = max_scroll;
        } else if self.selection.line < self.selection.scroll {
            self.selection.scroll = self.selection.line;
        }
    }

    fn max_line(&self) -> usize {
        match self.view_mode {
            ViewMode::Timeline => self.timeline.item_count(),
            ViewMode::Dag => self.dag.item_count(),
            ViewMode::Worker => self.worker.item_count(),
            ViewMode::Provenance => self.provenance.item_count(),
            ViewMode::Help => 0,
        }
        .saturating_sub(1)
    }
}

/// TUI configuration
#[derive(Debug, Clone)]
pub struct TuiConfig {
    /// Tick rate in milliseconds
    pub tick_rate_ms: u64,
    /// Max log entries to display
    pub max_entries: usize,
    /// Color scheme
    pub color_scheme: ColorScheme,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            tick_rate_ms: 250,
            max_entries: 1000,
            color_scheme: ColorScheme::Default,
        }
    }
}

/// Color scheme
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorScheme {
    /// Default colors
    Default,
    /// High contrast
    HighContrast,
    /// Dark mode
    Dark,
    /// Light mode
    Light,
}

/// TUI errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum TuiError {
    /// Terminal error
    #[error("terminal error: {0}")]
    Terminal(String),
    /// IO error
    #[error("io error: {0}")]
    Io(String),
    /// Log error
    #[error("log error: {0}")]
    Log(String),
    /// Render error
    #[error("render error: {0}")]
    Render(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tui_config_default() {
        let config = TuiConfig::default();
        assert_eq!(config.tick_rate_ms, 250);
        assert_eq!(config.max_entries, 1000);
    }

    #[test]
    fn test_selection_default() {
        let sel = Selection::default();
        assert_eq!(sel.line, 0);
        assert_eq!(sel.scroll, 0);
        assert!(sel.event_id.is_none());
    }

    #[test]
    fn test_view_mode_copy() {
        let mode = ViewMode::Timeline;
        assert_eq!(mode, ViewMode::Timeline);
    }

    #[test]
    fn test_color_scheme_copy() {
        let scheme = ColorScheme::Dark;
        assert_eq!(scheme, ColorScheme::Dark);
    }

    #[test]
    fn test_tui_error_messages() {
        let err = TuiError::Terminal("test".to_string());
        assert!(err.to_string().contains("terminal"));
    }
}
