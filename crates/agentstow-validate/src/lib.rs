use agentstow_core::{AgentStowError, Result, ValidateAs};
use agentstow_manifest::ArtifactDef;
use tracing::instrument;

pub struct Validator;

impl Validator {
    #[instrument(skip_all, fields(validate_as=?artifact.validate_as))]
    pub fn validate_rendered_file(artifact: &ArtifactDef, bytes: &[u8]) -> Result<()> {
        match artifact.validate_as {
            ValidateAs::None | ValidateAs::Markdown => Ok(()),
            ValidateAs::Json => {
                serde_json::from_slice::<serde_json::Value>(bytes).map_err(|e| {
                    AgentStowError::Validate {
                        message: format!("JSON 解析失败: {e}").into(),
                    }
                })?;
                Ok(())
            }
            ValidateAs::Toml => {
                let s = std::str::from_utf8(bytes).map_err(|e| AgentStowError::Validate {
                    message: format!("TOML 不是 UTF-8 文本: {e}").into(),
                })?;
                toml::from_str::<toml::Value>(s).map_err(|e| AgentStowError::Validate {
                    message: format!("TOML 解析失败: {e}").into(),
                })?;
                Ok(())
            }
            ValidateAs::Shell => {
                // v1 最小校验：必须是 UTF-8 且不包含 NUL。
                let s = std::str::from_utf8(bytes).map_err(|e| AgentStowError::Validate {
                    message: format!("shell 输出不是 UTF-8 文本: {e}").into(),
                })?;
                if s.as_bytes().contains(&0) {
                    return Err(AgentStowError::Validate {
                        message: "shell 输出包含 NUL 字节".into(),
                    });
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests;
