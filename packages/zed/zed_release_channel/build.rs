fn main() {
    // lib.rs 用 `option_env!("ZED_RELEASE_CHANNEL")` 读编译期值，改了要重编。
    // 原本这里还会在检测到该 env 时置 `__do_not_set_zed_release_channel` cfg，
    // 用于在 zed 的 `env!` / `include_str!` 两个分支间二选一；aacode 把两条路
    // 合并成 `option_env!` + 默认 dev 之后，该 cfg 已无对应分支，故删除。
    println!("cargo::rerun-if-env-changed=ZED_RELEASE_CHANNEL");
}