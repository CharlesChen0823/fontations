//! High level interface to font metadata.

pub mod metrics;
pub mod strings;

mod provider;

pub use provider::MetadataProvider;