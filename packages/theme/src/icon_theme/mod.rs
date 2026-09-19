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
//! **当前只到数据层**:还没有文件树组件(也就没有消费方),因此这里只做
//! 结构与查找逻辑,不接渲染。默认主题也**不含**文件类型映射表
//! (zed 有 400 余行 `FILE_SUFFIXES_BY_ICON_KEY`),等真做文件树时按需补。
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

/// 默认图标主题名。
pub const DEFAULT_ICON_THEME_NAME: &str = "ui-gpui (Default)";

/// 默认图标主题:只给出**目录与箭头**图标,不含文件类型映射。
///
/// zed 的默认主题带 400 余行「扩展名 → 图标」表;我们没有那批 SVG 资源,
/// 补一张查不到实物的表没有意义——等做文件树、真的接入图标资产时再补。
static DEFAULT_ICON_THEME: LazyLock<Arc<IconTheme>> = LazyLock::new(|| {
    Arc::new(IconTheme {
        id: "ui-gpui-default".into(),
        name: DEFAULT_ICON_THEME_NAME.into(),
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
        file_stems: HashMap::new(),
        file_suffixes: HashMap::new(),
        file_icons: HashMap::new(),
    })
});

/// 取默认图标主题。
pub fn default_icon_theme() -> Arc<IconTheme> {
    DEFAULT_ICON_THEME.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(
            theme.icon_for_file("main.rs").is_none(),
            "默认主题不含文件类型映射(无图标资产),应返回 None"
        );
    }
}
