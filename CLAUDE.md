# CLAUDE.md — PenguinGlider 架构与约束

面向 Mac 版 QQ（NT 内核）的群聊媒体缓存清理 CLI（Rust）。本文件给后续改动定边界。

## 真实环境事实（已在本机核实）

- 数据根：`~/Library/Containers/com.tencent.qq/Data/Library/Application Support/QQ/nt_qq_<hash>/nt_data`，可有多账号。
- 聊天媒体按 `<类目>/YYYY-MM/<变体>/<md5>.<ext>` 缓存：
  - `Pic/`、`Video/`、`Emoji/emoji-recv/`，变体 = `Ori|Thumb|OriTemp|ThumbTemp`。
- **磁盘上不区分群聊 / 私聊**。群↔文件映射在 `nt_db/*.db`，是 **SQLCipher 加密**的（直接读报 "file is not a database"）。
- 因此范围策略是 **时间 + 类型 + 变体 + 体积**，不解密 DB。这是产品决策，勿在不与用户确认的情况下改成「解密按群清理」。

## 绝不触碰

`Emoji/personal_emoji`、`Emoji/BaseEmojiSyastems`、`Emoji/marketface`、`nt_db/` 及任何非上述三类目录。
保护是**结构性**的：扫描只进入 `Category::subpath()` 列举的三条路径，新增类目务必维持这一点。

## 模块地图

| 文件 | 职责 |
|---|---|
| `src/model.rs` | `Category` / `Variant` / `FileEntry`，含磁盘路径与展示名映射 |
| `src/discover.rs` | 自动发现 `nt_qq_*/nt_data` 账号根 |
| `src/scan.rs` | 只读遍历，产出 `FileEntry` 清单；`is_month` 校验 `YYYY-MM` |
| `src/filter.rs` | `Scope` + `apply`，把全量清单收窄到待清理子集 |
| `src/report.rs` | 按 类目 → 月份 → 变体 聚合预览表 |
| `src/clean.rs` | 真正删除：废纸篓 / 永久删除 两种 `Mode`（dry-run 预览在 `main` 处理，不进入此模块） |
| `src/plan.rs` | 流程编排：`scan_all` 扫描全部账号、`select` 过滤出 `Selection`（借用不复制）、`Selection::print/execute` 预览与删除、`warn_if_qq_running`。`scan`/`clean`/`tui` 三条路径共用，确认逻辑由各自负责 |
| `src/tui.rs` | 交互模式（不带子命令时进入）：dialoguer 逐项询问构建 `Scope` → 复用 `plan` 预览并删除 |
| `src/util.rs` | `human` 体积格式化、`parse_size` 解析 |
| `src/main.rs` | clap 参数、scope 构建与校验、CLI 确认流程；`command` 为 `Option`，缺省转 `tui::run`；扫描/过滤/删除编排走 `plan` |

## 安全不变量（改动时必须维持）

1. 默认 dry-run；删除需显式 `--apply`。TUI 中等价物：必须主动选「废纸篓 / 永久删除」并通过二次确认才删，默认 No。
2. 默认走废纸篓（`trash` crate）；`--permanent` 才硬删。TUI 默认高亮「废纸篓」。
3. `--apply` 无 `--yes` 时必须交互确认 `yes`。TUI 用 `Select` + `Confirm` 充当该确认，绝不静默删除。
4. 任何新维度都走 `Scope`，过滤集中在 `filter::apply`（由 `plan::select` 统一调用），不要在 `scan`/`clean`/`tui` 里写散落的过滤。
5. TUI 仅在 TTY 下运行（`tui::run` 开头校验 `stdin().is_terminal()`），非交互场景一律走子命令。

## 开发

```bash
cargo build
./target/debug/pglider scan                      # 只读，安全
./target/debug/pglider --root <fixture>          # 交互模式（需真实终端；管道驱动不了 dialoguer）
./target/debug/pglider --root <fixture> clean --apply --yes   # 用合成目录测删除路径，勿拿真实数据测
```

测删除逻辑请用合成 fixture（`mktemp -d` 造同构目录）经 `--root` 跑，**不要**用真实 QQ 数据做破坏性测试。

## 文档约定

行为 / 选项变更时，同步更新 `README.md`（用户）与本文件（架构）。新增需要落档的概念若无处可放，先问用户，别擅自新建顶层文档。
