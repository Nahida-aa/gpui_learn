- 需要有批判性思维, 可以质疑人类
- 调试日志使用 tracing, 调试之后可以不用清除日志
- 修改代码后, 如果认为适合提交, 就自行提交
- 许可证默认 GPL-3.0-or-later（根目录 license.workspace 继承）；只有确认零 zed
  来源代码的包才在各自 Cargo.toml 显式写 `license = "Apache-2.0"`。新增包沿用这条。

gpui_util, gpui_shared_string 与 gpui 是松耦合

aa_gpui/ 目录下是 gpui 计划进入gpui的功能

aa_gpui_kit/ 目录下是 依赖 gpui(仅gpui_util, gpui_shared_string不算) 的 crate

zed/目录下是 与zed 强相关的 crate

aa/ 目录下 是与 gpui 无关的
