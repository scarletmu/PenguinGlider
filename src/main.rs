//! PenguinGlider（pglider）：清理 Mac 版 QQ（NT 内核）群聊产生的
//! 图片 / 视频 / 斗图表情缓存。
//!
//! 安全原则：默认 dry-run，默认走废纸篓；只扫描三类聊天媒体缓存目录，
//! 永不触碰加密库或用户资产（personal_emoji / 系统表情 / 商城表情）。

mod clean;
mod discover;
mod filter;
mod model;
mod plan;
mod report;
mod scan;
mod tui;
mod util;

use anyhow::{anyhow, Result};
use clap::{Args, Parser, Subcommand};
use filter::Scope;
use model::{Category, Variant};
use std::collections::BTreeSet;
use std::io::Write;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "pglider",
    version,
    about = "清理 Mac 版 QQ（NT 内核）群聊产生的图片 / 视频 / 斗图表情缓存",
    long_about = None
)]
struct Cli {
    /// 手动指定 nt_data 数据根（默认自动发现所有账号）
    #[arg(long, global = true)]
    root: Option<PathBuf>,

    /// 子命令；不带时进入交互式 TUI
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// 扫描并展示可清理范围（只读，不删除任何文件）
    Scan(ScopeArgs),
    /// 按选定范围清理（默认 dry-run，需 --apply 才真正执行）
    Clean(CleanArgs),
}

/// 范围选择参数（scan / clean 共用）。
#[derive(Args, Clone)]
struct ScopeArgs {
    /// 类目，逗号分隔：pic,video,emoji（默认全部）
    #[arg(long, value_delimiter = ',')]
    category: Vec<String>,

    /// 变体，逗号分隔：ori,thumb,oritemp,thumbtemp（默认全部）
    #[arg(long, value_delimiter = ',')]
    variant: Vec<String>,

    /// 仅清理早于该月（不含）的文件，格式 YYYY-MM
    #[arg(long)]
    before: Option<String>,

    /// 保留最近 N 个月，其余可清
    #[arg(long)]
    keep_months: Option<usize>,

    /// 仅指定月份，逗号分隔，格式 YYYY-MM（可多次）
    #[arg(long, value_delimiter = ',')]
    month: Vec<String>,

    /// 仅清理大于该体积的文件，如 1M / 500K / 2G
    #[arg(long)]
    min_size: Option<String>,
}

impl ScopeArgs {
    /// 是否带了任意筛选维度（都没带则表示「全部可清理缓存」）。
    fn is_filtered(&self) -> bool {
        !self.category.is_empty()
            || !self.variant.is_empty()
            || self.before.is_some()
            || self.keep_months.is_some()
            || !self.month.is_empty()
            || self.min_size.is_some()
    }
}

#[derive(Args)]
struct CleanArgs {
    #[command(flatten)]
    scope: ScopeArgs,

    /// 真正执行清理（不加则仅 dry-run 预览）
    #[arg(long)]
    apply: bool,

    /// 永久删除而非移入废纸篓（不可恢复，慎用）
    #[arg(long)]
    permanent: bool,

    /// 跳过交互确认（用于脚本）
    #[arg(long)]
    yes: bool,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("错误：{e:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    let roots = match &cli.root {
        Some(r) => {
            if !r.is_dir() {
                return Err(anyhow!("指定的 --root 不是有效目录：{}", r.display()));
            }
            vec![r.clone()]
        }
        None => discover::default_roots()?,
    };

    match &cli.command {
        Some(Command::Scan(args)) => cmd_scan(&roots, args),
        Some(Command::Clean(args)) => cmd_clean(&roots, args),
        None => tui::run(&roots),
    }
}

/// 把 CLI 范围参数转成 [`Scope`]，并校验键名 / 月份格式。
fn build_scope(args: &ScopeArgs) -> Result<Scope> {
    let categories = parse_keys(&args.category, "类目", Category::ALL.iter().map(|c| c.key()), Category::from_key)?;
    let variants = parse_keys(&args.variant, "变体", Variant::ALL.iter().map(|v| v.key()), Variant::from_key)?;

    for m in args.month.iter().chain(args.before.iter()) {
        if !scan::is_month(m) {
            return Err(anyhow!("月份格式应为 YYYY-MM：{m}"));
        }
    }

    Ok(Scope {
        categories,
        variants,
        before: args.before.clone(),
        keep_months: args.keep_months,
        months: args.month.iter().cloned().collect(),
        min_size: args.min_size.as_deref().map(util::parse_size).transpose()?,
    })
}

/// 把用户输入的短键逐个解析成枚举值，未知键给出带可选项提示的友好报错。
/// 单一真源是枚举自身的 `from_key` / `key`，新增类目只需改 `model.rs`。
fn parse_keys<'a, T: Ord>(
    inputs: &[String],
    kind: &str,
    valid: impl Iterator<Item = &'a str>,
    from_key: impl Fn(&str) -> Option<T>,
) -> Result<BTreeSet<T>> {
    let hint: Vec<&str> = valid.collect();
    inputs
        .iter()
        .map(|s| from_key(s).ok_or_else(|| anyhow!("未知{kind}：{s}（可选 {}）", hint.join("/"))))
        .collect()
}

fn cmd_scan(roots: &[PathBuf], args: &ScopeArgs) -> Result<()> {
    let scope = build_scope(args)?;
    let title = if args.is_filtered() {
        "待清理范围（已按条件筛选）"
    } else {
        "全部可清理缓存"
    };

    let inventories = plan::scan_all(roots)?;
    plan::select(roots, &inventories, &scope).print(title);
    Ok(())
}

fn cmd_clean(roots: &[PathBuf], args: &CleanArgs) -> Result<()> {
    let scope = build_scope(&args.scope)?;
    let inventories = plan::scan_all(roots)?;
    let selection = plan::select(roots, &inventories, &scope);
    selection.print("待清理范围");

    if selection.count == 0 {
        println!("\n没有匹配的文件，无需清理。");
        return Ok(());
    }

    // 默认 dry-run：仅预览，不进入删除路径。
    if !args.apply {
        println!(
            "\n[dry-run] 将清理 {} 个文件，预计回收 {}。加 --apply 才会真正执行。",
            selection.count,
            util::human(selection.bytes)
        );
        return Ok(());
    }

    let mode = if args.permanent {
        clean::Mode::Permanent
    } else {
        clean::Mode::Trash
    };

    // 真删前的安全提示
    plan::warn_if_qq_running();

    if !args.yes {
        let verb = if args.permanent { "永久删除" } else { "移入废纸篓" };
        print!(
            "\n即将{} {} 个文件（{}）。确认请输入 yes：",
            verb,
            selection.count,
            util::human(selection.bytes)
        );
        std::io::stdout().flush().ok();
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        if line.trim() != "yes" {
            println!("已取消。");
            return Ok(());
        }
    }

    selection.execute(mode)
}
