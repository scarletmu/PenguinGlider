# PenguinGlider 🐧

清理 **Mac 版 QQ（NT 内核）** 群聊产生的 **图片 / 视频 / 斗图表情** 缓存的命令行工具。
清理前先展示可回收范围，支持用户按需选择，默认安全（dry-run + 废纸篓）。

> 二进制名：`pglider`

## 它清理什么

QQ NT 把聊天产生的媒体按 `<类目>/YYYY-MM/<变体>/<md5>.<ext>` 缓存在账号沙盒里。
本工具只动这三类「聊天媒体缓存」：

| 类目 (`--category`) | 磁盘路径 | 说明 |
|---|---|---|
| `pic`   | `Pic/`              | 聊天图片缓存 |
| `video` | `Video/`            | 聊天视频缓存 |
| `emoji` | `Emoji/emoji-recv/` | 收到的自定义表情 / 斗图缓存 |

每个月份目录下分四种**变体**（`--variant`）：`ori`（原件）、`thumb`（缩略 / 显示用，通常占比最大）、`oritemp` / `thumbtemp`（临时残留，最安全）。

### 永不触碰

- `Emoji/personal_emoji`（自己收藏的表情）、`Emoji/BaseEmojiSyastems`（系统包）、`Emoji/marketface`（商城）—— 用户资产
- `nt_db/*`（消息 / 群信息数据库）以及其它任何目录

> ⚠️ **重要限制**：磁盘上群聊与私聊的图片 / 视频**混在一起、按月份组织，无法纯文件区分**（群↔文件映射在 SQLCipher 加密库里，本工具不解密）。因此清理范围按「时间 + 类型 + 变体 + 体积」选择，会同时覆盖群聊与私聊的缓存。

## 安装

```bash
cargo build --release
# 产物：target/release/pglider
```

## 用法

### 交互模式（TUI）

不带任何子命令直接运行，进入逐项引导的交互菜单：

```bash
pglider                 # 自动发现账号 → 选范围 → 预览 → 确认 → 清理
pglider --root <path>   # 指定单个 nt_data 账号根
```

流程：先扫描全部账号（只读），再依次询问 **类目 → 变体 → 保留最近 N 月 → 早于某月 → 最小体积**（任一留空即不限），随后展示可回收预览，最后让你在 **移入废纸篓 / 永久删除 / 取消** 间选择并二次确认。和 CLI 一样：不确认绝不删除，默认走废纸篓。

> 需在真实终端运行；管道 / 重定向（非 TTY）场景请改用下面的 `scan` / `clean` 子命令。

### 先看范围（只读，绝不删除）

```bash
pglider scan                          # 列出全部可回收缓存
pglider scan --category video         # 只看视频
pglider scan --before 2026-06         # 只看 2026-06 之前的
pglider scan --keep-months 2          # 看「保留最近 2 个月」之外的部分
pglider scan --variant thumb --min-size 1M
```

### 再清理（默认 dry-run）

```bash
pglider clean --before 2026-05                 # dry-run：只预览将清理什么
pglider clean --before 2026-05 --apply         # 真正执行 → 移入废纸篓（可恢复）
pglider clean --category emoji --apply --yes   # 跳过交互确认（脚本用）
pglider clean --variant oritemp,thumbtemp --apply   # 只清临时残留，最安全
pglider clean --before 2025-01 --apply --permanent  # 永久删除（不可恢复，慎用）
```

### 范围选项（scan / clean 通用）

| 选项 | 含义 |
|---|---|
| `--category pic,video,emoji` | 限定类目（默认全部） |
| `--variant ori,thumb,oritemp,thumbtemp` | 限定变体（默认全部） |
| `--before YYYY-MM` | 仅清理**早于**该月（不含该月）的文件 |
| `--keep-months N` | 保留最近 N 个月，其余可清 |
| `--month YYYY-MM[,…]` | 仅指定月份 |
| `--min-size 1M` | 仅大于该体积的文件（支持 B/K/M/G/T） |
| `--root <path>` | 手动指定 `nt_data` 数据根（默认自动发现所有账号） |

### clean 专属

| 选项 | 含义 |
|---|---|
| `--apply` | 真正执行（不加则仅 dry-run） |
| `--permanent` | 永久删除而非移入废纸篓 |
| `--yes` | 跳过交互确认 |

## 安全设计

- **默认 dry-run**：不加 `--apply` 绝不删除。
- **默认走废纸篓**：删除可恢复，除非显式 `--permanent`。
- **交互二次确认**：`--apply` 时需输入 `yes`（`--yes` 跳过）。
- **QQ 运行检测**：清理前若检测到 QQ 在跑会警告（建议先退出 QQ）。
- **白名单隔离**：只扫描三类媒体缓存目录，结构上无法触及用户资产或数据库。

## 自动定位

默认自动发现：
```
~/Library/Containers/com.tencent.qq/Data/Library/Application Support/QQ/nt_qq_*/nt_data
```
多账号会逐个列出 / 处理。
