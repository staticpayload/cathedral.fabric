//! CATHEDRAL.FABRIC Server
//!
//! HTTP API server for remote execution and cluster management.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod api;
pub mod auth;
pub mod handler;
pub mod middleware;

pub use api::{ApiServer, ServerConfig};
pub use auth::{Authenticator, AuthConfig, AuthError};
pub use handler::{Handler, HandlerError};
pub use middleware::{Middleware, MiddlewareStack};
