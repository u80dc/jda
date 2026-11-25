# jda: Java Heap Dump Analyzer

jda 是使用 Rust 编写的 Java 堆转储分析工具，采用交互控制台。项目目标是在有限内存下分析大型 HPROF 文件，后续将采用流式解析、磁盘索引与按需查询。

## 当前功能

- 接收一个文件路径参数，检查文件是否可读取且为普通文件。
- 启动时显示文件摘要，使用 `jda>` 提示符输入命令；输出保留在终端历史中，不使用全屏或线框。
- 支持 `/help` 查看帮助、`/info` 查看文件信息（含易读大小和精确字节数）。
- 输入 `/q` 或 `/Q` 后按 Enter 退出（忽略大小写及首尾空白）；也支持 Esc、Ctrl+C 和 Ctrl+D（空输入时）。
- 支持左右移动、Backspace/Delete 编辑、上下键历史、Tab 命令补全。
- 命令历史仅存于内存，最多 100 条；提交超过 256 字节的命令会被拒绝，不进入历史。编辑中的输入由行编辑器管理。
- 正常退出或返回错误时恢复终端状态。

当前版本只读取文件元信息，不读取堆内容，也不验证 HPROF 格式。类统计、对象查询、引用分析等功能尚未实现。

## 环境要求

- Rust 稳定版工具链（当前开发环境为 Rust 1.99.1；依赖版本由 Cargo.lock 固定）。
- 支持交互输入的终端，例如 Windows Terminal。

## 运行

```bash
cargo run -- path/to/app.hprof
```

文件路径包含空格时，请使用引号：

```powershell
cargo run -- "C:\dump files\app.hprof"
```

构建发布版本：

```bash
cargo build --release
```

Windows 下运行：

```powershell
.\target\release\jda.exe "C:\dumps\app.hprof"
```

查看帮助或版本（无需提供文件路径，显示后直接退出）：

```bash
cargo run -- -h
cargo run -- --help
cargo run -- -V
cargo run -- --version
```

## 依赖

| 依赖 | 用途 |
| --- | --- |
| clap | 命令行参数解析 |
| rustyline | 行编辑、命令历史、补全和键盘绑定 |
| anyhow | 错误上下文与传播 |

## 开发检查

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## 控制台示例

```text
jda · Java Heap Dump Analyzer

File  D:\dumps\app.hprof
Size  2.31 GiB (2480343613 bytes)

jda> /info
File  D:\dumps\app.hprof
Size  2.31 GiB (2480343613 bytes)

jda> /q
Bye.
```

手动验证：执行 `/help` 和 `/info` 后确认输出保留；使用上下键召回命令，左右键编辑，输入 `/h` 后按 Tab 补全。分别重新运行，验证 `/q`、`/Q` 和 Esc 均能退出并正常返回终端。

## 命令提示

输入 `/` 时会在输入右侧显示 `[/help  /info  /q]`，列表仅供查看，不会作为命令插入。输入 `/h` 或 `/H` 时，会显示剩余部分 `elp`，组合呈现为 `/help`；按右方向键接受唯一提示，或按 Tab 补全。

命令统一使用 `/` 前缀：`/help` 查看帮助、`/info` 查看文件信息、`/q` 退出，忽略 ASCII 大小写。提示只在光标位于输入末尾时显示，按 Enter 执行的是已经输入或接受补全的文本。

若终端环境设置了 `TERM=dumb`，行编辑器会退回普通输入模式，提示、补全及 Esc 绑定不可用。在 Windows Terminal 等支持交互的终端中，可以为当前 PowerShell 会话设置：

```powershell
$env:TERM = "xterm-256color"
cargo run -- README.md
```

## 模块化设计

后续按职责拆分模块，方便独立增改功能。当前 `main.rs` 负责启动，`cli.rs` 负责参数解析，`console.rs` 负责交互；随着功能迭代，逐步分离以下职责：

| 职责 | 设计边界 |
| --- | --- |
| 启动与参数 | 解析参数、组装模块、启动应用 |
| 交互控制台 | 输入编辑、历史、提示及输出协调 |
| 命令处理 | 统一命令定义，供执行、帮助和补全复用 |
| HPROF 解析 | 流式读取、格式校验及记录解码 |
| 磁盘存储 | 对象索引、临时文件及有界缓存 |
| 分析与报告 | 算法返回类型化结果，由报告模块负责展示和导出 |

解析、存储及分析模块独立于终端库，避免跨模块复制大型数据。模块在实际功能需要时建立；开发约定见 [AGENTS.md](AGENTS.md)。
