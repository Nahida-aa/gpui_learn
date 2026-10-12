# gpui_learn 的常用命令快捷方式（https://github.com/casey/just）
# 用法：just run _01_hello_world

# 注意 default-members 只含 packages/apps/aa-gpui-cli（见根 Cargo.toml），
# 所以裸 `cargo build` / `cargo test` 范围很窄。要全量请用带 --workspace 的 recipe。

# 列出所有可运行的例子（apps 下的二进制包）
list:
    ls apps/

# 运行某个例子：just run _01_hello_world
run example="_01_hello_world":
    cargo run -p {{example}}

# 构建默认成员（packages/apps/aa-gpui-cli）
build:
    cargo build

# 构建整个工作区（48 个示例 + 全部库，首次很久）
build-all:
    cargo build --workspace

# 检查（不编译产物，仅类型检查）
check:
    cargo check --workspace

# 运行 aa_gpui_kit_ui 库测试（需 test-support 特性）
test:
    cargo test -p aa_gpui_kit_ui -F test-support

# 运行整个工作区的测试（含各示例包）
test-all:
    cargo test --workspace

# 对 aa_gpui_kit_ui 跑 clippy
lint:
    cargo clippy -p aa_gpui_kit_ui

# 把 aa-bundle 安装为本地 CLI（~/.cargo/bin/aa-bundle）。
# 改了 aa-bundle 的代码后必须重跑才生效 —— cargo install 是快照式的。
# 下游用法见 ~/repos/ai_ls/AAgent 的 justfile（just install / just uninstall）。
install-cli:
    cargo install --path packages/apps/aa-bundle --force

stats:
    scc . --exclude-dir node_modules,dist,build,target,venv,.venv,__pycache__,.git,vendor,out,cmake-build-debug,CMakeFiles --exclude-ext lock,json,md,yaml,yml,toml,ini,conf
