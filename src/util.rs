//! 通用小工具：人类可读体积、体积解析。

use anyhow::{anyhow, Result};

/// 字节数 → 人类可读（如 `1.5G`、`602.3M`）。
pub fn human(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "K", "M", "G", "T"];
    let mut v = bytes as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{}{}", bytes, UNITS[0])
    } else {
        format!("{:.1}{}", v, UNITS[i])
    }
}

/// 解析体积阈值，如 `1M`、`500K`、`2G`、`1024`（纯数字按字节）。
pub fn parse_size(s: &str) -> Result<u64> {
    let s = s.trim();
    if s.is_empty() {
        return Err(anyhow!("空的体积值"));
    }
    let (num, mult) = match s.chars().last().unwrap().to_ascii_uppercase() {
        'B' => (&s[..s.len() - 1], 1u64),
        'K' => (&s[..s.len() - 1], 1024),
        'M' => (&s[..s.len() - 1], 1024 * 1024),
        'G' => (&s[..s.len() - 1], 1024 * 1024 * 1024),
        'T' => (&s[..s.len() - 1], 1024u64.pow(4)),
        c if c.is_ascii_digit() => (s, 1),
        c => return Err(anyhow!("无法识别的体积单位：{c}（支持 B/K/M/G/T）")),
    };
    let val: f64 = num
        .trim()
        .parse()
        .map_err(|_| anyhow!("无法解析体积数值：{s}"))?;
    Ok((val * mult as f64) as u64)
}
