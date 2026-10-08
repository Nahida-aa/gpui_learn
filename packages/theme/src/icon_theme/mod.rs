//! 图标主题:文件树里的图标映射,对齐 zed `crates/theme/src/icon_theme.rs`。
//!
//! 解决的问题:给定一个文件名/目录名,查出该用哪个图标资源。查找顺序
//! (对齐 zed 与 VS Code 图标主题的通行约定,从具体到宽泛):
//!
//! 1. **完整文件名**(`file_stems`):`Dockerfile`、`Cargo.lock` 这类
//!    没有扩展名或扩展名不足以区分的文件;
//! 2. **扩展名**(`file_suffixes`):`rs` → `rust`;
//! 3. 目录则先查 `named_directory_icons`(按目录名),再回退
//!    `directory_icons`(通用文件夹图标)。
//!
//! 查到的都是**图标 key**(如 `rust`),再用 [`IconTheme::file_icons`]
//! 取到真实资源路径(`icons/rust.svg`)——这层间接让多扩展名复用同一图标。
//!
//! **当前只到数据层**:gpui_learn 自己没有文件树组件,消费方在 AAgent
//! (`file_icons` 包),这里只做结构与查找逻辑,不接渲染。
//! 默认主题**包含** zed 原样的文件类型映射表(见 [`default_icon_theme`])。
//!
//! 与 zed 的差异:zed 用 `collections::HashMap`(它的 hashbrown 包装,
//! 便于以后换哈希算法),我们直接用 `std::collections::HashMap`。

pub mod schema;

use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

use gpui::SharedString;

use crate::Appearance;
use crate::icon_theme::schema::{IconThemeContent, IconThemeFamilyContent};

/// 一个图标主题家族(同一套图标的明暗成对)。
#[derive(Debug, Clone)]
pub struct IconThemeFamily {
    /// 家族 id。
    pub id: String,
    /// 家族名。
    pub name: SharedString,
    /// 作者。
    pub author: SharedString,
    /// 家族内的图标主题(通常 light / dark 各一)。
    pub themes: Vec<IconTheme>,
}

/// 一个图标主题(对齐 zed `IconTheme`)。
#[derive(Debug, Clone, PartialEq)]
pub struct IconTheme {
    /// 唯一标识。
    pub id: String,
    /// 展示名。
    pub name: SharedString,
    /// 明暗形态。
    pub appearance: Appearance,
    /// 通用目录图标。
    pub directory_icons: DirectoryIcons,
    /// 按目录名指定的图标。
    pub named_directory_icons: HashMap<String, DirectoryIcons>,
    /// 折叠箭头图标。
    pub chevron_icons: ChevronIcons,
    /// 完整文件名 → 图标 key。
    pub file_stems: HashMap<String, String>,
    /// 扩展名 → 图标 key。
    pub file_suffixes: HashMap<String, String>,
    /// 图标 key → 图标定义。
    pub file_icons: HashMap<String, IconDefinition>,
}

impl Default for IconTheme {
    fn default() -> Self {
        default_icon_theme().as_ref().clone()
    }
}

/// 目录图标(展开/折叠两态)。
#[derive(Debug, Clone, PartialEq)]
pub struct DirectoryIcons {
    pub collapsed: Option<SharedString>,
    pub expanded: Option<SharedString>,
}

/// 折叠箭头图标(展开/折叠两态)。
#[derive(Debug, Clone, PartialEq)]
pub struct ChevronIcons {
    pub collapsed: Option<SharedString>,
    pub expanded: Option<SharedString>,
}

/// 一个图标定义(目前只有资源路径)。
#[derive(Debug, Clone, PartialEq)]
pub struct IconDefinition {
    pub path: SharedString,
}

impl IconTheme {
    /// 文件的图标资源路径。查找顺序:**完整文件名 → 扩展名**。
    ///
    /// 两者都没命中时返回 `None`——调用方回退到通用文件图标
    /// (那个图标由文件树组件决定,不属于主题,故不在这里兜底)。
    pub fn icon_for_file(&self, file_name: &str) -> Option<&SharedString> {
        // 1) 完整文件名优先:`Cargo.lock` 比后缀 `lock` 更精确
        if let Some(key) = self.file_stems.get(file_name)
            && let Some(icon) = self.file_icons.get(key)
        {
            return Some(&icon.path);
        }
        // 2) 扩展名。`rsplit_once` 而非 `split` 的最后一个:取最后一段,
        //    这样 `foo.tar.gz` 查的是 `gz`(与 VS Code 图标主题一致)
        let (_, suffix) = file_name.rsplit_once('.')?;
        let key = self.file_suffixes.get(suffix)?;
        self.file_icons.get(key).map(|icon| &icon.path)
    }

    /// 目录的图标资源路径。先按目录名查,再回退通用文件夹图标。
    ///
    /// `expanded` 选择展开态还是折叠态图标。
    pub fn icon_for_directory(&self, directory_name: &str, expanded: bool) -> Option<&SharedString> {
        let icons = self
            .named_directory_icons
            .get(directory_name)
            .unwrap_or(&self.directory_icons);
        if expanded {
            icons.expanded.as_ref()
        } else {
            icons.collapsed.as_ref()
        }
    }

    /// 折叠箭头的图标资源路径。
    pub fn chevron_icon(&self, expanded: bool) -> Option<&SharedString> {
        if expanded {
            self.chevron_icons.expanded.as_ref()
        } else {
            self.chevron_icons.collapsed.as_ref()
        }
    }

    /// 由图标主题 JSON 的内容构建(对齐 zed 的同名转换)。
    pub fn from_content(content: IconThemeContent) -> Self {
        let appearance = match content.appearance {
            crate::AppearanceContent::Light => Appearance::Light,
            crate::AppearanceContent::Dark => Appearance::Dark,
        };
        Self {
            // 家族内主题名唯一,直接作 id(与配色主题同样的约定)
            id: content.name.clone(),
            name: content.name.into(),
            appearance,
            directory_icons: DirectoryIcons {
                collapsed: content.directory_icons.collapsed,
                expanded: content.directory_icons.expanded,
            },
            named_directory_icons: content
                .named_directory_icons
                .into_iter()
                .map(|(name, icons)| {
                    (
                        name,
                        DirectoryIcons {
                            collapsed: icons.collapsed,
                            expanded: icons.expanded,
                        },
                    )
                })
                .collect(),
            chevron_icons: ChevronIcons {
                collapsed: content.chevron_icons.collapsed,
                expanded: content.chevron_icons.expanded,
            },
            file_stems: content.file_stems,
            file_suffixes: content.file_suffixes,
            file_icons: content
                .file_icons
                .into_iter()
                .map(|(key, definition)| {
                    (
                        key,
                        IconDefinition {
                            path: definition.path,
                        },
                    )
                })
                .collect(),
        }
    }
}

/// 解析一个图标主题家族文件。
pub fn parse_icon_theme_family(
    bytes: &[u8],
) -> serde_json::Result<IconThemeFamily> {
    let content: IconThemeFamilyContent = serde_json::from_slice(bytes)?;
    Ok(IconThemeFamily {
        id: content.name.clone(),
        name: content.name.into(),
        author: content.author.into(),
        themes: content
            .themes
            .into_iter()
            .map(IconTheme::from_content)
            .collect(),
    })
}


const FILE_STEMS_BY_ICON_KEY: &[(&str, &[&str])] = &[
    ("docker", &["Containerfile", "Dockerfile", ".dockerignore"]),
    ("ruby", &["Podfile"]),
    ("heroku", &["Procfile"]),
];

const FILE_SUFFIXES_BY_ICON_KEY: &[(&str, &[&str])] = &[
    ("astro", &["astro"]),
    (
        "audio",
        &[
            "aac", "flac", "m4a", "mka", "mp3", "ogg", "opus", "wav", "wma", "wv",
        ],
    ),
    ("backup", &["bak"]),
    ("ballerina", &["bal"]),
    ("bicep", &["bicep"]),
    ("bun", &["lockb"]),
    ("c", &["c", "h"]),
    ("cairo", &["cairo"]),
    ("code", &["handlebars", "metadata", "rkt", "scm"]),
    ("coffeescript", &["coffee"]),
    (
        "cpp",
        &[
            "c++", "h++", "cc", "cpp", "cppm", "cxx", "hh", "hpp", "hxx", "inl", "ixx",
        ],
    ),
    ("crystal", &["cr", "ecr"]),
    ("csharp", &["cs"]),
    ("csproj", &["csproj"]),
    ("css", &["css", "pcss", "postcss"]),
    ("cue", &["cue"]),
    ("dart", &["dart"]),
    ("diff", &["diff"]),
    (
        "docker",
        &[
            "docker-compose.yml",
            "docker-compose.yaml",
            "compose.yml",
            "compose.yaml",
        ],
    ),
    (
        "document",
        &[
            "doc", "docx", "mdx", "odp", "ods", "odt", "pdf", "ppt", "pptx", "rtf", "txt", "xls",
            "xlsx",
        ],
    ),
    ("editorconfig", &["editorconfig"]),
    ("elixir", &["eex", "ex", "exs", "heex", "leex", "neex"]),
    ("elm", &["elm"]),
    (
        "erlang",
        &[
            "Emakefile",
            "app.src",
            "erl",
            "escript",
            "hrl",
            "rebar.config",
            "xrl",
            "yrl",
        ],
    ),
    (
        "eslint",
        &[
            "eslint.config.cjs",
            "eslint.config.cts",
            "eslint.config.js",
            "eslint.config.mjs",
            "eslint.config.mts",
            "eslint.config.ts",
            "eslintrc",
            "eslintrc.js",
            "eslintrc.json",
        ],
    ),
    ("font", &["otf", "ttf", "woff", "woff2"]),
    ("fsharp", &["fs"]),
    ("fsproj", &["fsproj"]),
    ("gitlab", &["gitlab-ci.yml", "gitlab-ci.yaml"]),
    ("gleam", &["gleam"]),
    ("go", &["go", "mod", "work"]),
    ("graphql", &["gql", "graphql", "graphqls"]),
    ("haskell", &["hs"]),
    ("hcl", &["hcl"]),
    (
        "helm",
        &[
            "helmfile.yaml",
            "helmfile.yml",
            "Chart.yaml",
            "Chart.yml",
            "Chart.lock",
            "values.yaml",
            "values.yml",
            "requirements.yaml",
            "requirements.yml",
            "tpl",
        ],
    ),
    ("html", &["htm", "html"]),
    (
        "image",
        &[
            "avif", "bmp", "gif", "heic", "heif", "ico", "j2k", "jfif", "jp2", "jpeg", "jpg",
            "jxl", "png", "psd", "qoi", "svg", "tiff", "webp",
        ],
    ),
    ("ipynb", &["ipynb"]),
    ("java", &["java"]),
    ("javascript", &["cjs", "js", "mjs"]),
    ("json", &["json", "jsonc"]),
    ("julia", &["jl"]),
    ("kdl", &["kdl"]),
    ("kotlin", &["kt"]),
    ("lock", &["lock"]),
    ("log", &["log"]),
    ("lua", &["lua"]),
    ("luau", &["luau"]),
    ("markdown", &["markdown", "md"]),
    ("metal", &["metal"]),
    ("nim", &["nim", "nims", "nimble"]),
    ("nix", &["nix"]),
    ("ocaml", &["ml", "mli", "mlx"]),
    ("odin", &["odin"]),
    ("php", &["php"]),
    (
        "prettier",
        &[
            "prettier.config.cjs",
            "prettier.config.js",
            "prettier.config.mjs",
            "prettierignore",
            "prettierrc",
            "prettierrc.cjs",
            "prettierrc.js",
            "prettierrc.json",
            "prettierrc.json5",
            "prettierrc.mjs",
            "prettierrc.toml",
            "prettierrc.yaml",
            "prettierrc.yml",
        ],
    ),
    ("prisma", &["prisma"]),
    ("puppet", &["pp"]),
    ("python", &["py"]),
    ("r", &["r", "R"]),
    ("react", &["cjsx", "ctsx", "jsx", "mjsx", "mtsx", "tsx"]),
    ("roc", &["roc"]),
    ("ruby", &["rb"]),
    ("rust", &["rs"]),
    ("sass", &["sass", "scss"]),
    ("scala", &["scala", "sc"]),
    ("settings", &["conf", "ini"]),
    ("solidity", &["sol"]),
    (
        "storage",
        &[
            "accdb", "csv", "dat", "db", "dbf", "dll", "fmp", "fp7", "frm", "gdb", "ib", "ldf",
            "mdb", "mdf", "myd", "myi", "pdb", "psv", "RData", "rdata", "sav", "sdf", "sql",
            "sqlite", "ssv", "tsv",
        ],
    ),
    (
        "stylelint",
        &[
            "stylelint.config.cjs",
            "stylelint.config.js",
            "stylelint.config.mjs",
            "stylelintignore",
            "stylelintrc",
            "stylelintrc.cjs",
            "stylelintrc.js",
            "stylelintrc.json",
            "stylelintrc.mjs",
            "stylelintrc.yaml",
            "stylelintrc.yml",
        ],
    ),
    ("surrealql", &["surql"]),
    ("svelte", &["svelte"]),
    ("swift", &["swift"]),
    ("tcl", &["tcl"]),
    ("template", &["hbs", "plist", "xml"]),
    (
        "terminal",
        &[
            "bash",
            "bash_aliases",
            "bash_login",
            "bash_logout",
            "bash_profile",
            "bashrc",
            "brushrc",
            "fish",
            "nu",
            "profile",
            "ps1",
            "sh",
            "zlogin",
            "zlogout",
            "zprofile",
            "zsh",
            "zsh_aliases",
            "zsh_histfile",
            "zsh_history",
            "zshenv",
            "zshrc",
        ],
    ),
    ("terraform", &["tf", "tfvars"]),
    ("toml", &["toml"]),
    ("typescript", &["cts", "mts", "ts"]),
    ("v", &["v", "vsh", "vv"]),
    (
        "vcs",
        &[
            "COMMIT_EDITMSG",
            "EDIT_DESCRIPTION",
            "MERGE_MSG",
            "NOTES_EDITMSG",
            "TAG_EDITMSG",
            "gitattributes",
            "gitignore",
            "gitkeep",
            "gitmodules",
        ],
    ),
    ("vbproj", &["vbproj"]),
    ("video", &["avi", "m4v", "mkv", "mov", "mp4", "webm", "wmv"]),
    ("vs_sln", &["sln"]),
    ("vs_suo", &["suo"]),
    ("vue", &["vue"]),
    ("vyper", &["vy", "vyi"]),
    ("wgsl", &["wgsl"]),
    ("yaml", &["yaml", "yml"]),
    ("zig", &["zig"]),
];

/// A mapping of a file type identifier to its corresponding icon.
const FILE_ICONS: &[(&str, &str)] = &[
    ("astro", "icons/file_icons/astro.svg"),
    ("audio", "icons/file_icons/audio.svg"),
    ("ballerina", "icons/file_icons/ballerina.svg"),
    ("bicep", "icons/file_icons/file.svg"),
    ("bun", "icons/file_icons/bun.svg"),
    ("c", "icons/file_icons/c.svg"),
    ("cairo", "icons/file_icons/cairo.svg"),
    ("code", "icons/file_icons/code.svg"),
    ("coffeescript", "icons/file_icons/coffeescript.svg"),
    ("cpp", "icons/file_icons/cpp.svg"),
    ("crystal", "icons/file_icons/file.svg"),
    ("csharp", "icons/file_icons/file.svg"),
    ("csproj", "icons/file_icons/file.svg"),
    ("css", "icons/file_icons/css.svg"),
    ("cue", "icons/file_icons/file.svg"),
    ("dart", "icons/file_icons/dart.svg"),
    ("default", "icons/file_icons/file.svg"),
    ("diff", "icons/file_icons/diff.svg"),
    ("docker", "icons/file_icons/docker.svg"),
    ("document", "icons/file_icons/book.svg"),
    ("editorconfig", "icons/file_icons/editorconfig.svg"),
    ("elixir", "icons/file_icons/elixir.svg"),
    ("elm", "icons/file_icons/elm.svg"),
    ("erlang", "icons/file_icons/erlang.svg"),
    ("eslint", "icons/file_icons/eslint.svg"),
    ("font", "icons/file_icons/font.svg"),
    ("fsharp", "icons/file_icons/fsharp.svg"),
    ("fsproj", "icons/file_icons/file.svg"),
    ("gitlab", "icons/file_icons/gitlab.svg"),
    ("gleam", "icons/file_icons/gleam.svg"),
    ("go", "icons/file_icons/go.svg"),
    ("graphql", "icons/file_icons/graphql.svg"),
    ("haskell", "icons/file_icons/haskell.svg"),
    ("hcl", "icons/file_icons/hcl.svg"),
    ("helm", "icons/file_icons/helm.svg"),
    ("heroku", "icons/file_icons/heroku.svg"),
    ("html", "icons/file_icons/html.svg"),
    ("image", "icons/file_icons/image.svg"),
    ("ipynb", "icons/file_icons/jupyter.svg"),
    ("java", "icons/file_icons/java.svg"),
    ("javascript", "icons/file_icons/javascript.svg"),
    ("json", "icons/file_icons/code.svg"),
    ("julia", "icons/file_icons/julia.svg"),
    ("kdl", "icons/file_icons/kdl.svg"),
    ("kotlin", "icons/file_icons/kotlin.svg"),
    ("lock", "icons/file_icons/lock.svg"),
    ("log", "icons/file_icons/info.svg"),
    ("lua", "icons/file_icons/lua.svg"),
    ("luau", "icons/file_icons/luau.svg"),
    ("markdown", "icons/file_icons/book.svg"),
    ("metal", "icons/file_icons/metal.svg"),
    ("nim", "icons/file_icons/nim.svg"),
    ("nix", "icons/file_icons/nix.svg"),
    ("ocaml", "icons/file_icons/ocaml.svg"),
    ("odin", "icons/file_icons/odin.svg"),
    ("phoenix", "icons/file_icons/phoenix.svg"),
    ("php", "icons/file_icons/php.svg"),
    ("prettier", "icons/file_icons/prettier.svg"),
    ("prisma", "icons/file_icons/prisma.svg"),
    ("puppet", "icons/file_icons/puppet.svg"),
    ("python", "icons/file_icons/python.svg"),
    ("r", "icons/file_icons/r.svg"),
    ("react", "icons/file_icons/react.svg"),
    ("roc", "icons/file_icons/roc.svg"),
    ("ruby", "icons/file_icons/ruby.svg"),
    ("rust", "icons/file_icons/rust.svg"),
    ("sass", "icons/file_icons/sass.svg"),
    ("scala", "icons/file_icons/scala.svg"),
    ("settings", "icons/file_icons/settings.svg"),
    ("solidity", "icons/file_icons/file.svg"),
    ("storage", "icons/file_icons/database.svg"),
    ("stylelint", "icons/file_icons/javascript.svg"),
    ("surrealql", "icons/file_icons/surrealql.svg"),
    ("svelte", "icons/file_icons/html.svg"),
    ("swift", "icons/file_icons/swift.svg"),
    ("tcl", "icons/file_icons/tcl.svg"),
    ("template", "icons/file_icons/html.svg"),
    ("terminal", "icons/file_icons/terminal.svg"),
    ("terraform", "icons/file_icons/terraform.svg"),
    ("toml", "icons/file_icons/toml.svg"),
    ("typescript", "icons/file_icons/typescript.svg"),
    ("v", "icons/file_icons/v.svg"),
    ("vbproj", "icons/file_icons/file.svg"),
    ("vcs", "icons/file_icons/git.svg"),
    ("video", "icons/file_icons/video.svg"),
    ("vs_sln", "icons/file_icons/file.svg"),
    ("vs_suo", "icons/file_icons/file.svg"),
    ("vue", "icons/file_icons/vue.svg"),
    ("vyper", "icons/file_icons/vyper.svg"),
    ("wgsl", "icons/file_icons/wgsl.svg"),
    ("yaml", "icons/file_icons/yaml.svg"),
    ("zig", "icons/file_icons/zig.svg"),
];

/// Returns a mapping of file associations to icon keys.
fn icon_keys_by_association(
    associations_by_icon_key: &[(&str, &[&str])],
) -> HashMap<String, String> {
    let mut icon_keys_by_association = HashMap::default();
    for (icon_key, associations) in associations_by_icon_key {
        for association in *associations {
            icon_keys_by_association.insert(association.to_string(), icon_key.to_string());
        }
    }

    icon_keys_by_association
}

/// 用户默认图标主题名(暗色):内置 Catppuccin 图标主题。
///
/// 浅色系统下用 [`DEFAULT_LIGHT_ICON_THEME_NAME`];两者都不在注册表里时,
/// 由 [`IconTheme::default`] 退回 [`ZED_DEFAULT_ICON_THEME_NAME`]。
///
/// 这与 zed 的 `theme::DEFAULT_ICON_THEME_NAME` **刻意不同**:zed 那份指向
/// 自带的单色图标集(见 [`ZED_DEFAULT_ICON_THEME_NAME`]),而这里的用户默认是
/// CTP 多彩图标。注册表的 `default_icon_theme()` 仍返回 zed 那份(对齐 zed),
/// 它同时是所有图标主题查不到类型时的最后兜底 —— CTP 表里没有 `default` 键,
/// `LICENSE-APACHE`、`foo.xyz` 这类要靠它才有图标。
pub const DEFAULT_ICON_THEME_NAME: &str = "Catppuccin Macchiato";

/// 浅色系统下的用户默认图标主题名。
pub const DEFAULT_LIGHT_ICON_THEME_NAME: &str = "Catppuccin Latte";

/// zed 自带的单色图标主题名(名字与 zed 一致,便于图标主题选择器里对得上)。
pub const ZED_DEFAULT_ICON_THEME_NAME: &str = "Zed (Default)";

/// 内置 Catppuccin 图标主题家族的 JSON
/// (`assets/icon_themes/catppuccin-icons.json`,来自 catppuccin/zed-icons)。
///
/// 走 `include_str!` 而不是资产加载:[`crate::ThemeRegistry::new`] 时就要注册
/// 完,而 `LoadThemes::JustBase` 下注册表拿到的资产源是空的,读不到任何资产。
/// JSON 只在这里用,故没有加进 assets crate 的嵌入列表(避免二进制里两份)。
const CATPPUCCIN_ICONS_JSON: &str =
    include_str!("../../../../assets/icon_themes/catppuccin-icons.json");

/// 内置 Catppuccin 图标主题家族:Macchiato(暗)+ Latte(浅)两个 variant,
/// 每个 1810 条后缀、400 个图标、10 条完整文件名。
///
/// 上游(catppuccin/zed-icons v1.24.0)还有 Frappé / Mocha 及各自的单色版,
/// 为控制仓库体积只留了默认会用到的这两套;要补的话把 `icons/<variant>/`
/// 复制进来、并在 JSON 里加回对应 theme 即可。
pub fn catppuccin_icon_theme_family() -> IconThemeFamilyContent {
    serde_json::from_str(CATPPUCCIN_ICONS_JSON).expect("内置 CTP 图标主题 JSON 应能解析")
}

/// zed 自带的单色图标主题:目录/箭头图标 + zed 原样的文件类型映射表。
///
/// 表(`FILE_STEMS_BY_ICON_KEY` / `FILE_SUFFIXES_BY_ICON_KEY` / `FILE_ICONS`)
/// 与 zed 逐行一致,引用的 92 个 SVG 都在 `assets/icons/file_icons/` 下。
/// 三处角色:
/// 1. [`crate::ThemeRegistry::default_icon_theme`] 返回它(对齐 zed)——
///    `file_icons::FileIcons` 查不到类型时的最后兜底;
/// 2. [`crate::ThemeRegistry::load_icon_theme`] 拿它当合并基底;
/// 3. [`IconTheme::default`] 退回它。
static DEFAULT_ICON_THEME: LazyLock<Arc<IconTheme>> = LazyLock::new(|| {
    Arc::new(IconTheme {
        id: "zed-default".into(),
        name: ZED_DEFAULT_ICON_THEME_NAME.into(),
        appearance: Appearance::Dark,
        directory_icons: DirectoryIcons {
            collapsed: Some("icons/file_icons/folder.svg".into()),
            expanded: Some("icons/file_icons/folder_open.svg".into()),
        },
        named_directory_icons: HashMap::new(),
        chevron_icons: ChevronIcons {
            collapsed: Some("icons/file_icons/chevron_right.svg".into()),
            expanded: Some("icons/file_icons/chevron_down.svg".into()),
        },
        file_stems: icon_keys_by_association(FILE_STEMS_BY_ICON_KEY),
        file_suffixes: icon_keys_by_association(FILE_SUFFIXES_BY_ICON_KEY),
        file_icons: HashMap::from_iter(FILE_ICONS.iter().map(|(ty, path)| {
            (
                ty.to_string(),
                IconDefinition {
                    path: (*path).into(),
                },
            )
        })),
    })
});

/// 取 zed 自带的单色图标主题(对齐 zed 的 `theme::default_icon_theme`)。
pub fn default_icon_theme() -> Arc<IconTheme> {
    DEFAULT_ICON_THEME.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::ThemeRegistry;

    fn theme_with_mappings() -> IconTheme {
        let content: IconThemeContent = serde_json::from_str(
            r##"{
                "name": "Test Icons",
                "appearance": "dark",
                "directory_icons": {
                    "collapsed": "folder.svg",
                    "expanded": "folder_open.svg"
                },
                "named_directory_icons": {
                    "src": { "collapsed": "src.svg", "expanded": "src_open.svg" }
                },
                "chevron_icons": { "collapsed": "right.svg", "expanded": "down.svg" },
                "file_stems": { "Dockerfile": "docker" },
                "file_suffixes": { "rs": "rust", "gz": "archive" },
                "file_icons": {
                    "docker": { "path": "docker.svg" },
                    "rust": { "path": "rust.svg" },
                    "archive": { "path": "archive.svg" }
                }
            }"##,
        )
        .expect("parse icon theme content");
        IconTheme::from_content(content)
    }

    #[test]
    fn resolves_stem_before_suffix() {
        let theme = theme_with_mappings();
        assert_eq!(
            theme.icon_for_file("Dockerfile").map(|s| s.to_string()),
            Some("docker.svg".to_string()),
            "完整文件名优先于扩展名"
        );
        assert_eq!(
            theme.icon_for_file("main.rs").map(|s| s.to_string()),
            Some("rust.svg".to_string())
        );
    }

    #[test]
    fn uses_last_suffix_for_multi_dot_names() {
        let theme = theme_with_mappings();
        assert_eq!(
            theme.icon_for_file("backup.tar.gz").map(|s| s.to_string()),
            Some("archive.svg".to_string()),
            "取最后一段后缀(gz),不是 tar"
        );
    }

    #[test]
    fn returns_none_when_unknown() {
        let theme = theme_with_mappings();
        assert!(
            theme.icon_for_file("mystery.xyz").is_none(),
            "未收录的扩展名应返回 None,由调用方回退通用图标"
        );
        assert!(
            theme.icon_for_file("no_extension").is_none(),
            "没有扩展名的文件查不到 stem 时也应返回 None"
        );
    }

    #[test]
    fn directory_icons_fall_back_to_generic() {
        let theme = theme_with_mappings();
        assert_eq!(
            theme
                .icon_for_directory("src", false)
                .map(|s| s.to_string()),
            Some("src.svg".to_string()),
            "有名目录用自己的图标"
        );
        assert_eq!(
            theme
                .icon_for_directory("random", true)
                .map(|s| s.to_string()),
            Some("folder_open.svg".to_string()),
            "未收录的目录回退通用文件夹(展开态)"
        );
    }

    #[test]
    fn parses_family_into_themes() {
        let json = br##"{
            "name": "Family",
            "author": "someone",
            "themes": [
                { "name": "Family Dark", "appearance": "dark" },
                { "name": "Family Light", "appearance": "light" }
            ]
        }"##;
        let family = parse_icon_theme_family(json).expect("parse family");
        assert_eq!(family.name, "Family");
        assert_eq!(family.author, "someone");
        assert_eq!(family.themes.len(), 2);
        assert_eq!(family.themes[0].appearance, Appearance::Dark);
        assert_eq!(family.themes[1].appearance, Appearance::Light);
    }

    #[test]
    fn default_theme_has_directory_and_chevron_icons() {
        let theme = default_icon_theme();
        assert!(theme.icon_for_directory("src", false).is_some());
        assert!(theme.chevron_icon(true).is_some());
    }

    #[test]
    fn default_theme_resolves_file_types_like_zed() {
        let theme = default_icon_theme();
        assert_eq!(
            theme.icon_for_file("main.rs").map(|s| s.to_string()),
            Some("icons/file_icons/rust.svg".to_string())
        );
        assert_eq!(
            theme.icon_for_file("Cargo.toml").map(|s| s.to_string()),
            Some("icons/file_icons/toml.svg".to_string())
        );
        assert_eq!(
            theme.icon_for_file("Dockerfile").map(|s| s.to_string()),
            Some("icons/file_icons/docker.svg".to_string()),
            "完整文件名走 file_stems"
        );
        assert!(
            theme.icon_for_file("mystery.xyz").is_none(),
            "未收录的扩展名仍应返回 None"
        );
    }

    #[test]
    fn builtin_catppuccin_icon_themes_are_registered() {
        let registry = ThemeRegistry::default();
        for name in [
            DEFAULT_LIGHT_ICON_THEME_NAME,
            DEFAULT_ICON_THEME_NAME,
            ZED_DEFAULT_ICON_THEME_NAME,
        ] {
            assert!(registry.get_icon_theme(name).is_ok(), "应注册 {name}");
        }
    }

    #[test]
    fn registry_default_icon_theme_is_the_zed_one() {
        // 对齐 zed:注册表的 default_icon_theme 是 zed 自带那张单色表,
        // 不是 CTP。CTP 表没有 `default` 键,`FileIcons` 查不到类型时的
        // 兜底全靠它 —— 若这里返回 CTP,`LICENSE-APACHE` 之类就没有图标。
        let registry = ThemeRegistry::default();
        let theme = registry.default_icon_theme().expect("默认图标主题应已注册");
        assert_eq!(theme.name, ZED_DEFAULT_ICON_THEME_NAME);
        assert_eq!(
            theme
                .file_icons
                .get("default")
                .map(|icon| icon.path.to_string()),
            Some("icons/file_icons/file.svg".to_string()),
            "兜底主题必须提供 `default` 类型,否则未知后缀的文件没有图标"
        );
        assert_eq!(
            theme.icon_for_file(".zlogin").map(|s| s.to_string()),
            Some("icons/file_icons/terminal.svg".to_string()),
            "zlogin/zlogout 要映射到 terminal 图标(与 zed 表一致)"
        );
    }

    #[test]
    fn every_builtin_icon_path_exists_in_assets() {
        // 表里每个路径都必须能在 assets 下找到,否则文件树会拿到空白图标
        // (上游还有 Frappé / Mocha 等 variant,补齐时这条测试就是守门的)。
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        for theme in catppuccin_icon_theme_family().themes {
            let paths = theme
                .file_icons
                .values()
                .map(|icon| icon.path.as_ref())
                .chain(theme.directory_icons.collapsed.iter().map(|p| p.as_ref()))
                .chain(theme.directory_icons.expanded.iter().map(|p| p.as_ref()));
            for path in paths {
                let relative = path.trim_start_matches("./");
                assert!(
                    assets.join(relative).exists(),
                    "{} 引用了不存在的图标: {relative}",
                    theme.name
                );
            }
        }
    }

    #[test]
    fn default_icon_theme_resolves_catppuccin_icons() {
        let registry = ThemeRegistry::default();
        let theme = registry
            .get_icon_theme(DEFAULT_ICON_THEME_NAME)
            .expect("默认图标主题应已注册");
        assert_eq!(
            theme.icon_for_file("main.rs").map(|s| s.to_string()),
            Some("icon_themes/catppuccin/macchiato/rust.svg".to_string()),
            "路径应是相对 assets 根,不带 JSON 里的 ./ 前缀"
        );
        assert_eq!(
            theme.icon_for_file("Cargo.toml").map(|s| s.to_string()),
            Some("icon_themes/catppuccin/macchiato/cargo.svg".to_string()),
            "完整文件名走 file_stems(Cargo.toml → cargo)"
        );
        assert_eq!(
            theme.icon_for_file("pyproject.toml").map(|s| s.to_string()),
            Some("icon_themes/catppuccin/macchiato/toml.svg".to_string()),
            "普通 .toml 走 file_suffixes"
        );
        assert_eq!(
            theme.icon_for_directory("anything", false).map(|s| s.to_string()),
            Some("icon_themes/catppuccin/macchiato/_folder.svg".to_string())
        );
        assert!(
            theme.chevron_icon(false).is_some(),
            "CTP 主题没有 chevron,应兜底到默认主题的箭头"
        );
    }

    #[test]
    fn light_default_icon_theme_uses_latte() {
        let registry = ThemeRegistry::default();
        let theme = registry
            .get_icon_theme(DEFAULT_LIGHT_ICON_THEME_NAME)
            .expect("浅色默认图标主题应已注册");
        assert_eq!(
            theme.icon_for_file("main.rs").map(|s| s.to_string()),
            Some("icon_themes/catppuccin/latte/rust.svg".to_string())
        );
    }

    #[test]
    fn every_icon_theme_path_exists_in_assets() {
        // 表里的路径必须能在 assets 下找到,否则文件树会拿到空白图标。
        let missing: Vec<_> = FILE_ICONS
            .iter()
            .map(|(_, path)| *path)
            .filter(|path| {
                !std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../assets")
                    .join(path)
                    .exists()
            })
            .collect();
        assert!(missing.is_empty(), "assets 里缺少这些图标: {missing:?}");
    }
}
