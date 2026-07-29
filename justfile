# gpui_learn 的常用命令快捷方式（https://github.com/casey/just）
# 用法：just run hello_window

# 列出所有可运行的例子（apps 下的二进制包）
list:
    cargo metadata --no-deps --format-version 1 | python3 -c "import sys,json; [print(p['name']) for p in json.load(sys.stdin)['packages'] if any(t=='bin' for t in p.get('targets',[]))]"

# 运行某个例子：just run hello_world_01
run example:
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
