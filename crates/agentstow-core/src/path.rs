use std::path::{Path, PathBuf};

use crate::{AgentStowError, Result};

pub fn absolutize(base_dir: &Path, p: &Path) -> PathBuf {
    if p.is_absolute() {
        return p.to_path_buf();
    }
    base_dir.join(p)
}

pub fn normalize_for_display(p: &Path) -> String {
    dunce::simplified(p).display().to_string()
}

pub fn ensure_parent_dir(path: &Path) -> Result<()> {
    let Some(parent) = path.parent() else {
        return Err(AgentStowError::InvalidArgs {
            message: "路径没有 parent，无法创建目录".into(),
        });
    };
    std::fs::create_dir_all(parent).map_err(AgentStowError::from)
}
