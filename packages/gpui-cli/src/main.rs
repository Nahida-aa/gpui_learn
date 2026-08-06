//! # gpui-cli —— GPUI 工程的脚手架工具
//!
//! 目前实现 `android init`：读取一个例子的 `Cargo.toml`（取 cargo 包名 / lib 名）
//! 与 `gpui.conf.json`（取 Android `identifier` / `app_name`），生成
//! `gen/android/` 下的完整 Gradle 工程。生成后直接 `./gradlew assembleDebug`。
//!
//! 设计哲学（对齐 Tauri 的 `tauri android init`）：
//! - 配置极简——`gpui.conf.json` 只放真正属于 Android、Cargo.toml 里没有的字段
//!   （identifier / app_name）；cargo 包名与 lib 名从 Cargo.toml 自动读，
//!   不重复声明。
//! - 目标 ABI 用默认值，不在配置里让用户填（Tauri 也是这样，默认全编、可用
//!   命令行参数收窄）。

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::Deserialize;

/// 所有模板以编译期内嵌方式打包，使 gpui-cli 完全自包含。
const TPL_APP_GRADLE: &str = include_str!("../templates/android-app.gradle.kts");
const TPL_SETTINGS: &str = include_str!("../templates/settings.gradle.kts");
const TPL_ROOT_BUILD: &str = include_str!("../templates/root-build.gradle.kts");
const TPL_MANIFEST: &str = include_str!("../templates/AndroidManifest.xml");
const TPL_STYLES: &str = include_str!("../templates/styles.xml");
const TPL_WRAPPER_PROPS: &str = include_str!("../templates/wrapper/gradle-wrapper.properties");
const TPL_GRADLEW: &str = include_str!("../templates/gradlew");
const TPL_GRADLEW_BAT: &str = include_str!("../templates/gradlew.bat");
const TPL_WRAPPER_JAR: &[u8] = include_bytes!("../templates/wrapper/gradle-wrapper.jar");
/// gen/android/.gitignore：只忽略构建产物与拷贝资源，生成的 Gradle 源码纳入版本追踪
/// （对齐 Tauri 的 src-tauri/gen/android/.gitignore）。
const TPL_GITIGNORE: &str = include_str!("../templates/android.gitignore");

/// 默认 ABI 列表：覆盖真机（arm64-v8a）与模拟器（x86_64）。
/// 完整可选集见 `ALL_ABIS`，用 `--targets` 可任意组合。
const DEFAULT_ABIS: &[&str] = &["arm64-v8a", "x86_64"];
/// 安卓上 GPUI 能编的全部 ABI（与 Tauri 默认集一致）。
const ALL_ABIS: &[&str] = &["arm64-v8a", "armeabi-v7a", "x86", "x86_64"];

#[derive(Parser)]
#[command(name = "gpui-cli", about = "GPUI 工程脚手架工具")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Android 工程相关子命令
    Android {
        #[command(subcommand)]
        sub: AndroidCmd,
    },
}

#[derive(Subcommand)]
enum AndroidCmd {
    /// 为指定工程生成 gen/android/ Gradle 工程
    Init {
        /// 工程目录（含 Cargo.toml 与 gpui.conf.json），默认当前目录
        #[arg(short = 'p', long, default_value = ".")]
        project_dir: PathBuf,
        /// 覆盖默认目标 ABI，逗号分隔，如 `--targets arm64-v8a,x86_64`
        #[arg(long, value_delimiter = ',')]
        targets: Option<Vec<String>>,
    },
}

/// gpui.conf.json 的内容（两个字段均可省略，缺省从 Cargo.toml 推导）。
///
/// - `app_name` 缺省 → Cargo.toml 的 `package.name`（用作安卓应用显示名）。
/// - `identifier` 缺省 → `<仓库名>.<package.name>`（安卓包名，反向域名风格）。
///   仓库名取 git 根目录名的 basename（如 gpui_learn），拿不到回落 "gpui_learn"。
///   故连 gpui.conf.json 都不存在也能 init，实现「配置极简」。
#[derive(Deserialize, Default)]
struct GpuiConf {
    identifier: Option<String>,
    app_name: Option<String>,
}

/// 从 Cargo.toml 解析出 [package] name 与 [lib] name
#[derive(Deserialize)]
struct CargoToml {
    package: CargoPackage,
    lib: Option<CargoLib>,
}

#[derive(Deserialize)]
struct CargoPackage {
    name: String,
}

#[derive(Deserialize)]
struct CargoLib {
    name: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Android { sub } => match sub {
            AndroidCmd::Init {
                project_dir,
                targets,
            } => android_init(&project_dir, targets),
        },
    }
}

fn android_init(project_dir: &Path, targets: Option<Vec<String>>) -> Result<()> {
    let project_dir = if project_dir.is_absolute() {
        project_dir.to_path_buf()
    } else {
        std::env::current_dir()?.join(project_dir)
    };
    anyhow::ensure!(
        project_dir.join("Cargo.toml").exists(),
        "找不到 {}/Cargo.toml，请确认工程目录（默认当前目录，可用 -p/--project-dir 指定）",
        project_dir.display()
    );

    // 1) 读 Cargo.toml：cargo_package = [package] name；rust_lib_name = [lib] name（缺省用 package name）
    let cargo_text = std::fs::read_to_string(project_dir.join("Cargo.toml"))
        .with_context(|| "读取 Cargo.toml 失败")?;
    let cargo: CargoToml = toml::from_str(&cargo_text).context("解析 Cargo.toml 失败")?;
    let cargo_package = &cargo.package.name;
    let rust_lib_name = cargo
        .lib
        .as_ref()
        .and_then(|l| l.name.clone())
        .unwrap_or_else(|| cargo_package.clone());

    // 2) 读 gpui.conf.json：identifier / app_name（均可选，缺省从 Cargo.toml 推导）。
    //    连文件都不存在也能 init——实现「配置极简」。
    let conf_path = project_dir.join("gpui.conf.json");
    let conf: GpuiConf = if conf_path.exists() {
        let conf_text =
            std::fs::read_to_string(&conf_path).with_context(|| "读取 gpui.conf.json 失败")?;
        serde_json::from_str(&conf_text).context("解析 gpui.conf.json 失败")?
    } else {
        GpuiConf::default()
    };
    // 字段缺省推导：`app_name` → 包名；`identifier` → <仓库名>.<包名>。
    // 仓库名取 git 根目录名（如 gpui_learn）。若不在 git 仓库内（拿不到 .git），
    // 直接报错——不硬编码假名，避免默认包名语义失真。
    // repo_name 与 cargo_package 都过 pkg_segment 规范化，保证拼出的 identifier
    // 是合法 Android applicationId（每段仅 ASCII 字母/数字/下划线、小写、不以数字开头）。
    let repo_name = repo_name_from(&project_dir)
        .context("无法确定仓库名来推导默认 identifier：当前目录不在 git 仓库内（找不到 .git），请在 git 仓库中运行，或在 gpui.conf.json 显式写 identifier")?;
    let app_name = conf
        .app_name
        .unwrap_or_else(|| cargo_package.clone());
    let identifier = conf.identifier.unwrap_or_else(|| {
        format!("{}.{}", pkg_segment(&repo_name), pkg_segment(&cargo_package))
    });

    // 3) 解析目标 ABI
    let abis = match &targets {
        Some(t) => {
            for a in t {
                anyhow::ensure!(
                    ALL_ABIS.contains(&a.as_str()),
                    "未知 ABI `{a}`。可选：{}",
                    ALL_ABIS.join(", ")
                );
            }
            t.clone()
        }
        None => DEFAULT_ABIS.iter().map(|s| s.to_string()).collect(),
    };
    let targets_literal = abis
        .iter()
        .map(|a| format!("\"{a}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let root_project_name = format!("GPUILearn{}", sanitize_ident(&app_name));

    // 4) 生成 gen/android/
    let out = project_dir.join("gen/android");

    // 3.5) 找到 vendored gpui-android 的 Java 源码目录，并算出相对 gen/android/app/ 的路径。
    // 这个路径因例子所处层级不同而变化，必须按实际位置计算，不能写死 `../../../../`。
    let android_java_src = find_android_java_src(&project_dir)
        .context("找不到 crates/gpui-android/android/src/main/java，请确认 gpui-android 已 vendored 在仓库内")?;
    let app_dir = out.join("app");
    let android_java_dir = relative_path(&app_dir, &android_java_src)
        .context("计算 gpui-android Java 源码相对路径失败")?;
    println!("引用 gpui-android Java 源码：{android_java_dir}");

    println!("生成 Android 工程到 {}", out.display());
    write_file(
        &out.join("settings.gradle.kts"),
        &TPL_SETTINGS.replace("{ROOT_PROJECT_NAME}", &root_project_name),
    )?;
    write_file(&out.join("build.gradle.kts"), TPL_ROOT_BUILD)?;
    write_file(
        &out.join("app/build.gradle.kts"),
        &TPL_APP_GRADLE
            .replace("{RUST_LIB_NAME}", &rust_lib_name)
            .replace("{CARGO_PACKAGE}", cargo_package)
            .replace("{TARGETS}", &targets_literal)
            .replace("{ANDROID_JAVA_DIR}", &android_java_dir),
    )?;
    write_file(&out.join("app/src/main/AndroidManifest.xml"), TPL_MANIFEST)?;
    write_file(&out.join("app/src/main/res/values/styles.xml"), TPL_STYLES)?;
    // gradle.properties：appId / appName + 通用 Android 配置
    let props = format!(
        "appId={}\nappName={}\nandroid.useAndroidX=true\nandroid.nonTransitiveRClass=true\norg.gradle.jvmargs=-Xmx2048m -Dfile.encoding=UTF-8\n",
        identifier, app_name
    );
    write_file(&out.join("gradle.properties"), &props)?;
    // wrapper
    write_file(
        &out.join("gradle/wrapper/gradle-wrapper.properties"),
        TPL_WRAPPER_PROPS,
    )?;
    write_bytes(
        &out.join("gradle/wrapper/gradle-wrapper.jar"),
        TPL_WRAPPER_JAR,
    )?;
    write_file(&out.join("gradlew"), TPL_GRADLEW)?;
    write_file(&out.join("gradlew.bat"), TPL_GRADLEW_BAT)?;
    set_executable(&out.join("gradlew"))?;
    // gen/android/.gitignore：构建产物/拷贝资源忽略，Gradle 源码纳入追踪。
    write_file(&out.join(".gitignore"), TPL_GITIGNORE)?;

    // 复制例子自带的 assets/（如 emoji 字体）到生成的 app/src/main/assets/。
    // 例子目录里放 assets/fonts/NotoColorEmoji.ttf 即可，无需手动拷进 gen/。
    let src_assets = project_dir.join("assets");
    if src_assets.is_dir() {
        copy_dir(&src_assets, &out.join("app/src/main/assets"))
            .with_context(|| "复制 assets/ 失败")?;
        println!("已复制例子 assets/ → app/src/main/assets/");
    }

    println!("完成。接下来：");
    println!("  cd {}", out.join("gradle").display());
    println!("  ./gradlew assembleDebug");
    Ok(())
}

/// 从工程目录向上查找 vendored 的 gpui-android Java 源码根
/// (`.../crates/gpui-android/android/src/main/java`)，返回其绝对路径。
fn find_android_java_src(project_dir: &Path) -> Option<PathBuf> {
    let candidate = Path::new("crates/gpui-android/android/src/main/java");
    let mut dir = if project_dir.is_absolute() {
        project_dir.to_path_buf()
    } else {
        std::env::current_dir().ok()?.join(project_dir)
    };
    loop {
        let try_path = dir.join(candidate);
        if try_path.is_dir() {
            return Some(try_path);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// 计算 `from` 到 `to` 的相对路径（用 `../` 形式），用于在生成文件里引用。
fn relative_path(from: &Path, to: &Path) -> Option<String> {
    use std::path::Component;
    let from = from.components().collect::<Vec<_>>();
    let to = to.components().collect::<Vec<_>>();
    // 找到公共前缀长度
    let mut common = 0;
    while common < from.len() && common < to.len() && from[common] == to[common] {
        common += 1;
    }
    let mut rel = Vec::new();
    // 从 from 退到公共前缀
    for _ in common..from.len() {
        rel.push(Component::Normal(std::ffi::OsStr::new("..")));
    }
    // 再从公共前缀走到 to
    for &c in &to[common..] {
        rel.push(c);
    }
    let mut s = String::new();
    for (i, c) in rel.iter().enumerate() {
        if i > 0 {
            s.push('/');
        }
        s.push_str(c.as_os_str().to_str()?);
    }
    Some(s)
}

fn sanitize_ident(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .split('_')
        .filter(|p| !p.is_empty())
        .take(3)
        .collect::<Vec<_>>()
        .join("")
}

/// 推导仓库名（用于默认 identifier 的命名空间前缀）。
///
/// 优先用 `git rev-parse --show-toplevel` 拿仓库根目录名；若 git 不可用，
/// 则向上回溯目录树找第一个含 `.git` 的祖先，取其目录名。**两者都拿不到
/// （不在 git 仓库内）返回 None**——调用方据此报错，不兜底假名。
fn repo_name_from(project_dir: &Path) -> Option<String> {
    // 1) git 可用时直接用仓库根目录名（最可靠）。
    if let Some(name) = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(project_dir)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| {
            Path::new(s.trim())
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default()
        })
        .filter(|s| !s.is_empty())
    {
        return Some(name);
    }

    // 2) git 不可用：向上找含 .git 的祖先目录，取其目录名。
    let mut dir = project_dir.canonicalize().ok()?;
    loop {
        if dir.join(".git").exists() {
            return dir
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .filter(|s| !s.is_empty());
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

/// 把一个字符串规范成 Android `applicationId` 的「合法段」：
/// - 仅保留 ASCII 字母/数字/下划线，其余（含 `-`、空格、中文、点等）替换为 `_`；
/// - 整体转小写（反向域名惯例全小写）；
/// - 若段以数字开头，前缀补 `_`（Java 标识符不能以数字开头）。
/// 仓库目录名可能含 `-`/大写，包名（cargo package name）应以同样规则约束，
/// 故 repo_name 与 cargo_package 都过这一道，保证拼出的 identifier 合法。
fn pkg_segment(s: &str) -> String {
    let mut seg: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    if seg.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        seg.insert(0, '_');
    }
    seg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkg_segment_normalizes_repo_and_package_names() {
        // 大写 + 连字符 → 小写 + 下划线
        assert_eq!(pkg_segment("My-Repo2"), "my_repo2");
        // 纯小写带连字符
        assert_eq!(pkg_segment("gpui-learn"), "gpui_learn");
        // 以数字开头 → 前缀补 _
        assert_eq!(pkg_segment("3repo"), "_3repo");
        // 中文等非 ASCII → 下划线（4 个汉字 → 4 个下划线）
        assert_eq!(pkg_segment("我的仓库"), "____");
        // 合法名不变
        assert_eq!(pkg_segment("uniform_list_07"), "uniform_list_07");
        // 混合：数字开头 + 非法字符
        assert_eq!(pkg_segment("2-My.App"), "_2_my_app");
    }
}

fn write_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("创建目录 {}", parent.display()))?;
    }

    std::fs::write(path, content).with_context(|| format!("写文件 {}", path.display()))?;
    Ok(())
}

fn write_bytes(path: &Path, content: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("创建目录 {}", parent.display()))?;
    }
    std::fs::write(path, content).with_context(|| format!("写文件 {}", path.display()))?;
    Ok(())
}

/// 递归复制目录（用于把例子的 assets/ 带进生成的工程）。
fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to).with_context(|| format!("创建目录 {}", to.display()))?;
    for entry in std::fs::read_dir(from).with_context(|| format!("读目录 {}", from.display()))? {
        let entry = entry?;
        let path = entry.path();
        let dest = to.join(entry.file_name());
        if path.is_dir() {
            copy_dir(&path, &dest)?;
        } else {
            std::fs::copy(&path, &dest).with_context(|| format!("复制 {} 失败", path.display()))?;
        }
    }
    Ok(())
}
#[cfg(unix)]

fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> Result<()> {
    Ok(())
}
