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
    /// 为指定例子生成 gen/android/ Gradle 工程
    Init {
        /// 例子目录（含 Cargo.toml 与 gpui.conf.json）
        #[arg(long, default_value = ".")]
        example: PathBuf,
        /// 覆盖默认目标 ABI，逗号分隔，如 `--targets arm64-v8a,x86_64`
        #[arg(long, value_delimiter = ',')]
        targets: Option<Vec<String>>,
    },
}

/// gpui.conf.json 的内容（只需两个字段）
#[derive(Deserialize)]
struct GpuiConf {
    identifier: String,
    app_name: String,
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
            AndroidCmd::Init { example, targets } => android_init(&example, targets),
        },
    }
}

fn android_init(example: &Path, targets: Option<Vec<String>>) -> Result<()> {
    let example = if example.is_absolute() {
        example.to_path_buf()
    } else {
        std::env::current_dir()?.join(example)
    };
    anyhow::ensure!(
        example.join("Cargo.toml").exists(),
        "找不到 {}/Cargo.toml，请确认 --example 指向一个例子目录",
        example.display()
    );

    // 1) 读 Cargo.toml：cargo_package = [package] name；rust_lib_name = [lib] name（缺省用 package name）
    let cargo_text = std::fs::read_to_string(example.join("Cargo.toml"))
        .with_context(|| "读取 Cargo.toml 失败")?;
    let cargo: CargoToml = toml::from_str(&cargo_text).context("解析 Cargo.toml 失败")?;
    let cargo_package = &cargo.package.name;
    let rust_lib_name = cargo
        .lib
        .as_ref()
        .and_then(|l| l.name.clone())
        .unwrap_or_else(|| cargo_package.clone());

    // 2) 读 gpui.conf.json：identifier / app_name
    let conf_path = example.join("gpui.conf.json");
    anyhow::ensure!(
        conf_path.exists(),
        "找不到 {}/gpui.conf.json。请创建，至少含 {{\"identifier\": \"...\", \"app_name\": \"...\"}}",
        example.display()
    );
    let conf_text =
        std::fs::read_to_string(&conf_path).with_context(|| "读取 gpui.conf.json 失败")?;
    let conf: GpuiConf = serde_json::from_str(&conf_text).context("解析 gpui.conf.json 失败")?;

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
    let root_project_name = format!("GPUILearn{}", sanitize_ident(&conf.app_name));

    // 4) 生成 gen/android/
    let out = example.join("gen/android");
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
            .replace("{TARGETS}", &targets_literal),
    )?;
    write_file(&out.join("app/src/main/AndroidManifest.xml"), TPL_MANIFEST)?;
    write_file(&out.join("app/src/main/res/values/styles.xml"), TPL_STYLES)?;
    // gradle.properties：appId / appName + 通用 Android 配置
    let props = format!(
        "appId={}\nappName={}\nandroid.useAndroidX=true\nandroid.nonTransitiveRClass=true\norg.gradle.jvmargs=-Xmx2048m -Dfile.encoding=UTF-8\n",
        conf.identifier, conf.app_name
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

    // 复制例子自带的 assets/（如 emoji 字体）到生成的 app/src/main/assets/。
    // 例子目录里放 assets/fonts/NotoColorEmoji.ttf 即可，无需手动拷进 gen/。
    let src_assets = example.join("assets");
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
