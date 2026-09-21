//! 模糊匹配：给 picker / 命令面板 / 文件查找器用的打分与高亮。
//!
//! 对齐 zed `crates/fuzzy_nucleo`（zed 正在从自研 `fuzzy` 迁到 `nucleo`，我们直接
//! 用终态那一版）：打分交给 [`nucleo`](https://docs.rs/nucleo)，zed 自己留的三件
//! 东西我们照搬 ——
//!
//! 1. [`CharBag`] 预筛：查询字符的位集合，先用一次 `&` 秒拒「连字符都不够」的候选；
//! 2. [`Case::Smart`] 惩罚：nucleo 层只能用 `CaseMatching::Ignore`（用 Smart 会把
//!    `"editor: backspace"` 这种大小写不符的候选直接判掉），所以大小写不符改成
//!    **扣分而不是淘汰**（每个不符字符乘 [`SMART_CASE_PENALTY_PER_MISMATCH`]）；
//! 3. 路径专用打分（文件名加分 + 长度惩罚 + 相对距离排序）。
//!
//! 与 zed 的**唯一 API 差异**：路径类型用 `std::path::Path` 而不是 zed 的
//! `path::RelPath`（我们没有那个 crate，因此 `PathStyle` 也自给了最小实现）。
//!
//! 典型用法：
//!
//! ```no_run
//! use aa_gpui_fuzzy::{Case, LengthPenalty, StringMatchCandidate, match_strings};
//!
//! // 同步版（候选少 / 已经在后台线程上）
//! let candidates: Vec<_> = ["hello", "help", "world"]
//!     .iter()
//!     .enumerate()
//!     .map(|(id, s)| StringMatchCandidate::new(id, *s))
//!     .collect();
//! let matches = match_strings(&candidates, "hel", Case::Ignore, LengthPenalty::Off, 10);
//!
//! // 异步版（候选多，按 CPU 分段并行 + 可取消）
//! async fn search(
//!     candidates: &[StringMatchCandidate],
//!     executor: gpui::BackgroundExecutor,
//! ) -> Vec<aa_gpui_fuzzy::StringMatch> {
//!     let cancel = std::sync::atomic::AtomicBool::new(false);
//!     aa_gpui_fuzzy::match_strings_async(
//!         candidates, "hel", Case::Ignore, LengthPenalty::Off, 10, &cancel, executor,
//!     )
//!     .await
//! }
//! ```

mod char_bag;
mod matcher;
mod paths;
mod strings;

pub use char_bag::CharBag;
pub use paths::{
    PathMatch, PathMatchCandidate, PathMatchCandidateSet, PathSet, PathStyle, match_fixed_path_set,
    match_path_sets,
};
pub use strings::{StringMatch, StringMatchCandidate, match_strings, match_strings_async};

use nucleo::pattern::{AtomKind, CaseMatching, Normalization, Pattern};

pub(crate) struct Cancelled;

/// 大小写敏感度。
///
/// 注意它**不会**淘汰大小写不符的候选，只影响排序，见模块文档第 2 点。
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Case {
    Smart,
    Ignore,
}

impl Case {
    pub fn smart_if_uppercase_in(query: &str) -> Self {
        if query.chars().any(|c| c.is_uppercase()) {
            Self::Smart
        } else {
            Self::Ignore
        }
    }

    pub fn is_smart(self) -> bool {
        matches!(self, Self::Smart)
    }
}

/// 是否按候选长度扣分（长路径 / 长名字往后排）。
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LengthPenalty {
    On,
    Off,
}

impl LengthPenalty {
    pub fn from_bool(on: bool) -> Self {
        if on { Self::On } else { Self::Off }
    }

    pub fn is_on(self) -> bool {
        matches!(self, Self::On)
    }
}

// Matching is always case-insensitive at the nucleo level — using
// `CaseMatching::Smart` there would *reject* candidates whose capitalization
// doesn't match the query, breaking pickers like the command palette
// (`"Editor: Backspace"` against the action named `"editor: backspace"`).
// `Case::Smart` is honored as a *scoring hint* instead: when the query
// contains uppercase, candidates whose matched characters disagree in case
// are downranked by a per-mismatch penalty rather than dropped.
pub(crate) struct Query {
    pub(crate) pattern: Pattern,
    /// Non-whitespace query chars in input order, populated only when a smart-case
    /// penalty will actually be charged. Aligns 1:1 with the indices appended by
    /// `Pattern::indices` (atom-order, needle-order within each atom).
    pub(crate) query_chars: Option<Vec<char>>,
    pub(crate) char_bag: CharBag,
}

impl Query {
    pub(crate) fn build(query: &str, case: Case) -> Option<Self> {
        if query.chars().all(char::is_whitespace) {
            return None;
        }
        let normalized = query.split_whitespace().collect::<Vec<_>>().join(" ");
        let pattern = Pattern::new(
            &normalized,
            CaseMatching::Ignore,
            Normalization::Smart,
            AtomKind::Fuzzy,
        );
        let wants_case_penalty = case.is_smart() && query.chars().any(|c| c.is_uppercase());
        let query_chars =
            wants_case_penalty.then(|| query.chars().filter(|c| !c.is_whitespace()).collect());
        Some(Query {
            pattern,
            query_chars,
            char_bag: CharBag::from(query),
        })
    }
}

#[inline]
pub(crate) fn count_case_mismatches(
    query_chars: Option<&[char]>,
    matched_chars: &[u32],
    candidate: &str,
    candidate_chars: &mut Vec<char>,
) -> u32 {
    let Some(query_chars) = query_chars else {
        return 0;
    };
    if query_chars.len() != matched_chars.len() {
        return 0;
    }
    candidate_chars.clear();
    candidate_chars.extend(candidate.chars());
    let mut mismatches: u32 = 0;
    for (&query_char, &pos) in query_chars.iter().zip(matched_chars) {
        if let Some(&candidate_char) = candidate_chars.get(pos as usize)
            && candidate_char != query_char
            && candidate_char.eq_ignore_ascii_case(&query_char)
        {
            mismatches += 1;
        }
    }
    mismatches
}

/// 每个大小写不符字符的扣分系数（乘算，所以不符越多扣得越狠）。
pub const SMART_CASE_PENALTY_PER_MISMATCH: f64 = 0.9;

#[inline]
pub(crate) fn case_penalty(mismatches: u32) -> f64 {
    if mismatches == 0 {
        1.0
    } else {
        SMART_CASE_PENALTY_PER_MISMATCH.powi(mismatches as i32)
    }
}

/// Reconstruct byte-offset match positions from a list of matched char offsets
/// that is already sorted ascending and deduplicated.
pub(crate) fn positions_from_sorted(s: &str, sorted_char_indices: &[u32]) -> Vec<usize> {
    let mut iter = sorted_char_indices.iter().copied().peekable();
    let mut out = Vec::with_capacity(sorted_char_indices.len());
    for (char_offset, (byte_offset, _)) in s.char_indices().enumerate() {
        if iter.peek().is_none() {
            break;
        }
        if iter.next_if(|&m| m == char_offset as u32).is_some() {
            out.push(byte_offset);
        }
    }
    out
}
