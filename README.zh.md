# SnoozeLine

[English](README.md) | [中文](README.zh.md)

SnoozeLine 是一个用 Rust 编写、支持自定义的精简 Claude Code 状态栏。它将当前模型与思考程度、工作目录、上下文占用和 5 小时／7 天额度用量集中展示在一行，并在数据可用时显示 Git 状态与输出风格。

基于 [CCometixLine](https://github.com/Haleclipse/CCometixLine) v1.1.2 独立维护，并选择性吸收 v2.0.1 的 Git 性能、输入兼容与预览改进；默认采用 `snooze26h` 主题，继续兼容现有配置。

## 效果预览

![SnoozeLine 的 snooze26h 主题：Fable 5.1、snooze26h 目录、26% 上下文、256.8k tokens、5h 额度缺失、7d 已用 24%、default 输出风格](assets/snoozeline-preview.png)

上图为加入思考程度显示之前的 `snooze26h` 主题实际终端截图。图标需要 Nerd Font，颜色随终端配色方案变化。

## 显示内容

| 字段 | 示例 | 含义 |
| --- | --- | --- |
| 模型 | `Fable 5.1 · max` | 当前模型名称，以及数据可用时的思考程度 |
| 目录 | `snooze26h` | 当前工作文件夹 |
| 上下文 | `26% · 256.8k tokens` | 上下文窗口占用，以及当前输入与缓存输入的 token 数 |
| 额度 | `5h - · 7d 24%` | 对应额度窗口的**已用百分比**；`-` 表示该窗口数据不可用 |
| Git | 分支与状态 | 当前目录的 Git 信息可用时显示 |
| 输出风格 | `default` | Claude Code 提供的输出风格名称 |

这张截图没有显示 Git 信息；末尾的火箭图标和 `default` 表示输出风格。

### 功能特点

- **默认布局精简**：模型、目录、上下文、额度、Git 和输出风格集中在一行，省略额度重置时间。
- **优先使用原生数据**：优先读取 Claude Code 提供的上下文和额度字段，保留兼容回退。
- **实时思考程度**：在模型名旁显示 `low`、`medium`、`high`、`xhigh` 或 `max`，跟随当前会话的选择。
- **Git 刷新加速**：一次查询获取 Git 信息，默认缓存 2 秒，过期后在后台刷新。
- **只读预览**：`--preview` 使用模拟数据展示当前布局，不查询 Git、额度接口或创建运行文件。
- **输入兼容**：小数格式的额度重置时间可正常解析；可选字段格式变化只影响对应字段。
- **终端配置界面**：通过 `--config` 预览主题，调整字段显示、顺序、颜色、图标和分隔符。
- **十个内置主题**：`snooze26h`、`cometix`、`default`、`minimal`、`gruvbox`、`nord`，以及四个 Powerline 主题；支持 TOML 自定义主题。
- **独立运行目录**：配置与缓存保存在 `~/.claude/snoozeline`，便于保留已有的 `ccline` 安装用于回滚。

## 安装

当前源码版本为 `0.2.0`，尚未发布此版本的 GitHub Release 或 npm 包。下面提供 macOS / Linux 的源码构建与安装方式；继承的私有 npm 模板仍固定在 `0.1.0`。

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

先查看当前布局的只读预览：

```sh
"$HOME/.claude/snoozeline/snoozeline" --preview
"$HOME/.claude/snoozeline/snoozeline" --preview --theme minimal
```

预览使用内置模拟数据，不代表当前会话的实际用量。

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
├── git-cache/*.json     # 按工作目录隔离的 Git 状态摘要
└── .api_usage_cache.json
```

可以将 `SNOOZELINE_HOME` 设为绝对路径，指定其他运行根目录。文件按需创建。

在 `models.toml` 中，如果 Claude 条目的 `display_name` 与 `pattern` 对应的标准名称一致，显示名称会跟随实际模型版本。例如，`pattern = "claude-opus-5"` 配合 `display_name = "Opus 5"`，遇到 `claude-opus-5-5` 时会显示 `Opus 5.5`，同时保留该条目的 `context_limit`。`Work Opus` 这样的自定义别名保持不变，`1M` 等上下文后缀仍正常追加。

### Git 缓存

默认两秒内复用同一工作目录的 Git 状态，过期后先显示缓存并在后台刷新；默认最多显示 30 秒前的状态。新目录首次查询最多等待 500 毫秒，较慢的查询交给后台继续完成。

可在 `config.toml` 中现有 `id = "git"` 条目的 `[segments.options]` 下设置 `cache_ttl_ms = 0` 禁用缓存；该选项范围为 `0–60000` 毫秒，较大的值按 60000 处理。自定义 Git 索引或工作树环境变量会自动绕过缓存。

### 思考程度

模型字段会自动追加当前会话的 [`effort.level`](https://code.claude.com/docs/en/statusline#available-data)，例如 `Fable 5.1 · max`。每次运行都读取新的标准输入，不缓存档位，也不从设置文件或环境变量读取默认档位。通过 `/effort` 或 `/model` 选择器修改后，Claude Code 刷新状态栏时便会显示新值；Claude Code 2.1.280 会在模型或档位变化时触发刷新，遵循其正常的防抖机制，无需额外轮询或配置。

如果模型或 Claude Code 版本不提供 `effort.level`，或字段缺失、为 null、格式错误、值不受支持，则只显示模型名。自定义模型别名和上下文后缀仍正常生效。

## 用量计算规则

- 优先使用 Claude Code 原生上下文数据。当前上下文 token 包括输入和缓存输入，不计输出 token。
- 上下文和额度百分比会校验并限制在 `0–100%`。
- `5h`、`7d` 均显示**已用百分比**。一个窗口缺失或已过重置时间时显示 `-`；两个窗口都没有可用数据时隐藏额度字段。
- 只要输入包含原生 `rate_limits` 字段，就以它为准，即使为空或格式异常也不回退。输入完全没有该字段时，仍可使用兼容的 Claude 用量接口及账户隔离缓存。
- `SNOOZELINE_NATIVE_QUOTA_ONLY=1` 可关闭全部额度接口回退，供独立账号入口使用。现有个人账号启动器无需修改。
- 输入 JSON 上限为 1 MiB；结构损坏或超长时只报告通用错误，不回显输入。
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
printf '%s\n' '{"model":{"id":"claude-fable-5-1","display_name":"Fable 5.1"},"effort":{"level":"max"},"workspace":{"current_dir":"/tmp/snoozeline-demo"},"context_window":{"context_window_size":1000000,"used_percentage":26,"current_usage":{"input_tokens":256800}},"rate_limits":{"five_hour":{},"seven_day":{"used_percentage":24}},"output_style":{"name":"default"}}' \
  | SNOOZELINE_HOME="$smoke_root" \
    ./target/debug/snoozeline --theme snooze26h
```

输出应包含 `Fable 5.1 · max`、`snoozeline-demo`、`26% · 256.8k tokens`、`5h - · 7d 24%` 和 `default`。这组模拟数据无需账号，也不会请求额度接口。

## 上游与许可

SnoozeLine 基于 Haleclipse 及贡献者的 CCometixLine v1.1.2，保留上游 Git 历史与作者署名，作为独立衍生项目维护，并非上游官方版本。

上游包元数据与 README 声明 `MIT`，但导入的 v1.1.2 快照缺少所引用的 `LICENSE` 正文。该声明仍待确认，源码公开并未解决声明缺失的问题；SnoozeLine 不会自行推定许可证正文或版权持有人。

具体基线和衍生修改见 [UPSTREAM.md](UPSTREAM.md)，第三方署名见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)，变更记录见 [CHANGELOG.md](CHANGELOG.md)。
