use std::fmt;

use serde::{Deserialize, Serialize};

fn validate_id(value: &str, label: &str) -> std::result::Result<(), String> {
    let s = value;
    if s.is_empty() {
        return Err(format!("{label} 不能为空"));
    }
    if s == "." || s == ".." {
        return Err(format!("{label} 不能为 {s:?}"));
    }
    if s.len() > 64 {
        return Err(format!("{label} 过长（max=64）：{s}"));
    }
    for ch in s.chars() {
        let ok = ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.');
        if !ok {
            return Err(format!(
                "{label} 含非法字符：{ch:?}（仅允许 [A-Za-z0-9_.-]）"
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct ArtifactId(String);

impl ArtifactId {
    pub fn parse(id: impl Into<String>) -> crate::Result<Self> {
        let s = id.into();
        validate_id(&s, "artifact_id")
            .map_err(|e| crate::AgentStowError::InvalidArgs { message: e.into() })?;
        Ok(Self(s))
    }

    #[must_use]
    pub fn new_unchecked(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ArtifactId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ArtifactId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        validate_id(&s, "artifact_id").map_err(serde::de::Error::custom)?;
        Ok(Self(s))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct ProfileName(String);

impl ProfileName {
    pub fn parse(name: impl Into<String>) -> crate::Result<Self> {
        let s = name.into();
        validate_id(&s, "profile")
            .map_err(|e| crate::AgentStowError::InvalidArgs { message: e.into() })?;
        Ok(Self(s))
    }

    #[must_use]
    pub fn new_unchecked(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProfileName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ProfileName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        validate_id(&s, "profile").map_err(serde::de::Error::custom)?;
        Ok(Self(s))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct TargetName(String);

impl TargetName {
    pub fn parse(name: impl Into<String>) -> crate::Result<Self> {
        let s = name.into();
        validate_id(&s, "target")
            .map_err(|e| crate::AgentStowError::InvalidArgs { message: e.into() })?;
        Ok(Self(s))
    }

    #[must_use]
    pub fn new_unchecked(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TargetName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for TargetName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        validate_id(&s, "target").map_err(serde::de::Error::custom)?;
        Ok(Self(s))
    }
}
