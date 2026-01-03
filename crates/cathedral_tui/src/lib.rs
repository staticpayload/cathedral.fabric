//! CATHEDRAL.FABRIC TUI
//!
//! Terminal UI for viewing traces, DAGs, and audit logs.
//! Deterministic rendering with stable ordering.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod ui;
pub mod view;
pub mod renderer;
pub mod input;
pub mod layout;

pub use ui::{TuiApp, TuiConfig, TuiError};
pub use view::{TimelineView, DagView, WorkerView, ProvenanceView};
pub use renderer::{Renderer, RenderConfig, RenderError};
pub use input::{InputHandler, InputEvent, KeyBinding};
pub use layout::{Layout, LayoutArea, LayoutConfig};
