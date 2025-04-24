pub mod bluejay_schema_analyzer;
pub mod container;
pub mod engine;
pub mod function_run_result;
pub mod scale_limits_analyzer;
use clap::ValueEnum;

pub use container::*;

/// Supported input encoding.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Default)]
pub enum Codec {
    #[default]
    /// JSON input.
    Json,
    /// Raw input, no validation, passed as-is.
    Raw,
    /// JSON input encoded as Messagepack.
    Messagepack,
}
