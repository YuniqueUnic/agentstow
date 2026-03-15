use std::net::SocketAddr;
use std::path::PathBuf;

use agentstow_core::ShellKind;
use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "agentstow",
    version,
    about = "Git-native source-of-truth manager for AI artifacts"
)]
pub struct Cli {
    /// 输出机器可读 JSON（用于测试/自动化）
    #[arg(long)]
    pub json: bool,

    /// 禁止交互式提示（用于 CI/自动化）
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,

    /// 关闭彩色输出
    #[arg(long = "no-color")]
    pub no_color: bool,

    /// 先切换工作目录再执行
    #[arg(long)]
    pub cwd: Option<PathBuf>,

    /// 指定 workspace 根目录（默认从 cwd 向上寻找 `agentstow.toml`）
    #[arg(long)]
    pub workspace: Option<PathBuf>,

    /// 默认 profile（可被子命令覆盖）
    #[arg(long)]
    pub profile: Option<String>,

    /// 全局超时（毫秒），用于脚本执行等
    #[arg(long)]
    pub timeout: Option<u64>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Render(RenderArgs),
    Validate(ValidateArgs),
    Link(LinkArgs),
    Env(EnvArgs),
    Scripts(ScriptsArgs),
    Mcp(McpArgs),
    Workspace(WorkspaceArgs),
    Serve(ServeArgs),
}

#[derive(Debug, Args)]
pub struct RenderArgs {
    #[arg(long)]
    pub artifact: String,
    #[arg(long)]
    pub profile: Option<String>,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct ValidateArgs {
    #[arg(long)]
    pub artifact: String,
    #[arg(long)]
    pub profile: Option<String>,
}

#[derive(Debug, Args)]
pub struct LinkArgs {
    /// 仅输出 plan，不执行安装
    #[arg(long)]
    pub plan: bool,
    /// 覆盖冲突 target
    #[arg(long)]
    pub force: bool,
    /// 指定 target（可重复）；为空则对所有 targets 生效
    #[arg(long = "target")]
    pub targets: Vec<String>,
    #[command(subcommand)]
    pub cmd: Option<LinkSubcommand>,
}

#[derive(Debug, Subcommand)]
pub enum LinkSubcommand {
    Status,
    Repair {
        /// 仅修复指定 target（可重复）；为空则修复所有不健康项
        #[arg(long = "target")]
        targets: Vec<String>,
        #[arg(long)]
        force: bool,
    },
}

#[derive(Debug, Args)]
pub struct EnvArgs {
    #[command(subcommand)]
    pub cmd: EnvSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum EnvSubcommand {
    Emit {
        #[arg(long)]
        set: String,
        #[arg(long)]
        shell: ShellKind,
        #[arg(long)]
        stdout: bool,
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[derive(Debug, Args)]
pub struct ScriptsArgs {
    #[command(subcommand)]
    pub cmd: ScriptsSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ScriptsSubcommand {
    Run {
        #[arg(long)]
        id: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        stdin: Option<String>,
    },
}

#[derive(Debug, Args)]
pub struct McpArgs {
    #[command(subcommand)]
    pub cmd: McpSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum McpSubcommand {
    Validate,
    Render {
        #[arg(long)]
        stdout: bool,
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[derive(Debug, Args)]
pub struct WorkspaceArgs {
    #[command(subcommand)]
    pub cmd: WorkspaceSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum WorkspaceSubcommand {
    Status,
    Init {
        /// 初始化 workspace（创建最小 agentstow.toml + 示例 artifacts）
        #[arg(long)]
        git_init: bool,
    },
}

#[derive(Debug, Args)]
pub struct ServeArgs {
    #[arg(long, default_value = "127.0.0.1:8787")]
    pub addr: SocketAddr,
}
