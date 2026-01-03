//! TUI views for traces, DAGs, and audit logs.

use cathedral_core::{EventId, RunId};
use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap, Table},
    Frame,
};
use ratatui::layout::Rect;

/// Trait for TUI views
pub trait View {
    /// Render the view
    fn render(&self, f: &mut Frame, area: Rect, selection: &crate::ui::Selection);

    /// Get item count for scrolling
    fn item_count(&self) -> usize;
}

/// Timeline view showing events chronologically
pub struct TimelineView {
    items: Vec<TimelineItem>,
}

impl TimelineView {
    /// Create new timeline view
    #[must_use]
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
        }
    }
}

impl Default for TimelineView {
    fn default() -> Self {
        Self::new()
    }
}

/// Timeline item
#[derive(Debug, Clone)]
pub struct TimelineItem {
    /// Tick
    pub tick: u64,
    /// Node ID
    pub node_id: String,
    /// Event kind
    pub kind: String,
    /// Detail
    pub detail: String,
}

impl View for TimelineView {
    fn render(&self, f: &mut Frame, area: Rect, selection: &crate::ui::Selection) {
        let title = Block::default()
            .title(" Timeline ")
            .borders(Borders::ALL);

        let items: Vec<ListItem> = self.items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let style = if i == selection.line {
                    Style::default().bg(Color::Blue).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(format!("{} | {:12} | {}", item.tick, item.node_id, item.kind))
                    .style(style)
            })
            .collect();

        let list = List::new(items)
            .block(title)
            .highlight_style(Style::default().add_modifier(Modifier::UNDERLINED));

        f.render_widget(list, area);
    }

    fn item_count(&self) -> usize {
        self.items.len()
    }
}

/// DAG view showing execution graph
pub struct DagView {
    nodes: Vec<DagNode>,
    edges: Vec<DagEdge>,
}

impl DagView {
    /// Create new DAG view
    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }
}

impl Default for DagView {
    fn default() -> Self {
        Self::new()
    }
}

/// DAG node
#[derive(Debug, Clone)]
pub struct DagNode {
    /// Node ID
    pub id: String,
    /// Label
    pub label: String,
    /// Status
    pub status: NodeStatus,
}

/// DAG edge
#[derive(Debug, Clone)]
pub struct DagEdge {
    /// From node
    pub from: String,
    /// To node
    pub to: String,
}

/// Node status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeStatus {
    /// Pending
    Pending,
    /// Running
    Running,
    /// Completed
    Completed,
    /// Failed
    Failed,
}

impl View for DagView {
    fn render(&self, f: &mut Frame, area: Rect, selection: &crate::ui::Selection) {
        let title = Block::default()
            .title(" Execution DAG ")
            .borders(Borders::ALL);

        let rows: Vec<Line> = self.nodes
            .iter()
            .enumerate()
            .map(|(i, node)| {
                let style = if i == selection.line {
                    Style::default().bg(Color::Blue).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let status_color = match node.status {
                    NodeStatus::Pending => Color::Yellow,
                    NodeStatus::Running => Color::Cyan,
                    NodeStatus::Completed => Color::Green,
                    NodeStatus::Failed => Color::Red,
                };
                Line::from(vec![
                    Span::raw(format!("{} ", node.id)),
                    Span::styled(format!("[{}]", node.label), Style::default().fg(status_color)),
                ])
                .style(style)
            })
            .collect();

        let paragraph = Paragraph::new(rows).block(title).wrap(Wrap { trim: false });
        f.render_widget(paragraph, area);
    }

    fn item_count(&self) -> usize {
        self.nodes.len()
    }
}

/// Worker view showing worker status
pub struct WorkerView {
    workers: Vec<WorkerStatus>,
}

impl WorkerView {
    /// Create new worker view
    #[must_use]
    pub fn new() -> Self {
        Self {
            workers: Vec::new(),
        }
    }
}

impl Default for WorkerView {
    fn default() -> Self {
        Self::new()
    }
}

/// Worker status
#[derive(Debug, Clone)]
pub struct WorkerStatus {
    /// Worker ID
    pub id: String,
    /// Status
    pub status: WorkerState,
    /// Tasks completed
    pub completed: usize,
    /// Tasks total
    pub total: usize,
}

/// Worker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerState {
    /// Idle
    Idle,
    /// Busy
    Busy,
    /// Offline
    Offline,
}

impl View for WorkerView {
    fn render(&self, f: &mut Frame, area: Rect, selection: &crate::ui::Selection) {
        let title = Block::default()
            .title(" Workers ")
            .borders(Borders::ALL);

        let rows: Vec<Line> = self.workers
            .iter()
            .enumerate()
            .map(|(i, worker)| {
                let style = if i == selection.line {
                    Style::default().bg(Color::Blue).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let status_color = match worker.status {
                    WorkerState::Idle => Color::Green,
                    WorkerState::Busy => Color::Yellow,
                    WorkerState::Offline => Color::Red,
                };
                Line::from(vec![
                    Span::raw(format!("{} ", worker.id)),
                    Span::styled(format!("{:?}", worker.status), Style::default().fg(status_color)),
                    Span::raw(format!(" ({}/{})", worker.completed, worker.total)),
                ])
                .style(style)
            })
            .collect();

        let paragraph = Paragraph::new(rows).block(title).wrap(Wrap { trim: false });
        f.render_widget(paragraph, area);
    }

    fn item_count(&self) -> usize {
        self.workers.len()
    }
}

/// Provenance view showing data lineage
pub struct ProvenanceView {
    entries: Vec<ProvenanceEntry>,
}

impl ProvenanceView {
    /// Create new provenance view
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl Default for ProvenanceView {
    fn default() -> Self {
        Self::new()
    }
}

/// Provenance entry
#[derive(Debug, Clone)]
pub struct ProvenanceEntry {
    /// Data ID
    pub data_id: String,
    /// Source
    pub source: String,
    /// Hash
    pub hash: String,
    /// Timestamp
    pub timestamp: String,
}

impl View for ProvenanceView {
    fn render(&self, f: &mut Frame, area: Rect, selection: &crate::ui::Selection) {
        let title = Block::default()
            .title(" Provenance ")
            .borders(Borders::ALL);

        let rows: Vec<Line> = self.entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let style = if i == selection.line {
                    Style::default().bg(Color::Blue).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                Line::from(vec![
                    Span::raw(format!("{} ", entry.data_id)),
                    Span::raw(format!("<- {} ", entry.source)),
                    Span::raw(format!("({})", entry.hash)),
                ])
                .style(style)
            })
            .collect();

        let paragraph = Paragraph::new(rows).block(title).wrap(Wrap { trim: false });
        f.render_widget(paragraph, area);
    }

    fn item_count(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeline_view_new() {
        let view = TimelineView::new();
        assert_eq!(view.items.len(), 0);
    }

    #[test]
    fn test_dag_view_new() {
        let view = DagView::new();
        assert_eq!(view.nodes.len(), 0);
    }

    #[test]
    fn test_worker_view_new() {
        let view = WorkerView::new();
        assert_eq!(view.workers.len(), 0);
    }

    #[test]
    fn test_provenance_view_new() {
        let view = ProvenanceView::new();
        assert_eq!(view.entries.len(), 0);
    }

    #[test]
    fn test_node_status_copy() {
        let status = NodeStatus::Completed;
        assert_eq!(status, NodeStatus::Completed);
    }

    #[test]
    fn test_worker_state_copy() {
        let state = WorkerState::Busy;
        assert_eq!(state, WorkerState::Busy);
    }

    #[test]
    fn test_timeline_item_clone() {
        let item = TimelineItem {
            tick: 1,
            node_id: "node1".to_string(),
            kind: "Test".to_string(),
            detail: "detail".to_string(),
        };
        let cloned = item.clone();
        assert_eq!(cloned.tick, 1);
    }
}
