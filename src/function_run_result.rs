use crate::BytesContainer;
use serde::{Deserialize, Serialize};

pub(crate) const FUNCTION_LOG_LIMIT: usize = 1_000;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FunctionRunResult {
    pub name: String,
    pub size: u64,
    pub memory_usage: u64,
    pub instructions: u64,
    pub logs: String,
    pub input: BytesContainer,
    pub output: BytesContainer,
    #[serde(skip)]
    pub profile: Option<String>,
    #[serde(skip)]
    pub scale_factor: f64,
    pub success: bool,
}

impl FunctionRunResult {
    pub fn input_size(&self) -> usize {
        self.input.raw.len()
    }

    pub fn output_size(&self) -> usize {
        self.output.raw.len()
    }
}
