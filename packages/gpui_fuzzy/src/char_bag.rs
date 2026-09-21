//! 候选预筛器：把字符串压成一个 `u64` 位集合，用一次按位与判断「查询需要的字符
//! 是否够」。
//!
//! 语义与 zed `crates/fuzzy/src/char_bag.rs` 一致（GPL-3.0，我们只搬了这 60 行的
//! 思路与常量布局）：
//!
//! - 小写字母 `a..=z`：各占 **2 位**，存「出现次数」的饱和计数（0/1/2/3，即
//!   `1` 次记 `01`、`2` 次记 `11`（3）、`3` 次以上也是 `11`）；
//! - 数字 `0..=9`：各占 1 位（第 52 位起），只记「有没有」；
//! - `-`：第 62 位；
//! - 其余字符（含非 ASCII）**直接忽略** —— 所以它是「必要不充分」的过滤器：
//!   通过预筛不代表一定匹配，不通过一定不匹配。

/// 查询 / 候选字符串的字符多重集摘要。
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct CharBag(u64);

pub fn simple_lowercase(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

impl CharBag {
    /// 查询 bag 是候选 bag 的子集时返回 true —— 也就是候选「有可能」匹配。
    pub fn is_superset(self, other: CharBag) -> bool {
        self.0 & other.0 == other.0
    }

    fn insert(&mut self, c: char) {
        let c = simple_lowercase(c);
        if c.is_ascii_lowercase() {
            let mut count = self.0;
            let idx = c as u8 - b'a';
            count >>= idx * 2;
            count = ((count << 1) | 1) & 3;
            count <<= idx * 2;
            self.0 |= count;
        } else if c.is_ascii_digit() {
            let idx = c as u8 - b'0';
            self.0 |= 1 << (idx + 52);
        } else if c == '-' {
            self.0 |= 1 << 62;
        }
    }
}

impl Extend<char> for CharBag {
    fn extend<T: IntoIterator<Item = char>>(&mut self, iter: T) {
        for c in iter {
            self.insert(c);
        }
    }
}

impl FromIterator<char> for CharBag {
    fn from_iter<T: IntoIterator<Item = char>>(iter: T) -> Self {
        let mut result = Self::default();
        result.extend(iter);
        result
    }
}

impl From<&str> for CharBag {
    fn from(s: &str) -> Self {
        let mut bag = Self(0);
        for c in s.chars() {
            bag.insert(c);
        }
        bag
    }
}

impl From<&[char]> for CharBag {
    fn from(chars: &[char]) -> Self {
        let mut bag = Self(0);
        for c in chars {
            bag.insert(*c);
        }
        bag
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_superset() {
        let query = CharBag::from("abc");
        assert!(CharBag::from("abcdef").is_superset(query));
        assert!(CharBag::from("aabbcc").is_superset(query));
        assert!(!CharBag::from("abd").is_superset(query));
    }

    #[test]
    fn test_counts_repetitions_up_to_three() {
        // 'a' 出现 1 / 2 / 3 次是不同的 bag，4 次与 3 次相同（饱和）。
        let one = CharBag::from("a");
        let two = CharBag::from("aa");
        let three = CharBag::from("aaa");
        let four = CharBag::from("aaaa");

        assert!(!one.is_superset(two));
        assert!(two.is_superset(one));
        assert!(three.is_superset(two));
        assert_eq!(three, four, "计数在 3 处饱和");
    }

    #[test]
    fn test_case_insensitive() {
        assert_eq!(CharBag::from("ABC"), CharBag::from("abc"));
    }

    #[test]
    fn test_digits_and_dash() {
        assert!(CharBag::from("file-1").is_superset(CharBag::from("1")));
        assert!(CharBag::from("file-1").is_superset(CharBag::from("-")));
        assert!(!CharBag::from("file1").is_superset(CharBag::from("-")));
    }

    #[test]
    fn test_non_ascii_ignored() {
        // 非 ASCII 字符直接忽略，所以中文候选的 bag 是「空」的——任何查询都能过预筛。
        assert_eq!(CharBag::from("中文"), CharBag::default());
    }
}
