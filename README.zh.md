# SnoozeLine

[English](README.md) | [中文](README.zh.md)

SnoozeLine 是一个用 Rust 编写、支持自定义的精简 Claude Code 状态栏。它将当前模型、工作目录、上下文占用和 5 小时／7 天额度用量集中展示在一行，并在数据可用时显示 Git 状态与输出风格。

基于 [CCometixLine](https://github.com/Haleclipse/CCometixLine) v1.1.2 独立维护，默认采用 `snooze26h` 主题。

## 效果预览

![SnoozeLine 的 snooze26h 主题：Fable 5.1、snooze26h 目录、26% 上下文、256.8k tokens、5h 额度缺失、7d 已用 24%、default 输出风格](assets/snoozeline-preview.png)

上图为 `snooze26h` 主题的实际终端截图。图标需要 Nerd Font，颜色随终端配色方案变化。

## 显示内容

| 字段 | 示例 | 含义 |
| --- | --- | --- |
| 模型 | `Fable 5.1` | 当前模型名称 |
| 目录 | `snooze26h` | 当前工作文件夹 |
| 上下文 | `26% · 256.8k tokens` | 上下文窗口占用，以及当前输入与缓存输入的 token 数 |
| 额度 | `5h - · 7d 24%` | 对应额度窗口的**已用百分比**；`-` 表示该窗口数据不可用 |
| Git | 分支与状态 | 当前目录的 Git 信息可用时显示 |
| 输出风格 | `default` | Claude Code 提供的输出风格名称 |

这张截图没有显示 Git 信息；末尾的火箭图标和 `default` 表示输出风格。

### 功能特点

- **默认布局精简**：模型、目录、上下文、额度、Git 和输出风格集中在一行，省略额度重置时间。
- **优先使用原生数据**：优先读取 Claude Code 提供的上下文和额度字段，保留兼容回退。
- **终端配置界面**：通过 `--config` 预览主题，调整字段显示、顺序、颜色、图标和分隔符。
- **十个内置主题**：`snooze26h`、`cometix`、`default`、`minimal`、`gruvbox`、`nord`，以及四个 Powerline 主题；支持 TOML 自定义主题。
- **独立运行目录**：配置与缓存保存在 `~/.claude/snoozeline`，便于保留已有的 `ccline` 安装用于回滚。

## 安装

仓库已公开托管 `0.1.0` 版本的源码，尚无 GitHub Release。下面提供 macOS / Linux 的源码构建与安装方式。

需要 Claude Code、Git 和 Rust stable。要显示默认主题的图标，请在终端选择 Nerd Font；也可以使用采用普通 emoji 图标的 `default` 主题。

### 1. 构建并安装

```sh
git clone https://github.com/snooze26h/SnoozeLine.git
cd SnoozeLine
cargo build --release --locked

install -d "$HOME/.claude/snoozeline"
install -m 0755 ./target/release/snoozeline "$HOME/.claude/snoozeline/snoozeline"
```

### 2. 接入 Claude Code

先备份 `~/.claude/settings.json`，再将下面的 `statusLine` 条目合并进去，保留其他设置。把示例命令替换为已安装二进制的绝对路径；路径含空格时需要保留内层引号。

```json
{
  "statusLine": {
    "type": "command",
    "command": "\"/absolute/path/to/.claude/snoozeline/snoozeline\"",
    "padding": 0
  }
}
```

重启 Claude Code 后加载状态栏。如果从 CCometixLine 迁移，请保留 `~/.claude/ccline`，方便恢复原来的状态栏命令。

<details>
<summary>可选：用 jq 备份并更新已有配置</summary>

需要安装 `jq`，且 `~/.claude/settings.json` 已存在。脚本会生成带时间戳的备份，仅更新 `statusLine` 条目。

```sh
settings_file="$HOME/.claude/settings.json"
(
  set -eu
  settings_dir="$(dirname "$settings_file")"
  backup_file="$(mktemp "$settings_dir/settings.json.before-snoozeline.$(date +%Y%m%d-%H%M%S).XXXXXX")"
  temp_file="$(mktemp "$settings_dir/.settings.json.snoozeline.XXXXXX")"
  trap 'rm -f "$temp_file"' EXIT HUP INT TERM

  cp -p "$settings_file" "$backup_file"
  cp -p "$settings_file" "$temp_file"
  jq --arg command "$HOME/.claude/snoozeline/snoozeline" \
    '.statusLine = ((.statusLine // {}) + {"type":"command","command":($command | @sh),"padding":0})' \
    "$settings_file" > "$temp_file"
  mv "$temp_file" "$settings_file"
  trap - EXIT HUP INT TERM
  printf 'Backup saved to: %s\n' "$backup_file"
)
```

回滚时，将下面的占位路径替换为脚本输出的准确备份路径。这会将整个配置文件恢复到备份时的内容：

```sh
backup_file="/exact/backup/path/printed/above"
cp -p "$backup_file" "$HOME/.claude/settings.json"
```

</details>

## 配置

打开交互式编辑器：

```sh
"$HOME/.claude/snoozeline/snoozeline" --config
```

按 `p` 轮换主题，按 `s` 保存，按 `Esc` 退出。编辑器也支持逐项调整字段和保存自定义主题。

默认运行根目录为 `~/.claude/snoozeline`：

```text
~/.claude/snoozeline/
├── config.toml          # 状态栏外观与启用的字段
├── models.toml          # 模型显示名称覆盖
├── themes/*.toml        # 内置与自定义主题
└── .api_usage_cache.json
```

可以将 `SNOOZELINE_HOME` 设为绝对路径，指定其他运行根目录。文件按需创建。

在 `models.toml` 中，如果 Claude 条目的 `display_name` 与 `pattern` 对应的标准名称一致，显示名称会跟随实际模型版本。例如，`pattern = "claude-opus-5"` 配合 `display_name = "Opus 5"`，遇到 `claude-opus-5-5` 时会显示 `Opus 5.5`，同时保留该条目的 `context_limit`。`Work Opus` 这样的自定义别名保持不变，`1M` 等上下文后缀仍正常追加。

## 用量计算规则

- 优先使用 Claude Code 原生上下文数据。当前上下文 token 包括输入和缓存输入，不计输出 token。
- 上下文和额度百分比会校验并限制在 `0–100%`。
- `5h`、`7d` 均显示**已用百分比**。一个窗口数据缺失时显示 `-`；原生与回退数据都无法提供任何窗口数值时，隐藏额度字段。
- 原生额度数据不可用时，可以使用兼容的 Claude 用量接口及账户隔离缓存作为回退。
- SnoozeLine 自身的缓存不保存对话正文。

## 开发

在仓库根目录执行：

```sh
cargo metadata --locked --no-deps --format-version 1
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked
git diff --check
```

构建后，使用原生模拟数据和临时运行目录渲染状态栏：

```sh
smoke_root="$(mktemp -d)"
printf '%s\n' '{"model":{"id":"claude-fable-5-1","display_name":"Fable 5.1"},"workspace":{"current_dir":"/tmp/snoozeline-demo"},"context_window":{"context_window_size":1000000,"used_percentage":26,"current_usage":{"input_tokens":256800}},"rate_limits":{"five_hour":{},"seven_day":{"used_percentage":24}},"output_style":{"name":"default"}}' \
  | SNOOZELINE_HOME="$smoke_root" \
    ./target/debug/snoozeline --theme snooze26h
```

输出应包含 `Fable 5.1`、`snoozeline-demo`、`26% · 256.8k tokens`、`5h - · 7d 24%` 和 `default`。这组模拟数据无需账号，也不会请求额度接口。

## 上游与许可

SnoozeLine 基于 Haleclipse 及贡献者的 CCometixLine v1.1.2，保留上游 Git 历史与作者署名，作为独立衍生项目维护，并非上游官方版本。

上游包元数据与 README 声明 `MIT`，但导入的 v1.1.2 快照缺少所引用的 `LICENSE` 正文。该声明仍待确认，源码公开并未解决声明缺失的问题；SnoozeLine 不会自行推定许可证正文或版权持有人。

具体基线和衍生修改见 [UPSTREAM.md](UPSTREAM.md)，第三方署名见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)，变更记录见 [CHANGELOG.md](CHANGELOG.md)。
