use std::path::Path;

use agentstow_core::{AgentStowError, Result};
use serde::Serialize;

pub fn init_tracing() -> Result<()> {
    use tracing_subscriber::EnvFilter;

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .with_target(false)
        .finish();

    // 避免在 tests 中多次 init 导致 panic
    let _ = tracing::subscriber::set_global_default(subscriber);
    Ok(())
}

pub fn print_json<T: Serialize>(value: &T) -> Result<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(value).map_err(|error| AgentStowError::Other(error.into()))?
    );
    Ok(())
}

pub fn write_bytes_file(path: &Path, bytes: &[u8]) -> Result<()> {
    agentstow_core::ensure_parent_dir(path)?;
    fs_err::write(path, bytes).map_err(AgentStowError::from)?;
    Ok(())
}

pub fn write_text_file(path: &Path, text: &str) -> Result<()> {
    agentstow_core::ensure_parent_dir(path)?;
    fs_err::write(path, text).map_err(AgentStowError::from)?;
    Ok(())
}

pub fn emit_error(json: bool, err: &AgentStowError) {
    if json {
        let payload = serde_json::json!({
            "error": err.to_string(),
            "exit_code": err.exit_code().as_i32(),
        });
        if print_json(&payload).is_ok() {
            return;
        }
    }

    eprintln!("{err}");
}
