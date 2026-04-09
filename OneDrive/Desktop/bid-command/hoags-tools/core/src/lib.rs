//! hoags_core — shared kernel for all Hoags tools.
//! PDF parsing, context loading, DAVA memory, event bus, connector agents.

pub mod context;
pub mod memory;
pub mod bus;
pub mod connectors;

pub use context::{load_context_file, resolve_key};
pub use memory::FieldMemory;
pub use bus::EventBus;
pub use connectors::run_all_connectors;
