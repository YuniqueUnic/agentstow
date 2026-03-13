use serde::{Deserialize, Serialize};
use ts_rs::{Config, TS};

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ApiError {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct HealthResponse {
    pub ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ManifestResponse {
    pub workspace_root: String,
    pub profiles: Vec<String>,
    pub artifacts: Vec<String>,
    pub targets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RenderResponse {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum InstallMethodResponse {
    Symlink,
    Junction,
    Copy,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LinkRecordResponse {
    pub artifact_id: String,
    pub profile: String,
    pub target_path: String,
    pub method: InstallMethodResponse,
    pub rendered_path: Option<String>,
    pub blake3: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LinkStatusResponseItem {
    pub artifact_id: String,
    pub profile: String,
    pub target_path: String,
    pub method: InstallMethodResponse,
    pub ok: bool,
    pub message: String,
}

pub fn export_bindings() -> Result<(), ts_rs::ExportError> {
    let config = Config::from_env();
    ApiError::export_all(&config)?;
    HealthResponse::export_all(&config)?;
    ManifestResponse::export_all(&config)?;
    RenderResponse::export_all(&config)?;
    InstallMethodResponse::export_all(&config)?;
    LinkRecordResponse::export_all(&config)?;
    LinkStatusResponseItem::export_all(&config)?;
    Ok(())
}

#[cfg(test)]
mod tests;
