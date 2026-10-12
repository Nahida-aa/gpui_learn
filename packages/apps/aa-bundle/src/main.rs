//! `aa-bundle` 的 CLI 壳 —— 参数解析后全部转调库侧（`aa_bundle::arch` 等）。
//!
//! 设计原则（对齐 cargo-bundle）：**打包元数据声明在 Cargo.toml，不在命令行**。
//! 所以这里的 flag 都是「覆盖项」而非「必填项」—— 正常用法就是
//! `aa-bundle arch -p <crate 目录>`，其余都从 Cargo.toml 推导/读取：
//! - 身份（identifier / name） ← `[package.metadata.bundle-<channel>]`
//! - Arch 设置（依赖 / 模板 / 图标名…） ← `[package.metadata.bundle-pkgbuild]`（可选）

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
    Pkgbuild {
        /// 源码目录（含 Cargo.toml 与 resources/），默认当前目录
        #[arg(short = 'p', long, default_value = ".")]
        project_dir: PathBuf,
        /// 频道：决定 bundle 表（`bundle-<channel>`）与图标后缀。默认 stable
        #[arg(long, default_value = "stable")]
        channel: String,
        /// 身份来源引用，覆盖默认的 `Cargo.toml#[package, metadata, bundle-<channel>]`
        #[arg(long)]
        bundle_meta: Option<String>,
        /// Arch 设置来源引用，覆盖默认的 `Cargo.toml#[package, metadata, bundle-pkgbuild]`
        #[arg(long)]
        pkgbuild_meta: Option<String>,
        /// 覆盖 .desktop 模板路径
        #[arg(long)]
        desktop_template: Option<PathBuf>,
        /// 覆盖二进制名（默认 crate 名）
        #[arg(long)]
        bin_name: Option<String>,
        /// 覆盖图标基础名（默认 crate 名）
        #[arg(long)]
        icon_name: Option<String>,
        /// 覆盖运行依赖（完全替代 Cargo.toml 里的值，可多次传）
        #[arg(long)]
        depends: Vec<String>,
        /// 覆盖构建依赖（可多次传）
        #[arg(long)]
        makedepends: Vec<String>,
        /// 覆盖目标架构（可多次传）
        #[arg(long)]
        arch: Vec<String>,
        /// 覆盖 url
        #[arg(long)]
        url: Option<String>,
        /// 覆盖 license（Cargo.toml 读不到时）
        #[arg(long)]
        license: Option<String>,
        /// 覆盖安装前缀（默认 /usr）
        #[arg(long)]
        prefix: Option<String>,
        /// 输出目录，默认 <project>/target/aa-bundle/arch
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Pkgbuild {
            project_dir,
            channel,
            bundle_meta,
            pkgbuild_meta,
            desktop_template,
            bin_name,
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
            let default_pkgbuild_meta = "Cargo.toml#[package, metadata, bundle-pkgbuild]";
            let pkgbuild_meta = pkgbuild_meta.as_deref().unwrap_or(default_pkgbuild_meta);

            let opts = aa_bundle::pkgbuild::PkgbuildOptions {
                project_dir: &project_dir,
                channel: &channel,
                bundle_meta,
                pkgbuild_meta,
                bin_name: bin_name.as_deref(),
                desktop_template: desktop_template.as_deref(),
                icon_name: icon_name.as_deref(),
                depends: (!depends.is_empty()).then_some(depends.as_slice()),
                makedepends: (!makedepends.is_empty()).then_some(makedepends.as_slice()),
                arch: (!arch.is_empty()).then_some(arch.as_slice()),
                url: url.as_deref(),
                license: license.as_deref(),
                prefix: prefix.as_deref(),
            };
            let (pkgbuild, desktop) = aa_bundle::pkgbuild::generate_pkgbuild(&opts)?;

            let out_dir = out.unwrap_or_else(|| project_dir.join("target/aa-bundle/arch"));
            aa_bundle::write_file(&out_dir.join("PKGBUILD"), &pkgbuild)?;
            println!("已生成 {}", out_dir.join("PKGBUILD").display());
            if let Some((name, rendered)) = desktop {
                aa_bundle::write_file(&out_dir.join(&name), &rendered)?;
                println!("已生成 {}", out_dir.join(name).display());
            }

            println!("接下来：");
            println!("  cd {}", out_dir.display());
            println!("  makepkg -si        # 构建并安装（卸载用 pacman -R <pkgname>）");
            Ok(())
        }
    }
}
