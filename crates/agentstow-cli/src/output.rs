use std::path::Path;

use agentstow_core::{AgentStowError, Result};
use agentstow_render::{RenderedDir, RenderedDirEntryKind};
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

pub fn write_rendered_dir(path: &Path, rendered: &RenderedDir) -> Result<()> {
    agentstow_core::ensure_parent_dir(path)?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let staging = tempfile::Builder::new()
        .prefix(".agentstow.render.")
        .tempdir_in(parent)
        .map_err(AgentStowError::from)?;

    fs_err::create_dir_all(staging.path()).map_err(AgentStowError::from)?;
    for entry in &rendered.entries {
        let entry_path = staging.path().join(&entry.relative_path);
        match entry.kind {
            RenderedDirEntryKind::Dir => {
                fs_err::create_dir_all(&entry_path).map_err(AgentStowError::from)?;
            }
            RenderedDirEntryKind::File => {
                agentstow_core::ensure_parent_dir(&entry_path)?;
                fs_err::write(&entry_path, &entry.bytes).map_err(AgentStowError::from)?;
            }
        }
    }

    if path.exists() {
        remove_existing(path)?;
    }
    fs_err::rename(staging.path(), path).map_err(AgentStowError::from)?;
    Ok(())
}

fn remove_existing(path: &Path) -> Result<()> {
    let meta = fs_err::symlink_metadata(path).map_err(AgentStowError::from)?;
    if meta.is_dir() && !meta.file_type().is_symlink() {
        fs_err::remove_dir_all(path).map_err(AgentStowError::from)?;
    } else {
        fs_err::remove_file(path).map_err(AgentStowError::from)?;
    }
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
