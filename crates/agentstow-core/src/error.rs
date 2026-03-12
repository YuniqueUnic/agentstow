use std::borrow::Cow;

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Ok = 0,
    GenericError = 1,
    InvalidArgs = 2,
    InvalidConfig = 3,
    ValidationFailed = 4,
    Conflict = 5,
    Io = 6,
    ExternalCommandFailed = 7,
}

impl ExitCode {
    #[must_use]
    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

#[derive(Debug, Error)]
pub enum AgentStowError {
    #[error("参数错误: {message}")]
    InvalidArgs { message: Cow<'static, str> },

    #[error("配置/manifest 错误: {message}")]
    Manifest { message: Cow<'static, str> },

    #[error("渲染失败: {message}")]
    Render { message: Cow<'static, str> },

    #[error("校验失败: {message}")]
    Validate { message: Cow<'static, str> },

    #[error("链接冲突: {message}")]
    LinkConflict { message: Cow<'static, str> },

    #[error("链接/安装失败: {message}")]
    Link { message: Cow<'static, str> },

    #[error("状态库失败: {message}")]
    State { message: Cow<'static, str> },

    #[error("Git 操作失败: {message}")]
    Git { message: Cow<'static, str> },

    #[error("脚本执行失败: {message}")]
    Script { message: Cow<'static, str> },

    #[error("MCP 失败: {message}")]
    Mcp { message: Cow<'static, str> },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl AgentStowError {
    #[must_use]
    pub fn exit_code(&self) -> ExitCode {
        match self {
            Self::InvalidArgs { .. } => ExitCode::InvalidArgs,
            Self::Manifest { .. } => ExitCode::InvalidConfig,
            Self::Render { .. } => ExitCode::InvalidConfig,
            Self::Validate { .. } => ExitCode::ValidationFailed,
            Self::LinkConflict { .. } => ExitCode::Conflict,
            Self::Link { .. } => ExitCode::GenericError,
            Self::State { .. } => ExitCode::GenericError,
            Self::Git { .. } => ExitCode::ExternalCommandFailed,
            Self::Script { .. } => ExitCode::ExternalCommandFailed,
            Self::Mcp { .. } => ExitCode::InvalidConfig,
            Self::Io(_) => ExitCode::Io,
            Self::Other(_) => ExitCode::GenericError,
        }
    }
}

pub type Result<T> = std::result::Result<T, AgentStowError>;
