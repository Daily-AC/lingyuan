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
        /// 动作动词：move | wait | gather | eat | craft | place | pickup | drop
        verb: String,
        /// move 用：--dir=north|south|east|west
        #[arg(long)]
        dir: Option<String>,
        /// 目标 tile 坐标 "x,y"，gather/pickup/place 用
        #[arg(long)]
        pos: Option<String>,
        /// 物品名（snake_case），eat/place/drop 用
        #[arg(long)]
        item: Option<String>,
        /// craft 配方 id（snake_case），如 bamboo_spear
        #[arg(long)]
        recipe: Option<String>,
        /// drop 数量
        #[arg(long, default_value_t = 1)]
        n: u16,
        /// attack 用：agent | creature
        #[arg(long)]
        target_kind: Option<String>,
        /// attack 用：agent_id（agent）或 creature_id（creature）
        #[arg(long)]
        target: Option<String>,
        /// write_sign / send_mail 用文本
        #[arg(long)]
        text: Option<String>,
        /// send_mail 用收件人 name
        #[arg(long)]
        to: Option<String>,
    },
    /// 删 token 文件
    Clear,
    /// Demo NPC：自动 join + 简单规则 AI 无限循环（找最近 plant 采集 / 饿了吃 / 看见怪攻击 / 随机走）
    Demo {
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "http://localhost:7777")]
        server: String,
        /// 每多少毫秒 observe + act 一次
        #[arg(long, default_value_t = 800)]
        period_ms: u64,
        /// 看到名字的对话频道（屏幕日志）
        #[arg(long, default_value_t = false)]
        verbose: bool,
    },
}
