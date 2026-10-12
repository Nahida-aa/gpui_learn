//! `aa-bundle` 的 CLI 壳 —— 参数解析后全部转调库侧（`aa_bundle::arch` 等）。

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "aa-bundle", about = "打包工具：身份解析 / 各平台安装包生成")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 生成 Arch Linux 的 PKGBUILD（本地开发安装用途）
    Arch {
        /// 源码目录（含 Cargo.toml 与 resources/），默认当前目录
        #[arg(short = 'p', long, default_value = ".")]
        project_dir: PathBuf,
        /// 频道：决定 bundle 表（`bundle-<channel>`）与图标后缀。默认 stable
        #[arg(long, default_value = "stable")]
        channel: String,
        /// 身份来源引用，覆盖默认的 `Cargo.toml#[package, metadata, bundle-<channel>]`
        #[arg(long)]
        bundle_meta: Option<String>,
        /// .desktop 模板（内含 $APP_NAME/$APP_ID/$APP_CLI/$APP_ICON/$APP_ARGS 等变量）
        #[arg(long)]
        desktop_template: Option<PathBuf>,
        /// 二进制名，默认取 crate 名
        #[arg(long)]
        bin_name: Option<String>,
        /// 工作区场景下的 `cargo build -p <package>`
        #[arg(long)]
        cargo_package: Option<String>,
        /// 图标基础名（不含后缀与扩展），默认取 crate 名
        #[arg(long)]
        icon_name: Option<String>,
        /// 运行依赖，可多次传
        #[arg(long)]
        depends: Vec<String>,
        /// 构建依赖，可多次传
        #[arg(long)]
        makedepends: Vec<String>,
        /// 目标架构，可多次传。默认 x86_64
        #[arg(long, default_value = "x86_64")]
        arch: Vec<String>,
        #[arg(long)]
        url: Option<String>,
        /// 覆盖 license（Cargo.toml 读不到时）
        #[arg(long)]
        license: Option<String>,
        /// 安装前缀，默认 /usr（pacman 包）
        #[arg(long, default_value = "/usr")]
        prefix: String,
        /// 输出目录，默认 <project>/target/aa-bundle/arch
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Arch {
            project_dir,
            channel,
            bundle_meta,
            desktop_template,
            bin_name,
            cargo_package,
            icon_name,
            depends,
            makedepends,
            arch,
            url,
            license,
            prefix,
            out,
        } => {
            let default_meta = format!("Cargo.toml#[package, metadata, bundle-{channel}]");
            let bundle_meta = bundle_meta.as_deref().unwrap_or(&default_meta);
            let makedepends = if makedepends.is_empty() {
                vec!["cargo".to_string(), "rust".to_string()]
            } else {
                makedepends
            };

            let opts = aa_bundle::arch::ArchOptions {
                project_dir: &project_dir,
                channel: &channel,
                bundle_meta,
                bin_name: bin_name.as_deref(),
                cargo_package: cargo_package.as_deref(),
                desktop_template: desktop_template.as_deref(),
                icon_name: icon_name.as_deref(),
                depends: &depends,
                makedepends: &makedepends,
                arch: &arch,
                url: url.as_deref(),
                license: license.as_deref(),
                prefix: &prefix,
            };
            let (pkgbuild, desktop) = aa_bundle::arch::generate_pkgbuild(&opts)?;

            let out_dir = out.unwrap_or_else(|| project_dir.join("target/aa-bundle/arch"));
            aa_bundle::write_file(&out_dir.join("PKGBUILD"), &pkgbuild)?;
            if let Some((name, rendered)) = desktop {
                aa_bundle::write_file(&out_dir.join(&name), &rendered)?;
                println!("已生成 {}", out_dir.join(name).display());
            }

            println!("已生成 {}", out_dir.join("PKGBUILD").display());
            println!("接下来：");
            println!("  cd {}", out_dir.display());
            println!("  makepkg -si        # 构建并安装（卸载用 pacman -R <pkgname>）");
            Ok(())
        }
    }
}
