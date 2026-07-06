//! seine-engine: a forward-chaining rule engine implementing a bounded,
//! differentially-proven subset of Drools DRL semantics.
//!
//! All semantics are pinned against real Drools (9.44.0.Final) via the
//! differential harness in this repository; see DECISIONS.md for every
//! probe result the implementation relies on.

pub mod drl;
mod phreak;
pub mod engine;
mod queries;
mod rx;
pub mod store;

pub use engine::{Engine, EngineError, Firing, JustificationView, SupportView};
pub use queries::{QueryOutput, QueryVal};
pub use store::{FactId, FactView, FieldType, TypeSchema, Value};
