//! CATHEDRAL.FABRIC Policy System
//!
//! Policy language and compiler for capability enforcement.
//! All decisions produce proof objects that are logged.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod lang;
pub mod compiler;
pub mod proof;
pub mod matcher;
pub mod redact;

pub use lang::{PolicyParser, PolicyAst, PolicyExpr};
pub use compiler::{PolicyCompiler, CompiledPolicy, PolicyError};
pub use proof::{DecisionProof, ProofKind, ProofField};
pub use matcher::{Matcher, MatchContext, MatchResult};
pub use redact::{Redactor, RedactionRule, RedactedView};
