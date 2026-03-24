use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{ArtifactKindResponse, InstallMethodResponse};

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

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LinkPlanRequest {
    /// 可选：为空则对所有 targets 生效
    pub targets: Vec<String>,
    /// target 未声明 profile 时可用（等价于 CLI 的全局 --profile）
    pub default_profile: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LinkApplyRequest {
    /// 可选：为空则对所有 targets 生效
    pub targets: Vec<String>,
    /// target 未声明 profile 时可用（等价于 CLI 的全局 --profile）
    pub default_profile: Option<String>,
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LinkRepairRequest {
    /// 可选：为空则只修复不健康项（坏链扫描结果）
    pub targets: Vec<String>,
    /// target 未声明 profile 时可用（等价于 CLI 的全局 --profile）
    pub default_profile: Option<String>,
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum LinkDesiredInstallResponse {
    Symlink { source_path: String },
    Junction { source_path: String },
    Copy { blake3: String, bytes_len: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LinkPlanItemResponse {
    pub target: String,
    pub artifact_id: String,
    pub profile: String,
    pub artifact_kind: ArtifactKindResponse,
    pub method: InstallMethodResponse,
    pub target_path: String,
    pub desired: LinkDesiredInstallResponse,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum LinkOperationActionResponse {
    Planned,
    Applied,
    Repaired,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LinkOperationItemResponse {
    pub action: LinkOperationActionResponse,
    pub item: LinkPlanItemResponse,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LinkOperationResponse {
    pub items: Vec<LinkOperationItemResponse>,
}
