//! CATHEDRAL.FABRIC Planner
//!
//! DSL compiler that transforms workflow definitions into typed DAGs
//! with explicit resource contracts and capability requirements.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod dsl;
pub mod dag;
pub mod compiler;
pub mod resource;
pub mod validate;

pub use dsl::{parse, ParseError};
pub use compiler::Ast;
pub use dag::{Dag, Node, Edge, NodeKind};
pub use compiler::{Compiler, CompilerOutput, CompilerWarning};
pub use resource::{ResourceContract, ResourceBounds};
pub use validate::{Validator, ValidationError};
