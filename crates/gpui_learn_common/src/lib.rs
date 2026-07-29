//! # gpui_learn_common
//!
//! 这是 gpui_learn 工作区里的**共享库（library crate）**。
//!
//! ## 它存在的意义（monorepo 视角）
//!
//! 在一个 monorepo 里，通常会有很多个「二进制例子」（`apps/*`），
//! 它们都要配置同样的 GPUI 依赖、同样的平台特性、同样的窗口初始化样板。
//! 如果每段样板都复制到每个例子里，版本一升级就要改 N 处，极易漂移。
//!
//! 所以这里把「GPUI 的接入方式」集中封装成一个库：
//!
//! - `apps/*` 只需要 `gpui_learn_common = { path = "../../crates/gpui_learn_common" }`，
//!   就能拿到配好的 GPUI，而不必关心 git 源、rev、feature 这些细节。
//! - 这正是 Rust monorepo「共享包」的标准玩法：`path` 依赖指向同仓库内的另一个 crate。
//!
//! ## 怎么用
//!
//! 看 [`run_app`] —— 它封装了 `gpui::App::new().run()` 的最小样板，
//! 每个例子一行就能启动自己的根 View。

// 把 gpui 整个重新导出，这样例子里写 `gpui_learn_common::gpui::*` 即可，
// 不必再在自己的 Cargo.toml 里声明 gpui 依赖。
pub use gpui;
pub use gpui_macros;
// 把 gpui_platform 重新导出，例子用它拿 application() 入口（已在 run_app 内封装）。
pub use gpui_platform;

/// 启动一个 GPUI 应用的最小样板（封装版，供「共享库」演示例子使用）。
///
/// `root` 是一个闭包，接收 `&mut App`，返回一个实现了 [`gpui::Render`] 的根 View 的 Entity。
/// 例子调用 `run_app(|cx| cx.new(|_| HelloWorld::new()))` 即可。
///
/// 注意：第一个例子 `apps/01_hello_world` **没有**用这个函数，而是直接写
/// `application().run(...)`，以便学习者看清 GPUI 原始 API。
/// 本函数存在的意义是「后续演示 monorepo 共享包」——把重复样板收敛到一处。
pub fn run_app<F, R>(root: F)
where
    F: FnOnce(&mut gpui::App) -> gpui::Entity<R> + 'static,
    R: gpui::Render + 'static,
{
    // 入口用 gpui_platform::application().run，与 01_hello_world 里写法一致，
    // 只是把「开窗口 + new 根 View」收敛成了一个参数。
    gpui_platform::application().run(move |cx: &mut gpui::App| {
        let window = gpui::WindowOptions::default();
        // open_window 的闭包接收两个参数：(&mut Window, &mut App)。
        // run 的闭包返回 ()，所以这里用 unwrap 而非 ?。
        cx.open_window(window, |_window, cx| root(cx)).unwrap();
    })
}
