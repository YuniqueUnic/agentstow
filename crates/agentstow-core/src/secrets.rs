use serde::{Deserialize, Serialize};

use crate::{AgentStowError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SecretBinding {
    /// ⚠️ 不推荐：会把明文写进 Git 真源。只用于测试或非敏感值。
    Literal { value: String },
    /// 从当前进程环境变量读取（推荐）。
    Env { var: String },
}

impl SecretBinding {
    pub fn resolve(&self) -> Result<String> {
        match self {
            Self::Literal { value } => Ok(value.clone()),
            Self::Env { var } => std::env::var(var).map_err(|_| AgentStowError::Validate {
                message: format!("缺少环境变量：{var}").into(),
            }),
        }
    }

    /// 用于“生成可提交配置文件”的场景：尽量避免把真实 secret 写进落地文件。
    ///
    /// - `Env { var }` -> `${VAR}`
    /// - `Literal { value }` -> 原值（⚠️ 不推荐）
    pub fn render_for_config(&self) -> String {
        match self {
            Self::Literal { value } => value.clone(),
            Self::Env { var } => format!("${{{var}}}"),
        }
    }
}
