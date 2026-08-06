# gpui_learn 的常用命令快捷方式（https://github.com/casey/just）
# 用法：just run hello_window

# 列出所有可运行的例子（apps 下的二进制包）
list:
    cargo metadata --no-deps --format-version 1 | python3 -c "import sys,json; [print(p['name']) for p in json.load(sys.stdin)['packages'] if any(t=='bin' for t in p.get('targets',[]))]"

# 运行某个例子：just run hello_world_01
run example="hello_world_01":
    cargo run -p {{example}}

# 构建默认成员（apps/*）
build:
    cargo build

# 构建整个工作区（含 crates/* 库）
build-all:
    cargo build --workspace

# 检查（不编译产物，仅类型检查）
check:
    cargo check --workspace

# ---- 09_slider：自研 ui-gpui Slider 演示 ----

# 运行 Slider 演示（GUI 窗口）
run_slider:
    cargo run -p slider_09

# 构建 Slider 演示
build_slider:
    cargo build -p ui-gpui -p slider_09

# 运行 ui-gpui 库测试（含 SliderState 交互单测，需 test-support 特性）
test_slider:
    cargo test -p ui-gpui -F test-support

# 对 ui-gpui 库跑 clippy
lint_slider:
    cargo clippy -p ui-gpui -p slider_09

stats:
    scc . --exclude-dir node_modules,dist,build,target,venv,.venv,__pycache__,.git,vendor,out,cmake-build-debug,CMakeFiles --exclude-ext lock,json,md,yaml,yml,toml,ini,conf
