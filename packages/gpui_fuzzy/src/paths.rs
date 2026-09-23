//! 文件路径模糊匹配：文件查找器、git 文件选择器等用它。
//!
//! API 与 zed `crates/fuzzy_nucleo/src/paths.rs` 一致：候选路径是
//! [`RelPath`]（不是 `std::path::Path`），[`PathStyle`] 也直接用 `path` crate 的
//! ——这样调用方（file_finder / project）不必在两套路径类型之间转换。
//!
//! 打分在字符串版之上多了三件事：文件名单独匹配加分、长度惩罚、
//! 与「相对当前文件的距离」排序（离得近的优先）。

use gpui::BackgroundExecutor;
use nucleo::Utf32Str;
use nucleo::pattern::Pattern;
use path::{PathStyle, rel_path::RelPath, rel_path::RelPathBuf};
use std::{
    cmp::Ordering,
    sync::{
        Arc,
        atomic::{self, AtomicBool},
    },
};

use crate::matcher::{self, LENGTH_PENALTY};
use crate::{
    Cancelled, Case, CharBag, Query, case_penalty, count_case_mismatches, positions_from_sorted,
};

/// 一个待匹配的路径。
#[derive(Clone, Debug)]
pub struct PathMatchCandidate<'a> {
    pub is_dir: bool,
    pub path: &'a RelPath,
    pub char_bag: CharBag,
}

impl<'a> PathMatchCandidate<'a> {
    /// Build a candidate whose prefilter bag covers both the worktree prefix and the path.
    /// Pass `None` when matching against paths that have no worktree prefix.
    pub fn new(path: &'a RelPath, is_dir: bool, path_prefix: Option<&RelPath>) -> Self {
        let mut char_bag = CharBag::default();
        if let Some(prefix) = path_prefix
            && !prefix.is_empty()
        {
            char_bag.extend(
                prefix
                    .as_unix_str()
                    .chars()
                    .map(|c| c.to_ascii_lowercase()),
            );
        }
        char_bag.extend(
            path.as_unix_str()
                .chars()
                .map(|c| c.to_ascii_lowercase()),
        );
        Self {
            is_dir,
            path,
            char_bag,
        }
    }
}

/// 一个路径匹配结果。`positions` 是**含前缀的** `candidate_buf` 里的字节偏移。
#[derive(Clone, Debug)]
pub struct PathMatch {
    pub score: f64,
    pub positions: Vec<usize>,
    pub worktree_id: usize,
    pub path: Arc<RelPath>,
    pub path_prefix: Arc<RelPath>,
    pub is_dir: bool,
    /// Number of steps removed from a shared parent with the relative path
    /// Used to order closer paths first in the search list
    pub distance_to_relative_ancestor: usize,
}

pub trait PathMatchCandidateSet<'a>: Send + Sync {
    type Candidates: Iterator<Item = PathMatchCandidate<'a>>;
    fn id(&self) -> usize;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn root_is_file(&self) -> bool;
    fn prefix(&self) -> Arc<RelPath>;
    fn candidates(&'a self, start: usize) -> Self::Candidates;
    fn path_style(&self) -> PathStyle;
}

impl PartialEq for PathMatch {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Eq for PathMatch {}

impl PartialOrd for PathMatch {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PathMatch {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score
            .partial_cmp(&other.score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| self.worktree_id.cmp(&other.worktree_id))
            .then_with(|| {
                other
                    .distance_to_relative_ancestor
                    .cmp(&self.distance_to_relative_ancestor)
            })
            .then_with(|| self.path.cmp(&other.path))
    }
}

/// 两个路径相距几「步」：从共同祖先往下各数一层，加 1。
pub(crate) fn distance_between_paths(path: &RelPath, relative_to: &RelPath) -> usize {
    let mut path_components = path.components();
    let mut relative_components = relative_to.components();

    while path_components
        .next()
        .zip(relative_components.next())
        .map(|(path_component, relative_component)| path_component == relative_component)
        .unwrap_or_default()
    {}
    path_components.count() + relative_components.count() + 1
}

/// 文件名单独再匹配一次，命中越密加分越多 —— 让 `parser.rs` 比
/// `src/deep/nested/parser_helper.txt` 排得靠前。
#[inline]
fn get_filename_match_bonus(
    candidate_buf: &str,
    pattern: &Pattern,
    matcher: &mut nucleo::Matcher,
) -> f64 {
    // 这里作用在**拼接后的展示串**上，所以用 std 的 Path 剥文件名（zed 同）。
    let Some(filename) = std::path::Path::new(candidate_buf)
        .file_name()
        .and_then(|f| f.to_str())
        .filter(|f| !f.is_empty())
    else {
        return 0.0;
    };
    let mut buf = Vec::new();
    let haystack = Utf32Str::new(filename, &mut buf);
    let score: u32 = pattern
        .atoms
        .iter()
        .filter_map(|atom| atom.score(haystack, matcher))
        .map(|s| s as u32)
        .sum();

    score as f64 / filename.len().max(1) as f64
}

fn path_match_helper<'a>(
    matcher: &mut nucleo::Matcher,
    query: &Query,
    candidates: impl Iterator<Item = PathMatchCandidate<'a>>,
    results: &mut Vec<PathMatch>,
    worktree_id: usize,
    path_prefix: &Arc<RelPath>,
    root_is_file: bool,
    relative_to: &Option<Arc<RelPath>>,
    path_style: PathStyle,
    cancel_flag: &AtomicBool,
) -> Result<(), Cancelled> {
    let mut candidate_buf = if !path_prefix.is_empty() && !root_is_file {
        let mut s = path_prefix.display(path_style).to_string();
        s.push_str(path_style.primary_separator());
        s
    } else {
        String::new()
    };
    let path_prefix_len = candidate_buf.len();
    let mut buf = Vec::new();
    let mut matched_chars: Vec<u32> = Vec::new();
    let mut candidate_chars: Vec<char> = Vec::new();
    for candidate in candidates {
        buf.clear();
        matched_chars.clear();
        if cancel_flag.load(atomic::Ordering::Relaxed) {
            return Err(Cancelled);
        }

        if !candidate.char_bag.is_superset(query.char_bag) {
            continue;
        }

        candidate_buf.truncate(path_prefix_len);
        if root_is_file {
            candidate_buf.push_str(path_prefix.as_unix_str());
        } else {
            candidate_buf.push_str(candidate.path.as_unix_str());
        }

        let haystack = Utf32Str::new(&candidate_buf, &mut buf);

        let Some(score) = query.pattern.indices(haystack, matcher, &mut matched_chars) else {
            continue;
        };

        let case_mismatches = count_case_mismatches(
            query.query_chars.as_deref(),
            &matched_chars,
            &candidate_buf,
            &mut candidate_chars,
        );

        matched_chars.sort_unstable();
        matched_chars.dedup();

        let length_penalty = candidate_buf.len() as f64 * LENGTH_PENALTY;
        let filename_bonus = get_filename_match_bonus(&candidate_buf, &query.pattern, matcher);
        let positive = (score as f64 + filename_bonus) * case_penalty(case_mismatches);
        let adjusted_score = positive - length_penalty;
        let positions = positions_from_sorted(&candidate_buf, &matched_chars);

        results.push(PathMatch {
            score: adjusted_score,
            positions,
            worktree_id,
            path: if root_is_file {
                Arc::clone(path_prefix)
            } else {
                candidate.path.into()
            },
            path_prefix: if root_is_file {
                RelPath::empty_arc()
            } else {
                Arc::clone(path_prefix)
            },
            is_dir: candidate.is_dir,
            distance_to_relative_ancestor: relative_to.as_ref().map_or(usize::MAX, |relative_to| {
                distance_between_paths(candidate.path, relative_to.as_ref())
            }),
        });
    }
    Ok(())
}

/// 单组固定候选（比如「当前 worktree 的全部路径」）的同步匹配。
pub fn match_fixed_path_set(
    candidates: Vec<PathMatchCandidate>,
    worktree_id: usize,
    worktree_root_name: Option<Arc<RelPath>>,
    query: &str,
    case: Case,
    max_results: usize,
    path_style: PathStyle,
) -> Vec<PathMatch> {
    let Some(query) = Query::build(query, case) else {
        return Vec::new();
    };

    let mut config = nucleo::Config::DEFAULT;
    config.set_match_paths();
    let mut matcher = matcher::get_matcher(config);

    let root_is_file = worktree_root_name.is_some() && candidates.iter().all(|c| c.path.is_empty());

    let path_prefix = worktree_root_name.unwrap_or_else(|| RelPath::empty_arc());

    let mut results = Vec::new();

    path_match_helper(
        &mut matcher,
        &query,
        candidates.into_iter(),
        &mut results,
        worktree_id,
        &path_prefix,
        root_is_file,
        &None,
        path_style,
        &AtomicBool::new(false),
    )
    .ok();
    gpui_util::truncate_to_bottom_n_sorted_by(&mut results, max_results, &|a, b| b.cmp(a));
    matcher::return_matcher(matcher);
    results
}

/// 多组候选（多个 worktree / 项目）并行匹配，可取消。
pub async fn match_path_sets<'a, Set: PathMatchCandidateSet<'a>>(
    candidate_sets: &'a [Set],
    query: &str,
    relative_to: &Option<Arc<RelPath>>,
    case: Case,
    max_results: usize,
    cancel_flag: &AtomicBool,
    executor: BackgroundExecutor,
) -> Vec<PathMatch> {
    let path_count: usize = candidate_sets.iter().map(|s| s.len()).sum();
    if path_count == 0 {
        return Vec::new();
    }

    let path_style = candidate_sets[0].path_style();

    let query = if path_style.is_windows() {
        query.replace('\\', "/")
    } else {
        query.to_owned()
    };

    let Some(query) = Query::build(&query, case) else {
        return Vec::new();
    };

    let num_cpus = executor.num_cpus().min(path_count);
    let segment_size = path_count.div_ceil(num_cpus);
    let mut segment_results = (0..num_cpus)
        .map(|_| Vec::with_capacity(max_results))
        .collect::<Vec<_>>();
    let mut config = nucleo::Config::DEFAULT;
    config.set_match_paths();
    let mut matchers = matcher::get_matchers(num_cpus, config);
    executor
        .scoped(|scope| {
            for (segment_idx, (results, matcher)) in segment_results
                .iter_mut()
                .zip(matchers.iter_mut())
                .enumerate()
            {
                let query = &query;
                let relative_to = relative_to.clone();
                scope.spawn(async move {
                    let segment_start = segment_idx * segment_size;
                    let segment_end = segment_start + segment_size;

                    let mut tree_start = 0;
                    for candidate_set in candidate_sets {
                        let tree_end = tree_start + candidate_set.len();

                        if tree_start < segment_end && segment_start < tree_end {
                            let start = tree_start.max(segment_start) - tree_start;
                            let end = tree_end.min(segment_end) - tree_start;
                            let candidates = candidate_set.candidates(start).take(end - start);

                            if path_match_helper(
                                matcher,
                                query,
                                candidates,
                                results,
                                candidate_set.id(),
                                &candidate_set.prefix(),
                                candidate_set.root_is_file(),
                                &relative_to,
                                path_style,
                                cancel_flag,
                            )
                            .is_err()
                            {
                                break;
                            }
                        }

                        if tree_end >= segment_end {
                            break;
                        }
                        tree_start = tree_end;
                    }
                });
            }
        })
        .await;

    matcher::return_matchers(matchers);
    if cancel_flag.load(atomic::Ordering::Acquire) {
        return Vec::new();
    }

    let mut results = segment_results.concat();
    gpui_util::truncate_to_bottom_n_sorted_by(&mut results, max_results, &|a, b| b.cmp(a));
    results
}

/// 一个最朴素的候选集实现：直接抱着 `Vec<RelPathBuf>`。
///
/// 真实场景里通常由 worktree / 项目自己实现 [`PathMatchCandidateSet`]，
/// 这个实现主要给测试和简单调用方用。
#[derive(Default)]
pub struct PathSet {
    paths: Vec<(RelPathBuf, bool)>,
}

impl PathSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, path: impl Into<RelPathBuf>, is_dir: bool) {
        self.paths.push((path.into(), is_dir));
    }
}

impl<'a> PathMatchCandidateSet<'a> for PathSet {
    type Candidates = std::vec::IntoIter<PathMatchCandidate<'a>>;

    fn id(&self) -> usize {
        0
    }

    fn len(&self) -> usize {
        self.paths.len()
    }

    fn root_is_file(&self) -> bool {
        false
    }

    fn prefix(&self) -> Arc<RelPath> {
        RelPath::empty_arc()
    }

    fn candidates(&'a self, start: usize) -> Self::Candidates {
        let empty = RelPath::empty();
        self.paths[start..]
            .iter()
            .map(|(path, is_dir)| PathMatchCandidate::new(path, *is_dir, Some(empty)))
            .collect::<Vec<_>>()
            .into_iter()
    }

    fn path_style(&self) -> PathStyle {
        PathStyle::local()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试里造一个 `&'static RelPath`：`RelPath` 内部是 String，
    /// 泄漏一次即可拿到 `'static` 引用（测试进程无所谓）。
    fn rel(path: &str) -> &'static RelPath {
        Box::leak(rel_path_buf(path).into())
    }

    fn rel_path_buf(path: &str) -> RelPathBuf {
        RelPath::new(std::path::Path::new(path), PathStyle::Unix)
            .expect("test path should be relative")
            .into_owned()
    }

    fn candidates(paths: &[&str]) -> Vec<PathMatchCandidate<'static>> {
        paths
            .iter()
            .map(|p| PathMatchCandidate::new(rel(p), false, None))
            .collect()
    }

    #[test]
    fn test_match_fixed_path_set_basic() {
        let cs = candidates(&["src/lib/parser.rs", "src/bin/main.rs", "README.md"]);
        let results =
            match_fixed_path_set(cs, 0, None, "parser", Case::Ignore, 10, PathStyle::Unix);
        let matched: Vec<String> = results
            .iter()
            .map(|m| m.path.as_unix_str().to_string())
            .collect();
        assert_eq!(matched, vec!["src/lib/parser.rs"]);
    }

    #[test]
    fn test_match_fixed_path_set_with_prefix() {
        let cs = candidates(&["lib/parser.rs", "lib/main.rs"]);
        let results = match_fixed_path_set(
            cs,
            0,
            Some(rel("myproject").into()),
            "mai",
            Case::Ignore,
            10,
            PathStyle::Unix,
        );
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path.as_unix_str(), "lib/main.rs");
        assert_eq!(results[0].path_prefix.as_unix_str(), "myproject");
        // 位置是相对「前缀 + 分隔符 + 路径」拼出来的串，所以 'm' 落在前缀之后。
        assert!(results[0].positions.iter().all(|p| *p > "myproject/".len()));
    }

    #[test]
    fn test_filename_bonus_prefers_short_filename() {
        let cs = candidates(&["parser.rs", "src/deep/nested/parser_helper.txt"]);
        let results =
            match_fixed_path_set(cs, 0, None, "parser", Case::Ignore, 10, PathStyle::Unix);
        assert_eq!(results[0].path.as_unix_str(), "parser.rs");
    }

    #[test]
    fn test_is_dir_and_worktree_id_are_carried() {
        let cs = vec![
            PathMatchCandidate::new(rel("src"), true, None),
            PathMatchCandidate::new(rel("src/main.rs"), false, None),
        ];
        let results = match_fixed_path_set(cs, 7, None, "src", Case::Ignore, 10, PathStyle::Unix);
        assert!(results.iter().all(|m| m.worktree_id == 7));
        assert!(
            results
                .iter()
                .any(|m| m.is_dir && m.path.as_ref() == rel("src"))
        );
    }

    #[test]
    fn test_empty_query_returns_nothing_for_paths() {
        // 与字符串版不同：路径匹配没有「空查询返回全部」的分支（zed 同）。
        let cs = candidates(&["a.rs", "b.rs"]);
        let results = match_fixed_path_set(cs, 0, None, "", Case::Ignore, 10, PathStyle::Unix);
        assert!(results.is_empty());
    }

    #[test]
    fn test_max_results() {
        let cs = candidates(&["a/one.rs", "a/two.rs", "a/three.rs"]);
        let results = match_fixed_path_set(cs, 0, None, "a", Case::Ignore, 2, PathStyle::Unix);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_distance_between_paths() {
        // 同一目录下的两个文件：共同前缀走完后两边都没剩余 → 1。
        assert_eq!(distance_between_paths(rel("a/b/c.rs"), rel("a/b/d.rs")), 1);
        assert_eq!(distance_between_paths(rel("a/b.rs"), rel("a/b.rs")), 1);
        // 只有第一段就分叉：两边各剩 1 层 → 1 + 1 + 1 = 3。
        assert_eq!(distance_between_paths(rel("x/y.rs"), rel("p/q.rs")), 3);
    }

    #[test]
    fn test_path_style() {
        assert_eq!(PathStyle::Unix.primary_separator(), "/");
        assert_eq!(PathStyle::Windows.primary_separator(), "\\");
        assert!(PathStyle::Windows.is_windows());
        assert!(!PathStyle::Unix.is_windows());
    }

    #[gpui::test]
    async fn test_match_path_sets(executor: BackgroundExecutor) {
        let mut set = PathSet::new();
        set.push(rel_path_buf("src/lib/parser.rs"), false);
        set.push(rel_path_buf("src/bin/main.rs"), false);
        set.push(rel_path_buf("tests/parser_test.rs"), false);

        let relative_to: Option<Arc<RelPath>> = Some(rel("src/bin").into());
        let results = match_path_sets(
            &[set],
            "parser",
            &relative_to,
            Case::Ignore,
            10,
            &AtomicBool::new(false),
            executor,
        )
        .await;

        let matched: Vec<String> = results
            .iter()
            .map(|m| m.path.as_unix_str().to_string())
            .collect();
        assert_eq!(matched.len(), 2);
        // 离 relative_to 更近的排在前面。
        assert_eq!(matched[0], "src/lib/parser.rs");
        assert_eq!(matched[1], "tests/parser_test.rs");
    }

    #[gpui::test]
    async fn test_match_path_sets_cancelled(executor: BackgroundExecutor) {
        let mut set = PathSet::new();
        set.push(rel_path_buf("src/lib/parser.rs"), false);
        let results = match_path_sets(
            &[set],
            "parser",
            &None,
            Case::Ignore,
            10,
            &AtomicBool::new(true),
            executor,
        )
        .await;
        assert!(results.is_empty());
    }

    #[gpui::test]
    async fn test_match_path_sets_windows_query(executor: BackgroundExecutor) {
        // Windows 风格下查询里的 '\' 会被换成 '/'，所以 "src\main" 也能命中。
        let mut set = PathSet::new();
        set.push(rel_path_buf("src/main.rs"), false);

        struct WinSet(PathSet);
        impl<'a> PathMatchCandidateSet<'a> for WinSet {
            type Candidates = std::vec::IntoIter<PathMatchCandidate<'a>>;
            fn id(&self) -> usize {
                self.0.id()
            }
            fn len(&self) -> usize {
                self.0.len()
            }
            fn root_is_file(&self) -> bool {
                self.0.root_is_file()
            }
            fn prefix(&self) -> Arc<RelPath> {
                self.0.prefix()
            }
            fn candidates(&'a self, start: usize) -> Self::Candidates {
                self.0.candidates(start)
            }
            fn path_style(&self) -> PathStyle {
                PathStyle::Windows
            }
        }

        let results = match_path_sets(
            &[WinSet(set)],
            "src\\main",
            &None,
            Case::Ignore,
            10,
            &AtomicBool::new(false),
            executor,
        )
        .await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path.as_unix_str(), "src/main.rs");
    }
}
