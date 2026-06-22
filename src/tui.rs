//! 轻量交互式菜单（不带子命令直接运行 `pglider` 时进入）：
//! 逐项选择清理范围 → 预览可回收量 → 确认 → 清理。
//!
//! 流程编排（扫描 / 过滤 / 预览 / 删除）复用 [`crate::plan`]，与 `clean` 子命令同源；
//! 本模块只负责「交互式地」构建 [`Scope`] 与确认。删除前必须显式选删除方式并二次确认。

use crate::clean::Mode;
use crate::filter::Scope;
use crate::model::{Category, Variant};
use crate::plan;
use crate::scan;
use crate::util;
use anyhow::{anyhow, Result};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, MultiSelect, Select};
use std::collections::BTreeSet;
use std::io::IsTerminal;
use std::path::PathBuf;

/// 交互式入口。`roots` 为已确定的账号数据根（自动发现或 `--root`）。
pub fn run(roots: &[PathBuf]) -> Result<()> {
    if !std::io::stdin().is_terminal() {
        return Err(anyhow!(
            "交互模式需要终端。非交互场景请用子命令，如 `pglider scan` 或 `pglider clean --apply`（见 --help）。"
        ));
    }

    println!("PenguinGlider 交互模式 — 正在扫描 {} 个账号…\n", roots.len());
    let inventories = plan::scan_all(roots)?;
    let total_files: usize = inventories.iter().map(Vec::len).sum();
    if total_files == 0 {
        println!("未扫描到任何可清理缓存。");
        return Ok(());
    }
    println!("共扫描到 {total_files} 个文件。\n");

    // 实际存在的月份（降序），供「按月份截止」直接选择。
    let months: Vec<String> = inventories
        .iter()
        .flatten()
        .map(|e| e.month.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .rev()
        .collect();

    let scope = match build_scope_interactive(&months)? {
        Some(s) => s,
        None => {
            println!("已取消。");
            return Ok(());
        }
    };

    let selection = plan::select(roots, &inventories, &scope);
    selection.print("待清理范围");
    if selection.count == 0 {
        println!("\n没有匹配的文件，无需清理。");
        return Ok(());
    }
    println!(
        "\n合计 {} 个文件，可回收 {}。",
        selection.count,
        util::human(selection.bytes)
    );

    // 确认操作：删除前必须显式选择删除方式（保持安全不变量）。
    let actions = ["移入废纸篓（可恢复）", "永久删除（不可恢复）", "取消"];
    let mode = match Select::with_theme(&ColorfulTheme::default())
        .with_prompt("确认操作")
        .items(&actions)
        .default(0)
        .interact()?
    {
        0 => Mode::Trash,
        1 => Mode::Permanent,
        _ => {
            println!("已取消。");
            return Ok(());
        }
    };

    plan::warn_if_qq_running();

    let verb = if mode == Mode::Permanent {
        "永久删除"
    } else {
        "移入废纸篓"
    };
    let confirmed = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!(
            "即将{verb} {} 个文件（{}），确认？",
            selection.count,
            util::human(selection.bytes)
        ))
        .default(false)
        .interact()?;
    if !confirmed {
        println!("已取消。");
        return Ok(());
    }

    selection.execute(mode)
}

/// 逐项询问构建 [`Scope`]。返回 `None` 表示用户中途清空了类目 / 变体（视为取消）。
fn build_scope_interactive(months: &[String]) -> Result<Option<Scope>> {
    let theme = ColorfulTheme::default();

    println!("下面分 5 步圈定要清理的范围：类目和版本用空格勾选（默认全选），");
    println!("后 3 项按需进一步收窄，选「不…」即不限制。\n");

    let categories = match multiselect_set(
        &theme,
        "① 清理哪几类缓存？（空格勾选/取消，回车确认）",
        &Category::ALL,
        &Category::ALL.map(Category::label),
        "类目",
    )? {
        Some(s) => s,
        None => return Ok(None),
    };

    let variants = match multiselect_set(
        &theme,
        "② 清理哪些版本？（缩略图最占地且会自动重建；Temp 是垃圾最安全）",
        &Variant::ALL,
        &Variant::ALL.map(Variant::label),
        "版本",
    )? {
        Some(s) => s,
        None => return Ok(None),
    };

    // ③ 保留最近 N 个月：预置常用值 + 自定义。
    let keep_choices = [
        "不保留（最早到最新都可清理）",
        "最近 1 个月",
        "最近 2 个月",
        "最近 3 个月",
        "最近 6 个月",
        "自定义…",
    ];
    let keep_months = match Select::with_theme(&theme)
        .with_prompt("③ 保留最近几个月不动？")
        .items(&keep_choices)
        .default(0)
        .interact()?
    {
        0 => None,
        1 => Some(1),
        2 => Some(2),
        3 => Some(3),
        4 => Some(6),
        _ => optional_input(&theme, "输入要保留的月数（如 4）", |s| {
            s.parse::<usize>().map_err(|_| format!("请输入正整数：{s}"))
        })?,
    };

    // ④ 只清理早于某月：从实际扫描到的月份里直接选，省去手敲格式。
    let mut before_choices = vec!["不限（所有月份都纳入）".to_string()];
    before_choices.extend(months.iter().map(|m| format!("早于 {m}（{m} 及更新的保留）")));
    before_choices.push("自定义…".to_string());
    let pick = Select::with_theme(&theme)
        .with_prompt("④ 按月份截止？")
        .items(&before_choices)
        .default(0)
        .interact()?;
    let before = if pick == 0 {
        None
    } else if pick == before_choices.len() - 1 {
        optional_input(&theme, "输入月份 YYYY-MM（清理早于它的）", |s| {
            if scan::is_month(s) {
                Ok(s.to_string())
            } else {
                Err("格式应为 YYYY-MM，例如 2026-01".to_string())
            }
        })?
    } else {
        Some(months[pick - 1].clone())
    };

    // ⑤ 只清理超过某体积：预置常用阈值 + 自定义。
    let size_choices = ["不限体积", "大于 1M", "大于 5M", "大于 10M", "自定义…"];
    let min_size = match Select::with_theme(&theme)
        .with_prompt("⑤ 只清理超过多大的文件？")
        .items(&size_choices)
        .default(0)
        .interact()?
    {
        0 => None,
        1 => Some(1024 * 1024),
        2 => Some(5 * 1024 * 1024),
        3 => Some(10 * 1024 * 1024),
        _ => optional_input(&theme, "输入体积阈值（如 1M / 500K）", |s| {
            util::parse_size(s).map_err(|e| e.to_string())
        })?,
    };

    Ok(Some(Scope {
        categories,
        variants,
        before,
        keep_months,
        months: BTreeSet::new(),
        min_size,
    }))
}

/// 多选枚举（默认全选）。全部取消选择视为放弃，返回 `None`。
fn multiselect_set<T: Ord + Copy>(
    theme: &ColorfulTheme,
    prompt: &str,
    all: &[T],
    labels: &[&str],
    kind: &str,
) -> Result<Option<BTreeSet<T>>> {
    let picked = MultiSelect::with_theme(theme)
        .with_prompt(prompt)
        .items(labels)
        .defaults(&vec![true; all.len()])
        .interact()?;
    if picked.is_empty() {
        println!("未选择任何{kind}。");
        return Ok(None);
    }
    Ok(Some(picked.iter().map(|&i| all[i]).collect()))
}

/// 可空文本输入：留空返回 `None`，否则用 `parse` 解析（解析失败会就地重新提示）。
fn optional_input<T>(
    theme: &ColorfulTheme,
    prompt: &str,
    parse: impl Fn(&str) -> std::result::Result<T, String>,
) -> Result<Option<T>> {
    let raw: String = Input::with_theme(theme)
        .with_prompt(prompt)
        .allow_empty(true)
        .validate_with(|s: &String| {
            let s = s.trim();
            if s.is_empty() {
                Ok(())
            } else {
                parse(s).map(|_| ())
            }
        })
        .interact_text()?;
    match raw.trim() {
        "" => Ok(None),
        s => parse(s).map(Some).map_err(|e| anyhow!(e)),
    }
}
