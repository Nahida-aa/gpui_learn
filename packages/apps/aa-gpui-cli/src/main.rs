//! # aa-gpui-cli —— GPUI 工程的脚手架工具
//!
//! 目前实现 `android init`：读取一个例子的 `Cargo.toml`（取 cargo 包名 / lib 名），
//! 并按需经 `--bundle-meta` 读取身份表（`identifier` / `name`），生成
//! `gen/android/` 下的完整 Gradle 工程。生成后直接 `./gradlew assembleDebug`。
//!
//! 设计哲学（对齐 Tauri 的 `tauri android init`）：
//! - 配置极简——身份表只放真正属于平台、Cargo.toml 里没有的字段
//!   （identifier / name）；两者**均可缺省**，缺的从 Cargo.toml 推导。
//!   cargo 包名与 lib 名从 Cargo.toml 自动读，不重复声明。
//! - **没有魔法配置文件**：身份来源必须用 `--bundle-meta` 显式指定，
//!   不隐式读取任何约定文件名。
//! - 目标 ABI 用默认值，不在配置里让用户填（Tauri 也是这样，默认全编、可用
//!   命令行参数收窄）。

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use serde::Deserialize;

/// 所有模板以编译期内嵌方式打包，使 aa-gpui-cli 完全自包含。
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
#[command(name = "aa-gpui-cli", about = "GPUI 工程脚手架工具")]
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
        /// 工程目录（含 Cargo.toml），默认当前目录
        #[arg(short = 'p', long, default_value = ".")]
        project_dir: PathBuf,
        /// 身份来源，两种形式：`<文件>`（文件根即身份表）或
        /// `<文件>#[k1, k2, ...]`（数组路径下钻到目标表）。支持 .toml / .json，
        /// 相对路径基于 -p 工程目录。表内 identifier / name 均可缺省。
        /// 例：`gpui.conf.json`、`Cargo.toml#[package, metadata, bundle-stable]`
        #[arg(long)]
        bundle_meta: Option<String>,
        /// 覆盖默认目标 ABI，逗号分隔，如 `--targets arm64-v8a,x86_64`
        #[arg(long, value_delimiter = ',')]
        targets: Option<Vec<String>>,
    },
}

/// 身份表：`--bundle-meta` 指向的结构（两个字段均可缺省，缺的从 Cargo.toml 推导）。
///
/// 字段名**对齐 cargo-bundle 的 `BundleSettings`**（`identifier` / `name`），
/// 因此可以直接指向 Cargo.toml 的 `[package.metadata.bundle*]` 表，
/// 或任何根级含这两个 key 的 JSON 文件。
#[derive(Deserialize, Default, Debug)]
struct IdentityTable {
    identifier: Option<String>,
    name: Option<String>,
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
                bundle_meta,
                targets,
            } => android_init(&project_dir, bundle_meta.as_deref(), targets),
        },
    }
}

fn android_init(
    project_dir: &Path,
    bundle_meta: Option<&str>,
    targets: Option<Vec<String>>,
) -> Result<()> {
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

    // 2) 读身份表（--bundle-meta，可选）：identifier / name 均可缺省，
    //    缺的走下面的 Cargo.toml 推导。不传 --bundle-meta 就是纯推导。
    let identity = match bundle_meta {
        Some(reference) => load_identity_table(reference, &project_dir)?,
        None => IdentityTable::default(),
    };
    // 字段缺省推导（repo_name 与 cargo_package 都过 pkg_segment 规范化，拼出的
    // identifier 保证合法）：`name` → 包名；`identifier` → <仓库名>.<包名>。
    // 仓库名取 git 根目录名（如 gpui_learn）。若不在 git 仓库内（拿不到 .git），
    // 直接报错——不硬编码假名，避免默认包名语义失真。
    let repo_name = repo_name_from(&project_dir).context(
        "无法确定仓库名来推导默认 identifier：当前目录不在 git 仓库内（找不到 .git），\
         请在 git 仓库中运行，或用 --bundle-meta 指定含 identifier 的身份来源",
    )?;
    let app_name = identity.name.unwrap_or_else(|| cargo_package.clone());
    let identifier = identity.identifier.unwrap_or_else(|| {
        format!(
            "{}.{}",
            pkg_segment(&repo_name),
            pkg_segment(&cargo_package)
        )
    });
    // 统一校验最终 identifier（无论来自 conf 还是自动推导）：段必须以字母开头、
    // 仅含字母/数字/下划线；用户显式写的非法值直接报错，避免静默写坏包名。
    validate_identifier(&identifier)?;

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
        .context("找不到 packages/gpui-android/android/src/main/java，请确认 gpui-android 已 vendored 在仓库内")?;
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

/// 解析 `--bundle-meta` 引用并读出身份表。
///
/// 语法：`<文件>` 或 `<文件>#[k1, k2, ...]`。
/// - 文件部分相对工程目录解析；类型按扩展名分派（.toml / .json）。
/// - 省略 `#[...]` 时取文件根作为表；给了路径则逐层下钻。
/// - 表内 `identifier` / `name` 均可缺省（返回 `None`，由调用方从 Cargo.toml
///   推导）；但**引用本身必须解析成功**——文件缺失、路径不存在、中途或目标
///   不是表、字段存在却不是字符串，都硬报错。不静默回落：路径写错、文件被移动
///   这类漂移必须立刻暴露，而不是悄悄用推导值生成一个错的包名。
fn load_identity_table(reference: &str, project_dir: &Path) -> Result<IdentityTable> {
    // 拆「文件」与「结构路径」。分隔符用 `#` 而非 `:`——后者会与 Windows 盘符
    // （`C:\...`）碰撞；路径用数组形式而非点分——JSON key 合法含点，且
    // identifier 本身（如 `dev.aa.AaCode`）就含点，点分在这里会歧义。
    let (file_part, path_part) = match reference.split_once('#') {
        Some((file, path)) => {
            let path = path.trim();
            // 路径规范写法是 `[k1, k2, ...]`：剥掉首尾方括号（省略方括号也容忍）；
            // 只写半边说明是笔误，直接报错而不是猜。
            let inner = match (path.starts_with('['), path.ends_with(']')) {
                (true, true) => &path[1..path.len() - 1],
                (false, false) => path,
                _ => bail!(
                    "--bundle-meta 的结构路径方括号不配对：`{reference}`\
                     （应为 `文件#[k1, k2, ...]`）"
                ),
            };
            (file.trim(), Some(inner))
        }
        None => (reference.trim(), None),
    };
    anyhow::ensure!(
        !file_part.is_empty(),
        "--bundle-meta 引用的文件名为空：`{reference}`"
    );

    let file_path = project_dir.join(file_part);
    let text = std::fs::read_to_string(&file_path)
        .with_context(|| format!("读取 --bundle-meta 指向的文件失败：{}", file_path.display()))?;

    // 按扩展名把文件解析成统一的 JSON 值树（TOML 也转成 JSON 值，路径遍历只有一套逻辑）
    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mut value: serde_json::Value = match ext.as_str() {
        "toml" => {
            let toml_value: toml::Value = toml::from_str(&text)
                .with_context(|| format!("解析 {} 失败（TOML）", file_path.display()))?;
            serde_json::to_value(toml_value).context("TOML 值转 JSON 值失败")?
        }
        "json" => serde_json::from_str(&text)
            .with_context(|| format!("解析 {} 失败（JSON）", file_path.display()))?,
        other => bail!(
            "--bundle-meta 不支持的文件类型 `.{other}`（仅 .toml / .json）：{}",
            file_path.display()
        ),
    };

    // 逐层下钻结构路径
    let mut walked: Vec<&str> = Vec::new();
    if let Some(path) = path_part {
        for key in path.split(',') {
            let key = key.trim();
            anyhow::ensure!(
                !key.is_empty(),
                "--bundle-meta 的结构路径含空 key：`{reference}`"
            );
            let object = value.as_object().with_context(|| {
                format!(
                    "--bundle-meta `{reference}`：下钻 key `{key}` 时所在层级不是表/对象（实际是 {}）",
                    value_type_name(&value)
                )
            })?;
            walked.push(key);
            value = object.get(key).cloned().with_context(|| {
                let parent = if walked.len() > 1 {
                    walked[..walked.len() - 1].join(" > ")
                } else {
                    "文件根".to_string()
                };
                format!("--bundle-meta `{reference}`：{parent} 下找不到 key `{key}`")
            })?;
        }
    }

    let table = value.as_object().with_context(|| {
        format!(
            "--bundle-meta `{reference}`：目标不是表/对象（实际是 {}），无法读取 identifier / name",
            value_type_name(&value)
        )
    })?;

    // 字段存在但类型不对 → 硬错误；缺省 → None，由调用方推导。
    // 只认字符串：把数字/布尔静默转成字符串会掩盖配置错误。
    let string_field = |key: &str| -> Result<Option<String>> {
        match table.get(key) {
            None => Ok(None),
            Some(serde_json::Value::String(s)) => Ok(Some(s.clone())),
            Some(other) => bail!(
                "--bundle-meta `{reference}`：字段 `{key}` 应为字符串，实际是 {}",
                value_type_name(other)
            ),
        }
    };
    Ok(IdentityTable {
        identifier: string_field("identifier")?,
        name: string_field("name")?,
    })
}

/// JSON 值的中文类型名，用于错误消息。
fn value_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "布尔值",
        serde_json::Value::Number(_) => "数字",
        serde_json::Value::String(_) => "字符串",
        serde_json::Value::Array(_) => "数组",
        serde_json::Value::Object(_) => "表/对象",
    }
}

/// 从工程目录向上查找 vendored 的 gpui-android Java 源码根
/// (`.../packages/gpui-android/android/src/main/java`)，返回其绝对路径。
fn find_android_java_src(project_dir: &Path) -> Option<PathBuf> {
    // gpui-android 的规范位置是仓库根的 packages/（见根 Cargo.toml 的 members）。
    // 兼容旧的 crates/ 布局以平滑迁移，但以 packages/ 为准。
    for candidate in [
        "packages/gpui-android/android/src/main/java",
        "crates/gpui-android/android/src/main/java",
    ] {
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
                break;
            }
        }
    }
    None
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

/// 把一个字符串规范成 Android `applicationId` 的「合法段」(`[A-Za-z][A-Za-z0-9_]*`)：
/// - 仅保留 ASCII 字母/数字/下划线，其余（含 `-`、空格、中文、点等）替换为 `_`；
/// - 整体转小写（反向域名惯例全小写）；
/// - **剔除开头所有非字母**（数字/下划线不能作段首：Android 段必须为字母开头，而
///   不是给它们补 `_` —— 补 `_` 后仍是非法段）。如 `_09_a11y` → `a11y`、`3repo` → `repo`；
/// - 若剔除后不含任何字母（纯符号/中文名），兜底为单段 `x`，保证拼出的 identifier
///   段非空且合法。
///
/// 仓库目录名可能含 `-`/大写，包名（cargo package name）应以同样规则约束，
/// 故 repo_name 与 cargo_package 都过这一道。
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
    while seg
        .chars()
        .next()
        .map(|c| !c.is_ascii_alphabetic())
        .unwrap_or(false)
    {
        seg.remove(0);
    }
    if seg.is_empty() {
        seg.push('x');
    }
    seg
}

/// 校验完整 identifier（`a`本身、各段 `.` 分隔）：每段须 `[A-Za-z][A-Za-z0-9_]*`。
/// 来自 `--bundle-meta` 或自动推导的 identifier 都走这里——非法时**报错**而非静默改写，
/// 避免悄悄改变用户预期的包名。
fn validate_identifier(id: &str) -> Result<()> {
    let ok = id.split('.').all(|seg| {
        let mut chars = seg.chars();
        let first_ok = chars
            .next()
            .map(|c| c.is_ascii_alphabetic())
            .unwrap_or(false);
        first_ok && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    });
    if !ok {
        bail!(
            "identifier `{id}` 不是合法 Android applicationId：每段必须以字母开头、\
             仅含字母/数字/下划线。请修改 --bundle-meta 指向的身份表里的 identifier，\
             或删掉该项以按 <仓库名>.<包名> 自动推导。"
        );
    }
    Ok(())
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
        // 以数字开头 → 剔除数字，不能前缀补 _
        assert_eq!(pkg_segment("3repo"), "repo");
        // 中文等非 ASCII → 下划线；剔光后兜底为单个合法段
        assert_eq!(pkg_segment("我的仓库"), "x");
        // 合法名不变
        assert_eq!(pkg_segment("uniform_list_07"), "uniform_list_07");
        // 混合：下划线/数字开头 + 非法字符 → 剔除前导非字母
        assert_eq!(pkg_segment("_09_a11y"), "a11y");
        assert_eq!(pkg_segment("2-My.App"), "my_app");
    }

    #[test]
    fn validate_identifier_accepts_good_and_rejects_bad() {
        assert!(validate_identifier("gpui_learn.a11y_09").is_ok());
        assert!(validate_identifier("dev.gpui.mobile").is_ok());
        // 段以下划线开头 → 非法
        assert!(validate_identifier("gpui_learn._09_a11y").is_err());
        // 段以数字开头 → 非法
        assert!(validate_identifier("gpui_learn.09a").is_err());
        // 含非法字符 → 非法
        assert!(validate_identifier("gpui-lean.a11y_09").is_err());
        // 空段 / 空串 → 非法
        assert!(validate_identifier("").is_err());
        assert!(validate_identifier("gpui_learn..a11y").is_err());
    }

    // ---- --bundle-meta 身份表解析 ----

    /// 建一个本测试独有的临时目录（pid + tag 防相撞）。
    fn identity_test_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "aa-gpui-cli-identity-{}-{}",
            std::process::id(),
            tag
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("建临时目录失败");
        dir
    }

    #[test]
    fn identity_reads_json_root() {
        let dir = identity_test_dir("json_root");
        std::fs::write(
            dir.join("id.json"),
            r#"{"identifier": "aa.hello_android", "name": "Hello Android"}"#,
        )
        .unwrap();
        // 文件根即身份表
        let table = load_identity_table("id.json", &dir).unwrap();
        assert_eq!(table.identifier.as_deref(), Some("aa.hello_android"));
        assert_eq!(table.name.as_deref(), Some("Hello Android"));
    }

    #[test]
    fn identity_fields_are_optional() {
        let dir = identity_test_dir("optional");
        std::fs::write(dir.join("id.json"), r#"{"name": "Only Name"}"#).unwrap();
        let table = load_identity_table("id.json", &dir).unwrap();
        assert_eq!(table.identifier, None, "缺 identifier → None，由调用方推导");
        assert_eq!(table.name.as_deref(), Some("Only Name"));
    }

    #[test]
    fn identity_reads_toml_deep_path() {
        let dir = identity_test_dir("toml_deep");
        std::fs::write(
            dir.join("Cargo.toml"),
            r#"
[package]
name = "ug_01_hello_android"

[package.metadata.bundle]
identifier = "aa.hello_android"
name = "Hello Android"
icon = ["app-icon.png"]
"#,
        )
        .unwrap();
        // 数组路径下钻 + key 两侧空白容忍；表内额外 key（icon）忽略
        let table = load_identity_table("Cargo.toml#[package, metadata, bundle]", &dir).unwrap();
        assert_eq!(table.identifier.as_deref(), Some("aa.hello_android"));
        assert_eq!(table.name.as_deref(), Some("Hello Android"));
    }

    #[test]
    fn identity_array_path_supports_dotted_keys() {
        // JSON key 合法含点，且 identifier 本身含点——所以路径必须是数组形式
        let dir = identity_test_dir("dotted_key");
        std::fs::write(
            dir.join("id.json"),
            r#"{"a.b": {"identifier": "dev.aa.AaCode", "name": "AaCode"}}"#,
        )
        .unwrap();
        let table = load_identity_table("id.json#[a.b]", &dir).unwrap();
        assert_eq!(table.identifier.as_deref(), Some("dev.aa.AaCode"));
        assert_eq!(table.name.as_deref(), Some("AaCode"));
    }

    #[test]
    fn identity_errors_are_hard_not_silent() {
        let dir = identity_test_dir("errors");
        std::fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"x\"\n\n[package.metadata.bundle]\nidentifier = 42\nname = \"Ok\"\n",
        )
        .unwrap();
        std::fs::write(dir.join("arr.json"), r#"["not", "an", "object"]"#).unwrap();

        // 文件缺失
        let err = load_identity_table("nope.json", &dir)
            .unwrap_err()
            .to_string();
        assert!(err.contains("nope.json"), "应指出文件：{err}");

        // 路径不存在
        let err = load_identity_table("Cargo.toml#[package, metadata, missing]", &dir)
            .unwrap_err()
            .to_string();
        assert!(err.contains("missing"), "应指出缺的 key：{err}");

        // 字段类型不对（identifier 是数字）
        let err = load_identity_table("Cargo.toml#[package, metadata, bundle]", &dir)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("identifier") && err.contains("字符串"),
            "应指出字段与期望类型：{err}"
        );

        // 下钻中途不是表
        let err = load_identity_table("arr.json#[0]", &dir)
            .unwrap_err()
            .to_string();
        assert!(err.contains("不是表/对象"), "应指出层级类型：{err}");

        // 目标本身不是表
        let err = load_identity_table("arr.json", &dir).unwrap_err().to_string();
        assert!(err.contains("目标不是表/对象"), "应指出目标类型：{err}");

        // 不支持的扩展名
        std::fs::write(dir.join("id.yaml"), "identifier: x\n").unwrap();
        let err = load_identity_table("id.yaml", &dir).unwrap_err().to_string();
        assert!(err.contains("yaml"), "应指出扩展名：{err}");

        // 结构路径含空 key
        let err = load_identity_table("Cargo.toml#[package, , name]", &dir)
            .unwrap_err()
            .to_string();
        assert!(err.contains("空 key"), "应指出空 key：{err}");
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
        std::fs::create_dir_all(parent)
            .with_context(|| format!("创建目录 {}", parent.display()))?;
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
