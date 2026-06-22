//! 核心数据模型：清理对象的分类、变体与单文件条目。
//!
//! 设计前提（已在真实环境核实）：QQ NT 把「聊天产生的媒体」按
//! `<类目>/YYYY-MM/<变体>/<md5>.<ext>` 组织，**磁盘上不区分群聊 / 私聊**，
//! 群↔文件映射在 SQLCipher 加密库里。本工具只按时间 + 类型 + 变体清理，
//! 不触碰加密库，也永不进入 personal_emoji / BaseEmojiSyastems / marketface 等用户资产目录。

use std::path::PathBuf;

/// 可清理的三类「聊天媒体缓存」。每一类在磁盘上结构同构。
///
/// 派生顺序（`PartialOrd`/`Ord`）即声明顺序，与 [`Category::ALL`] 一致，
/// 供报表按类目稳定排序。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub enum Category {
    /// 图片缓存：`Pic/`
    Pic,
    /// 视频缓存：`Video/`
    Video,
    /// 斗图 / 收到的自定义表情缓存：`Emoji/emoji-recv/`
    ///
    /// 注意：仅 `emoji-recv`。同级的 `personal_emoji`（自己收藏）、
    /// `BaseEmojiSyastems`（系统包）、`marketface`（商城）属于用户资产，永不清理。
    Emoji,
}

impl Category {
    pub const ALL: [Category; 3] = [Category::Pic, Category::Video, Category::Emoji];

    /// 命令行短键。
    pub fn key(self) -> &'static str {
        match self {
            Category::Pic => "pic",
            Category::Video => "video",
            Category::Emoji => "emoji",
        }
    }

    /// 由命令行短键解析（大小写不敏感）；未知键返回 `None`。
    pub fn from_key(s: &str) -> Option<Category> {
        let s = s.trim().to_ascii_lowercase();
        Category::ALL.into_iter().find(|c| c.key() == s)
    }

    /// 中文展示名。
    pub fn label(self) -> &'static str {
        match self {
            Category::Pic => "图片",
            Category::Video => "视频",
            Category::Emoji => "斗图表情",
        }
    }

    /// 相对 `nt_data` 根的子路径。
    pub fn subpath(self) -> &'static str {
        match self {
            Category::Pic => "Pic",
            Category::Video => "Video",
            Category::Emoji => "Emoji/emoji-recv",
        }
    }
}

/// 月份目录下的变体子目录。`*Temp` 是中断 / 失败遗留的临时件，最安全。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub enum Variant {
    /// 原图 / 原视频：`Ori`
    Ori,
    /// 缩略 / 显示用图：`Thumb`（实测占比最大）
    Thumb,
    /// 原件临时残留：`OriTemp`
    OriTemp,
    /// 缩略临时残留：`ThumbTemp`
    ThumbTemp,
}

impl Variant {
    pub const ALL: [Variant; 4] = [
        Variant::Ori,
        Variant::Thumb,
        Variant::OriTemp,
        Variant::ThumbTemp,
    ];

    /// 磁盘目录名。
    pub fn dir_name(self) -> &'static str {
        match self {
            Variant::Ori => "Ori",
            Variant::Thumb => "Thumb",
            Variant::OriTemp => "OriTemp",
            Variant::ThumbTemp => "ThumbTemp",
        }
    }

    /// 面向用户的中文标签（含安全提示），供交互界面展示。
    pub fn label(self) -> &'static str {
        match self {
            Variant::Ori => "原件 — 原图/原视频，删后再看历史消息需重新下载",
            Variant::Thumb => "缩略图 — 聊天列表显示用，占地最大、会自动重建（最划算）",
            Variant::OriTemp => "原件临时残留 — 下载中断的垃圾，删除最安全",
            Variant::ThumbTemp => "缩略图临时残留 — 垃圾，删除最安全",
        }
    }

    /// 命令行短键（小写）。
    pub fn key(self) -> &'static str {
        match self {
            Variant::Ori => "ori",
            Variant::Thumb => "thumb",
            Variant::OriTemp => "oritemp",
            Variant::ThumbTemp => "thumbtemp",
        }
    }

    /// 由命令行短键解析（大小写不敏感）；未知键返回 `None`。
    pub fn from_key(s: &str) -> Option<Variant> {
        let s = s.trim().to_ascii_lowercase();
        Variant::ALL.into_iter().find(|v| v.key() == s)
    }
}

/// 扫描到的单个文件。
#[derive(Clone, Debug)]
pub struct FileEntry {
    pub path: PathBuf,
    pub size: u64,
    pub category: Category,
    /// `YYYY-MM`
    pub month: String,
    pub variant: Variant,
}
