use std::collections::BTreeMap;
use std::io::Write as _;

use agentstow_core::{AgentStowError, ProfileName, Result};
use agentstow_web_types::{ProfileDetailResponse, ProfileVarUpdateItemRequest};

use super::WorkspaceQueryService;

impl WorkspaceQueryService {
    pub(crate) fn update_profile_vars(
        &self,
        profile_name: &ProfileName,
        vars: &[ProfileVarUpdateItemRequest],
    ) -> Result<ProfileDetailResponse> {
        let manifest = self.load_manifest()?;
        if !manifest.profiles.contains_key(profile_name) {
            return Err(AgentStowError::Manifest {
                message: format!("profile 不存在：{}", profile_name.as_str()).into(),
            });
        }

        let manifest_path = self
            .workspace_root
            .join(agentstow_manifest::DEFAULT_MANIFEST_FILE);
        let current = fs_err::read_to_string(&manifest_path).map_err(AgentStowError::from)?;
        let next = rewrite_profile_vars_content(&current, profile_name, vars)?;

        let mut temp =
            tempfile::NamedTempFile::new_in(&self.workspace_root).map_err(AgentStowError::from)?;
        temp.write_all(next.as_bytes()).map_err(AgentStowError::from)?;
        temp.flush().map_err(AgentStowError::from)?;
        agentstow_manifest::Manifest::load_from_path(temp.path())?;
        temp.persist(&manifest_path)
            .map_err(|error| AgentStowError::Other(anyhow::anyhow!(error.error)))?;

        self.profile_detail(profile_name)
    }
}

fn rewrite_profile_vars_content(
    content: &str,
    profile_name: &ProfileName,
    vars: &[ProfileVarUpdateItemRequest],
) -> Result<String> {
    let mut document: toml::Table =
        toml::from_str(content).map_err(|error| AgentStowError::Manifest {
            message: format!("解析 manifest 失败：{error}").into(),
        })?;
    let profile_table = locate_profile_table(&mut document, profile_name)?;
    let extends = profile_table.remove("extends");

    profile_table.clear();
    if let Some(extends) = extends {
        profile_table.insert("extends".into(), extends);
    }

    let parsed_vars = parse_profile_var_items(vars)?;
    profile_table.insert("vars".into(), toml::Value::Table(parsed_vars));

    toml::to_string_pretty(&document).map_err(|error| AgentStowError::Manifest {
        message: format!("序列化 manifest 失败：{error}").into(),
    })
}

fn locate_profile_table<'a>(
    document: &'a mut toml::Table,
    profile_name: &ProfileName,
) -> Result<&'a mut toml::Table> {
    let profiles_value = document
        .get_mut("profiles")
        .ok_or_else(|| AgentStowError::Manifest {
            message: "manifest 缺少 profiles 节点".into(),
        })?;
    let profiles = profiles_value.as_table_mut().ok_or_else(|| AgentStowError::Manifest {
        message: "manifest 中的 profiles 必须是 table".into(),
    })?;
    let profile_value =
        profiles
            .get_mut(profile_name.as_str())
            .ok_or_else(|| AgentStowError::Manifest {
                message: format!("profile 不存在：{}", profile_name.as_str()).into(),
            })?;
    profile_value.as_table_mut().ok_or_else(|| AgentStowError::Manifest {
        message: format!("profile `{}` 必须是 table", profile_name.as_str()).into(),
    })
}

fn parse_profile_var_items(
    vars: &[ProfileVarUpdateItemRequest],
) -> Result<toml::Table> {
    let mut dedup = BTreeMap::<String, toml::Value>::new();

    for item in vars {
        let key = item.key.trim();
        if key.is_empty() {
            return Err(AgentStowError::InvalidArgs {
                message: "profile 变量 key 不能为空".into(),
            });
        }
        if dedup.contains_key(key) {
            return Err(AgentStowError::InvalidArgs {
                message: format!("profile 变量 key 重复：{key}").into(),
            });
        }

        let json_value: serde_json::Value =
            serde_json::from_str(item.value_json.trim()).map_err(|error| {
                AgentStowError::InvalidArgs {
                    message: format!("profile 变量 `{key}` 不是合法 JSON：{error}").into(),
                }
            })?;
        dedup.insert(key.to_string(), json_value_to_toml(&json_value)?);
    }

    Ok(dedup.into_iter().collect())
}

fn json_value_to_toml(value: &serde_json::Value) -> Result<toml::Value> {
    match value {
        serde_json::Value::Null => Err(AgentStowError::InvalidArgs {
            message: "profile 变量不支持 JSON null；请改用字符串、布尔、数字、数组或对象".into(),
        }),
        serde_json::Value::Bool(value) => Ok(toml::Value::Boolean(*value)),
        serde_json::Value::Number(value) => {
            if let Some(integer) = value.as_i64() {
                return Ok(toml::Value::Integer(integer));
            }
            if let Some(unsigned) = value.as_u64() {
                let integer = i64::try_from(unsigned).map_err(|_| AgentStowError::InvalidArgs {
                    message: format!("profile 变量整数超出 TOML i64 范围：{unsigned}").into(),
                })?;
                return Ok(toml::Value::Integer(integer));
            }
            if let Some(float) = value.as_f64() {
                return Ok(toml::Value::Float(float));
            }
            Err(AgentStowError::InvalidArgs {
                message: "profile 变量数字格式不受支持".into(),
            })
        }
        serde_json::Value::String(value) => Ok(toml::Value::String(value.clone())),
        serde_json::Value::Array(values) => values
            .iter()
            .map(json_value_to_toml)
            .collect::<Result<Vec<_>>>()
            .map(toml::Value::Array),
        serde_json::Value::Object(values) => values
            .iter()
            .map(|(key, value)| Ok((key.clone(), json_value_to_toml(value)?)))
            .collect::<Result<toml::Table>>()
            .map(toml::Value::Table),
    }
}
