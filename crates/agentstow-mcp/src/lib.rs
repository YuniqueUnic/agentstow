mod adapter;
mod api;
mod parse;
mod snippet;
mod types;

pub use types::{McpDryRunCheck, McpDryRunCheckStatus, McpSnippetFormat, McpTargetAdapter};

pub struct Mcp;

#[cfg(test)]
mod tests;
