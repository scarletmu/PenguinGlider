//! 定位 QQ NT 的账号数据根目录（`nt_data`）。
//!
//! 路径形如：
//! `~/Library/Containers/com.tencent.qq/Data/Library/Application Support/QQ/nt_qq_<hash>/nt_data`
//! 可能有多个账号（多个 `nt_qq_*`），全部返回。

use anyhow::{anyhow, Result};
use std::path::PathBuf;

/// QQ 沙盒内 NT 数据的父目录（含若干 `nt_qq_*`）。
fn qq_app_support() -> Result<PathBuf> {
    let home = std::env::var_os("HOME").ok_or_else(|| anyhow!("无法读取 HOME 环境变量"))?;
    Ok(PathBuf::from(home)
        .join("Library/Containers/com.tencent.qq/Data/Library/Application Support/QQ"))
}

/// 自动发现所有账号的 `nt_data` 根目录。
pub fn default_roots() -> Result<Vec<PathBuf>> {
    let base = qq_app_support()?;
    if !base.is_dir() {
        return Err(anyhow!(
            "未找到 QQ 数据目录：{}\n请确认已安装 Mac 版 QQ（NT 内核）并至少登录过一次。",
            base.display()
        ));
    }

    let mut roots = Vec::new();
    for entry in std::fs::read_dir(&base)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("nt_qq_") {
            let nt_data = entry.path().join("nt_data");
            if nt_data.is_dir() {
                roots.push(nt_data);
            }
        }
    }

    if roots.is_empty() {
        return Err(anyhow!(
            "在 {} 下未找到任何 nt_qq_*/nt_data 账号目录。",
            base.display()
        ));
    }
    roots.sort();
    Ok(roots)
}
