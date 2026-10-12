# ssh-mgr

[English](README.md) | 简体中文

使用 Rust 编写的终端 SSH 管理工具。配置目录形成分组树，通过系统 OpenSSH 和 tmux 打开独立连接窗口。

## 安装与启动

需要 Rust stable、OpenSSH 和 tmux。macOS 可以使用 `brew install tmux` 安装 tmux。

```bash
cargo build --release
./target/release/ssh-mgr --lang zh
```

程序在固定的 `ssh-mgr` tmux session 中运行。默认扫描 `~/.config/ssh-mgr/`；目录不存在时初始化内置示例，已有文件不会被覆盖。

```bash
ssh-mgr --lang en
ssh-mgr --lang zh --config ./examples/config
SSH_MGR_LANG=zh ssh-mgr
```

语言参数优先于 `SSH_MGR_LANG`，默认英语。已运行的 TUI 需要退出重启才能更换语言。

## 配置

每个文件支持多个 `[[server]]`，文件夹路径作为分组名称。

```toml
[[server]]
name = "api"
host = "example.com"
user = "ubuntu"
port = 22
 description = "API 服务"
tags = ["api"]

[server.password]
key = "~/.ssh/id_ed25519"
```

认证来源至多配置一个：`key`（私钥）、`env`（密码环境变量名称）、`value`（明文密码）、`prompt = true`（OpenSSH 交互输入）。未指定时使用 OpenSSH 默认认证流程。

`env/value` 使用临时 askpass 辅助程序；密码文件权限为 `0600`。SSH 正常退出后清理临时目录。请勿把真实凭据或私钥提交到仓库。

## 本地环境变量文件

启动时自动读取可选文件 `~/.config/ssh-mgr/env`，即使使用 `--config` 也会读取。已有进程环境变量优先，文件不会覆盖它们。TUI 在 tmux 中启动时会再次加载，因此已有 tmux 服务也可以使用该文件。

```dotenv
# ~/.config/ssh-mgr/env
SSH_MGR_PROD_PASSWORD='你的密码'
SSH_MGR_LANG=zh
```

在 TOML 中引用：

```toml
[server.password]
env = "SSH_MGR_PROD_PASSWORD"
```

支持 dotenv 格式的注释、引号和 `export NAME=value`；文件作为数据解析，不执行 shell 命令。建议执行 `chmod 600 ~/.config/ssh-mgr/env`，并避免将文件提交到仓库。格式错误会阻止启动，错误信息不输出密码内容。

修改 env 文件后需退出并重新启动 TUI；`r` 只重新加载 TOML。

## 快捷键

| 按键 | 操作 |
| --- | --- |
| ↑/↓、j/k | 列表上下移动 |
| ←/→、h/l | 折叠/展开分组；折叠子项时回到父组 |
| o、Enter | 创建 SSH 窗口并进入 |
| O | 后台创建 SSH 窗口，保留当前 TUI |
| / | 编辑搜索词 |
| Enter（搜索中） | 确认过滤结果 |
| Esc | 清除搜索或关闭帮助 |
| r | 重新加载配置 |
| ? | 打开帮助 |
| q | 退出 |

搜索输入时字母作为搜索内容，方向键选择结果；确认搜索后使用字母快捷键导航和连接。

## 界面截图

以下为此前在 Ghostty 中拍摄的界面，尚未更新为国际化版本。

![主界面](docs/screenshots/ghostty-dashboard.png)
![搜索](docs/screenshots/ghostty-search.png)
![帮助](docs/screenshots/ghostty-help.png)

## 开发与贡献

```bash
cargo fmt --all -- --check
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

贡献要求见 [CONTRIBUTING.md](CONTRIBUTING.md)，安全说明见 [SECURITY.md](SECURITY.md)，项目采用 [MIT 许可证](LICENSE)。
