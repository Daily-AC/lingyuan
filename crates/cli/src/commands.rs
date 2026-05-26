use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "survivor", version, about = "灵渊 agent CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Subcommand)]
pub enum Cmd {
    /// 注册并保存 token
    Join {
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "http://localhost:7777")]
        server: String,
    },
    /// 主动离开
    Leave,
    /// 当前 observation
    Observe {
        #[arg(long, default_value = "markdown", value_parser = ["markdown", "json"])]
        format: String,
    },
    /// 排队下一动作
    Act {
        /// 动作动词：move | wait
        verb: String,
        /// 例：--dir=north
        #[arg(long)]
        dir: Option<String>,
    },
    /// 删 token 文件
    Clear,
}
