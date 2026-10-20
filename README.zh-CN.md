# flash.tmux

[English](README.md) | 简体中文

在 tmux copy-mode 中提供增量搜索和字母标签跳转，灵感来自 [folke](https://github.com/folke) 开发的 [flash.nvim](https://github.com/folke/flash.nvim)。核心逻辑使用 Rust 实现，通过轻量的 TPM 入口接入 tmux。

## 截图与操作演示

截图使用 Ghostty、zsh、JetBrains Mono 和 Catppuccin Frappe 主题，展示的是本仓库中的真实文件。以下操作使用 vi copy-mode，并已配置 `v` 开始选择。选择、复制、切换 pane 和分屏按键属于用户的 tmux 配置，不是 flash.tmux 安装的绑定。

### 1. 搜索与字母标签跳转

**操作：** 在仓库目录执行 `head -n 16 src/config.rs`。按 `prefix + [` 进入 copy-mode，按 `s`，输入 `flash`，停在选择标签之前。

**截图效果：** 蓝色 `⚡` 提示栏显示查询词、匹配数和标签数；匹配内容高亮，并在旁边显示粉色单字符标签。按屏幕上显示的标签，就能跳到对应匹配的起点。标签取决于当前光标和可见内容，无需记忆截图中的具体字母。

![搜索 Rust 配置内容，显示匹配高亮、粉色跳转标签和蓝色闪电提示栏](assets/screenshots/1-search.png)

### 2. 通过搜索扩展选区

**操作：** 保持同一份文件输出，在 copy-mode 中从没有活动选区的状态开始：

```text
s → DEFAULTS: → Enter → 0 → v
s → flash-prompt-fg → Enter → j → $
```

第一次搜索定位声明，`0` 移到行首，`v` 设置选择起点。第二次搜索把选区扩展到最后一个配置项附近，再用 `j` 和 `$` 包含结尾的 `];` 行。

**截图效果：** 整段 `DEFAULTS` 配置已被选中，搜索标签已经消失。画面展示的是选区扩展完成的阶段。接下来按你配置的 `y` 复制；使用 tmux-yank 的 `copy-pipe` 等不退出的复制动作时，可以继续留在 copy-mode，重复搜索、选择和复制。

![通过搜索定位两端后，完整选中 DEFAULTS 配置块](assets/screenshots/2-selection.png)

### 3. 仅搜索当前 Pane

**操作：** 退出 copy-mode，用你配置的快捷键左右分屏（本例为 `prefix + h`）。右侧执行 `head -n 15 Cargo.toml`；左侧执行 `head -n 16 src/config.rs`，再进入 copy-mode，按 `s`，输入 `flash-`，停在选择标签之前。

**截图效果：** 只有左侧 pane 暗化并显示匹配高亮、跳转标签；右侧保持正常颜色，没有搜索标签。蓝色 `⚡` 提示栏显示本次查询和匹配、标签数量。

![左侧 pane 进行搜索，右侧 Cargo.toml 的显示保持不变](assets/screenshots/3-split-pane.png)

### 4. 从光标位置扩大语法选区

**操作：** 在仓库根目录执行 `cat tests/snippets/rust.rs`。进入 copy-mode，把光标移到 `42` 上，按 `S`。连续按两次 `;`，先预览 `let_declaration`，再扩大到 `block`；停在按 Enter 确认之前。按 `,` 可以缩小范围，按 Esc 可以取消。

**截图效果：** 函数体处于预览状态，提示栏通过 `[block]` 标明当前节点。粉色标签可以直接选择其他祖先节点。这只是浮层预览，尚未替换 tmux 的实际选区；按 Enter 才会确认。

![S 从 42 扩大到外围 Rust 代码块的预览](assets/screenshots/4-treesitter-range-preview.png)

### 5. 搜索文字并选择外围语法范围

**操作：** 在同一份可见 Rust 示例中，先用 Esc 取消上一例的预览，把光标移到代码的其他位置。按 `R`，输入 `42`，停在选择标签或按 Enter 之前。

**截图效果：** 查询命中 `42`，标签分别对应它的 `integer_literal`、`let_declaration`、`block`、`function_item` 等范围。根据需要选择代码结构，而不只是命中的文字。标签字母以当前界面为准；在这个搜索模式中，`;`、`,` 不是扩大/缩小命令。

![R 搜索 42，同时展示源码和外围语法范围标签](assets/screenshots/5-treesitter-search-labels.png)

### 6. 从搜索命中直接选中完整函数

**操作：** 接着第 5 例，按提示栏中 `function_item` 对应的标签。浮层关闭后，停在复制或取消选区之前。

**截图效果：** 从 `fn main()` 到最后一个 `}` 的完整函数被选中，不包含 shell 命令和提示符，语法标签已经消失。接下来使用你自己的复制绑定即可，不改变原有 tmux 复制流程。

![通过 R 搜索直接确认完整 Rust 函数选区](assets/screenshots/6-treesitter-function-selection.png)

## 环境要求

- tmux 3.3 或更新版本，支持 popup。目前本机验证环境为 macOS arm64、tmux 3.6a。
- 顶部或底部需要有可见的 tmux 状态栏。搜索提示显示在消息栏中，避免遮挡原始内容行。`status off` 时插件会给出错误提示，不修改原 pane。
- 支持真彩色的终端，以显示默认的 Catppuccin Frappe 配色。
- 兼容的预编译程序，或者用于源码构建的 Rust 1.90+、Cargo 和 C 编译器。运行预编译程序不需要安装 Rust；Python 3.11+ 仅用于开发和发布测试。
- 自动安装需要 Git、curl、tar，以及 sha256sum 或 shasum，并且能够访问公开的 GitHub Release。

## 功能特性

- **增量搜索（`s`）**：输入足够的可见文字后，用标签或 Enter 跳转。采用字面匹配、智能大小写和 Unicode 光标定位。
- **不抢占查询输入的标签**：可能继续扩展当前搜索词的字符不会分配为跳转标签，避免输入和选择相互冲突。
- **语法选择（`S`）**：选择光标所在的命名语法节点及其祖先，用 `;` 扩大、`,` 缩小，也可以直接选择范围标签。
- **语法搜索（`R`）**：搜索文字后直接选择对应的声明、代码块或函数，不必先移动到命中位置。
- **内置语言解析器**：提供 15 个 Tree-sitter 解析器，默认自动识别，也支持按窗格指定语言；无需 Neovim 或额外安装 Tree-sitter。
- **保留复制习惯**：普通搜索延伸已有选区，语法选择建立精确的字符选区；复制绑定和剪贴板仍由你的配置控制。
- **当前窗格隔离**：只处理当前窗格的可见内容，不交换或调整原窗格大小。
- **安全预览与取消**：预览不修改真实光标和选区；Esc/Ctrl+C 取消，视口或状态变化会使待执行的语法操作失效。

## 本地开发与源码安装

将 `/path/to/flash.tmux` 替换为你自己的仓库目录：

```sh
git clone https://github.com/y00rb/flash.tmux.git /path/to/flash.tmux
cd /path/to/flash.tmux
cargo build
```

在 `tmux.conf` 中添加以下配置，使用同一个仓库路径：

```tmux
set -g @flash-bin '/path/to/flash.tmux/target/debug/flash-tmux'
run-shell 'sh /path/to/flash.tmux/flash.tmux'
```

重载 tmux 配置以加载插件。修改源码后重新执行 `cargo build`；只更新可执行程序不需要重载，修改绑定或选项则需要。

需要优化构建时，执行 `cargo build --release`，并将 `@flash-bin` 改为 `/path/to/flash.tmux/target/release/flash-tmux`。

显式设置 `@flash-bin` 后始终跳过 Release 下载，即使路径无效也只会报错。加载插件不会运行 Cargo，按 s 搜索也不会构建或下载。

## TPM 与预编译版本

在 TPM 中配置已发布的版本标签：

```tmux
# 放在 TPM 初始化之前，使用已发布的版本标签。
set -g @plugin 'y00rb/flash.tmux#v0.2.0'
```

按 `prefix + I`。TPM 克隆并加载插件后，`flash.tmux` 会把当前源码中 `VERSION` 对应的程序安装到插件的 `bin/` 目录。仓库地址从公开 GitHub 仓库的 HTTPS/SSH origin 中识别，不下载 `latest`，不需要 Rust；已有兼容程序时不联网。私有仓库或需要鉴权的 Release 不支持自动下载。

固定标签安装处于 detached HEAD：TPM 的 `prefix + U` 使用 `git pull`，不会切换到新标签。更新时修改配置中的标签，再在已安装的插件目录明确获取并切换新标签，最后重新加载插件：

```sh
PLUGIN_DIR='/path/to/tpm/plugins/flash.tmux'
git -C "$PLUGIN_DIR" fetch origin tag v0.2.0
git -C "$PLUGIN_DIR" checkout --detach v0.2.0
```

使用实际目录和版本，切换前检查本地修改，不要强制覆盖。加载后检查实际 checkout 与 `bin/flash-tmux --version` 一致。不带 `#tag` 的分支跟踪安装可以使用 `prefix + U` 拉取更新，但要等对应 Release 存在后再更新。插件加载时会同步二进制版本。Release 不存在时会报错，不会让搜索继续使用不兼容的旧程序。

本地开发保留 `@flash-bin`；测试 Release 安装时，从配置中移除这个覆盖，并执行 `tmux set-option -gu @flash-bin` 后再重载。只删除配置行不会清除正在运行的 tmux server 中的选项。下载失败时，等待 Release 公开后重新加载插件，或者在插件目录执行 `sh scripts/ensure.sh` 重试。搜索按键本身不触发下载。

也可以手动安装某个已发布版本：

```sh
# 在安装后的插件目录中执行，替换为实际的仓库和版本标签。
sh scripts/install.sh --version v0.2.0 --repo y00rb/flash.tmux
```

安装脚本会选择对应的 macOS arm64/x86_64 或 Linux arm64/x86_64 musl 程序，验证 SHA-256 和程序版本，然后原子替换 `bin/flash-tmux`，并在 `bin/install-info` 中记录版本与平台。下载、校验或运行失败时会保留原有程序。并发安装会串行处理；若安装进程被强制结束，需要先检查 `bin/.install-lock/pid`，确认进程已退出，再删除遗留锁并重试。不支持的平台或离线环境可以源码构建并设置 `@flash-bin`。

推送 `vX.Y.Z` 标签会触发发布；`VERSION`、Cargo.toml 与标签必须一致。发布依赖检查成功，再构建四个平台、核对程序版本，准备完整的草稿附件后公开 Release。上传失败的草稿可重试；已公开版本不覆盖，修复问题应发布新版本。预发布会标记为 prerelease，普通分支 push 不发布 Release。

Linux musl 构建旨在减少对 glibc 版本的依赖，并不保证适用于所有内核、终端或发行版。校验和用于检测文件损坏，不能防范发布者或发布渠道被入侵。

## 使用方法

1. 进入 tmux copy-mode，通常按 `prefix + [`。
2. 按小写 **s**。
3. 输入当前可见文本，逐步缩小字面量匹配范围。
4. 按显示的标签跳到对应匹配的起点，或按 Enter 选择最近的匹配。
5. 再次按 s，可以开始新的搜索。

搜索提示显示为 `⚡ 查询词  [3 matches / 3 labels]`，默认使用 Frappe Blue 背景。匹配内容使用略微提亮的 Surface1 背景，标签保持 Pink 背景。

Backspace 删除一个字素簇。Esc 或 Ctrl+C 取消搜索，不改变原来的光标和选区。没有匹配时，按 Enter 会继续停留在搜索界面。全部为小写的查询不区分大小写，包含大写字母的查询区分大小写。标签是单个 ASCII 字符，大小写分别对应不同按键。如果某个标签字符可以继续扩展当前任一匹配，它不会被用作标签，以保证继续输入搜索词的行为明确。匹配过多时，可以继续输入以缩小范围。

标签通常显示在匹配文本之后；遇到行边界时，可能改用相邻的安全单元格。如果没有安全位置，该匹配仍会高亮，但不会显示标签；可以继续缩小查询范围，再用 Enter 跳转。搜索不会跨越终端折行后的屏幕行，也不会搜索屏幕外的历史内容。需要先滚动到目标区域，再进行搜索。

原 pane 不会被交换或缩放。窗口尺寸或 copy-mode 状态发生变化时，会取消尚未执行的跳转。同一个 pane 或客户端同时只能运行一次搜索。通过 tmux 关闭搜索 popup，也会保留原来的 copy-mode。

## 选择文本与连续复制

插件不绑定 **v**、**y** 或 **q**，不修改剪贴板设置，也不依赖 tmux-yank。使用你自己的 copy-mode 按键配置即可：

```text
s → 搜索 → 标签 → v → s → 搜索 → 标签 → y
```

已有选区时，跳转会扩展选区。按 y 后是否退出 copy-mode，完全取决于你自己的绑定。如果使用 tmux-yank，可以在 TPM 初始化之前设置：

```tmux
set -g @yank_action 'copy-pipe'
```

这样 y 复制后不会退出 copy-mode，可以反复搜索、选择和复制，直到用 q 退出。后加载的绑定可能覆盖 tmux-yank，可以检查实际生效的绑定：

```sh
tmux list-keys -T copy-mode-vi y
```

**搜索界面打开时，q 和 s 是普通的搜索字符或已分配的标签，不是退出或重新搜索命令。** 如果想用 q 触发原有的 copy-mode 退出绑定，需要先按 Esc 取消搜索。如果用户的 y 绑定会退出 copy-mode，就需要重新进入 copy-mode 后再按 s 搜索。

## 语法范围选择

| 按键 | 用途 | 模式内操作 |
| --- | --- | --- |
| `s` | 搜索文字并跳到起点 | 输入查询，标签或 Enter 确认 |
| `S` | 选择当前光标所在的语法结构 | `;` 扩大、`,` 缩小，标签或 Enter 确认 |
| `R` | 搜索文字并选择对应的语法结构 | 输入查询，选择范围标签或按 Enter |

`S` 和 `R` 是独立入口，不必搭配。光标已在 `42` 上时，用 `S` 从最小节点扩大到声明、
代码块或函数；光标不在目标上时，用 `R` 搜索 `42`，直接选择 `let_declaration` 或
`function_item` 对应的标签。Enter 选择最近命中位置的最小范围。
`R` 中的 `;`、`,` 是查询字符或标签，不是扩大/缩小命令。
如果喜欢逐级调整，也可以使用 `R → 查询 → Enter → S`。

预览不修改原窗格；确认后以字符选区替换原选区，不自动复制或退出 copy-mode。
Esc/Ctrl+C 取消并保留原光标和选区。复制仍用你自己的绑定。
普通 `s` 搜索仍然延伸已有选区。重合的节点边界也会列在提示栏中，标签覆盖屏幕单元，
不插入或移动源码字符。标签字母会随查询变化。

### 语言与 cat 测试示例

内置 Rust、Python、JavaScript、TypeScript、TSX、JSON、Bash、C、C++、Go、Lua、Ruby、
Zig、PHP、Java 共 15 个解析器。不依赖 Neovim、外部 Tree-sitter CLI 或运行时下载。
嵌入解析器的许可证见 THIRD_PARTY_LICENSES.md。

```sh
# 在仓库根目录、需要测试的窗格中执行：
cat tests/snippets/rust.rs
# 进入 copy-mode，把光标移到 value/42 上按 S；或按 R 搜索 42。
# 自动识别有歧义时，显式指定语言：
tmux set-option -p @flash-treesitter-language rust
# 移除窗格覆盖，恢复使用全局配置：
tmux set-option -pu @flash-treesitter-language
```

`auto` 是默认值，正常使用不需要执行语言设置命令。显式设置的窗格覆盖会持续生效，直到修改或取消。

全部文件与命令见 [tests/snippets/README.md](tests/snippets/README.md)。语言配置值为
`rust`、`python`、`javascript`、`typescript`、`tsx`、`json`、`bash`、`c`、`cpp`、`go`、
`lua`、`ruby`、`zig`、`php`、`java`；也接受 `js`、`ts`、`sh`、`rb`、`c++` 别名。
窗格配置优先于全局配置。

`auto` 用真实语法树验证光标附近的内容，不根据源文件名猜测；C/C++ 等公共语法可能有歧义。
局部恢复最多向前检查 32 个逻辑行起点、向后检查 16 个换行，单片段上限为 8 KiB。
识别与解析共享 250ms 预算，源码上限 256 KiB。过长、截断的声明和混合输出可能需要显式语言。
只处理可见文本；视口边缘截断的软换行会被拒绝，不猜测选择坐标。
不支持语言注入、跨窗口操作、编辑器 operator-pending，以及 Neovim 不占屏幕单元的 inline 标签。
目标是核心流程兼容，不宣称完全复刻 flash.nvim。

## 配置

请在 **TPM 初始化或手动加载插件之前**设置选项。插件只为未设置的选项填充默认值，不覆盖用户已有配置。触发键必须互不冲突；插件替换所配置按键的原生 copy-mode 动作，重载时保留用户替换的绑定。

```tmux
set -g @flash-key 's'
set -g @flash-treesitter-key 'S'
set -g @flash-treesitter-search-key 'R'
set -g @flash-treesitter-language 'auto'
set -g @flash-labels 'asdfghjklASDFHJKLqwertyuiopzxcvbnmQWERTYUIOPZXCVBNM'
set -g @flash-label-bg '#f4b8e4'   # Frappe Pink
set -g @flash-label-fg '#303446'   # Base
set -g @flash-match-fg '#c6d0f5'   # Text
set -g @flash-match-bg '#51576d'   # Surface1
set -g @flash-prompt-bg '#8caaee'  # Blue
set -g @flash-prompt-fg '#303446'  # Base
```

颜色使用 `#RRGGBB` 格式。匹配背景色和提示背景色可以设为空字符串，表示使用终端默认背景；前景色和标签背景色不能留空。标签字符必须是不重复、可打印、非空格的 ASCII 字符。触发键支持 tmux 按键名称，只注册在 `copy-mode` 和 `copy-mode-vi` 按键表中，不会拦截 tmux copy-mode 之外的 Neovim 按键。

`@flash-bin` 用于可选地覆盖可执行程序路径，适合本地开发。插件在内部通过 `@flash-bound-key` 记录上次绑定的触发键，在重载时清理旧绑定，同时避免删除用户后来替换的绑定。

## 测试

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build
python3 tests/tmux_probe.py
python3 tests/e2e.py
python3 tests/treesitter_e2e.py
sh tests/plugin.sh
sh tests/install.sh
python3 tests/bootstrap_test.py
python3 tests/release_version_test.py
python3 tests/publish_test.py
python3 scripts/check-release.py --binary target/debug/flash-tmux
```

维护者在推送版本标签前，还应执行 `python3 scripts/check-release.py --tag v0.2.0`，其中版本号替换为实际目标版本。自动安装测试模拟 GitHub 下载，并在独立 tmux server 中验证插件入口；发布测试模拟 `gh`，覆盖草稿恢复和禁止覆盖公开版本，不会实际发布。

Python 终端测试会创建独立、用后销毁的 tmux server 和伪终端，不使用你的真实会话。测试中的剪贴板操作会被替换为写入本地临时文件。环境中存在 `nvim` 时，会执行 Neovim pane 尺寸相关测试。安装测试使用本地模拟下载，不需要网络。

捕获的制表符按标准的每八列一个 tab stop 处理；自定义终端 tab stops 尚未验证。Unicode 测试覆盖中文等 CJK 字符、组合音标、基本 emoji、一个 ZWJ 序列和区域指示符旗帜。不同版本的终端和 tmux 可能使用不同的 Unicode 表；报告显示或跳转偏移问题时，请附上两者的版本。输入和匹配保留原始 Unicode 序列，不会自动归一化组合形式与分解形式。

## 运行时文件

程序在 `$XDG_CACHE_HOME/flash-tmux` 或 `~/.cache/flash-tmux` 下创建私有目录，存放 pane/客户端级别的操作系统文件锁和搜索快照。正常搜索结束后会清理临时快照；同一 pane 下次搜索时，会清理崩溃遗留的快照。少量锁文件会保留以供复用，进程退出时操作系统会自动释放锁。
