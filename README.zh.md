# SnoozeLine

[English](README.md) | [中文](README.zh.md)

SnoozeLine 是一个供个人在本地使用的精简 Claude Code 状态栏，默认内置主题为 `snooze26h`。

> **项目来源：** SnoozeLine 是基于 Haleclipse 及贡献者的 [CCometixLine](https://github.com/Haleclipse/CCometixLine) v1.1.2 开发的独立维护衍生版，并非上游官方版本。具体基线、署名与许可证据见 [UPSTREAM.md](UPSTREAM.md) 和 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。

## 当前状态

- 版本：`0.1.0`，尚未发布
- 仓库：已私有托管于 [snooze26h/SnoozeLine](https://github.com/snooze26h/SnoozeLine)，并已配置 `origin`
- 分发：仅私有托管源码，没有 GitHub Release，也没有 SnoozeLine npm 包
- 安装：已并行安装到 `~/.claude/snoozeline/snoozeline`；Claude Code 已切换使用 SnoozeLine，旧 `~/.claude/ccline` 完整保留用于回滚

## 显示内容

默认状态栏只保留必要信息：

```text
模型 | 文件夹 | 上下文% · tokens | 5h% · 7d% | Git 分支/状态
```

`snooze26h` 主题不显示“共享”字样，也不显示额度重置日期。

## 数据规则

- 优先使用 Claude Code 原生上下文数据。
- 当前上下文 token 只计算输入与缓存输入，不计算输出 token。
- 上下文和额度百分比会校验并限制在 `0–100%`。
- 原生 `5h`、`7d` 均表示**已用百分比**；数据缺失时显示 `-`，不会伪造。
- 原生额度缺失时，可以使用兼容的 Claude 用量接口及账户隔离缓存作为回退。
- SnoozeLine 的缓存不会保存对话正文。

## 运行文件

默认运行根目录为 `~/.claude/snoozeline`：

```text
~/.claude/snoozeline/
├── config.toml
├── models.toml
├── themes/*.toml
└── .api_usage_cache.json
```

可以用绝对路径环境变量 `SNOOZELINE_HOME` 指定其他根目录。SnoozeLine 不会自动移动或删除 `~/.claude/ccline` 中的文件。

## 构建与测试

需要 Rust stable：

```sh
cargo metadata --locked --no-deps --format-version 1
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked
git diff --check
```

只使用原生模拟数据进行本地冒烟测试，模型段应显示 `Fable 5.1`：

```sh
printf '%s\n' '{"model":{"id":"claude-fable-5-1","display_name":"Fable 5.1"},"workspace":{"current_dir":"/tmp/snoozeline-demo"},"context_window":{"context_window_size":1000000,"used_percentage":24,"current_usage":{"input_tokens":242700}},"rate_limits":{"five_hour":{"used_percentage":18},"seven_day":{"used_percentage":4}}}' \
  | SNOOZELINE_HOME=/tmp/snoozeline-smoke \
    ./target/debug/snoozeline --theme snooze26h
```

## 本机安装与迁移

下面的通用迁移方式会让 SnoozeLine 与现有 `ccline` 并存，先备份 Claude 配置，再仅修改状态栏命令。本机已按同样的可回滚方式完成迁移。

```sh
cargo build --release --locked

install -d "$HOME/.claude/snoozeline"
install -m 0755 ./target/release/snoozeline "$HOME/.claude/snoozeline/snoozeline"

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
    '.statusLine = ((.statusLine // {}) + {"type":"command","command":$command,"padding":0})' \
    "$settings_file" > "$temp_file"
  mv "$temp_file" "$settings_file"
  trap - EXIT HUP INT TERM
  printf 'Backup saved to: %s\n' "$backup_file"
)
```

迁移后需要重启 Claude Code。回滚命令：

```sh
backup_file="/exact/backup/path/printed/above"
cp -p "$backup_file" "$HOME/.claude/settings.json"
```

## 许可与来源

上游项目在包元数据与 README 中声明 `MIT`，但 v1.1.2 源码快照缺少其引用的 `LICENSE` 正文。SnoozeLine 不会自行编造许可证文件或版权持有人。在准确的上游许可声明得到确认前，本仓库保持私有。
