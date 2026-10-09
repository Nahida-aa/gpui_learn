//! `nucleo::Matcher` 的复用池 —— 每个 Matcher 内部有打分用的缓冲区，
//! 每次查询新建会很贵（尤其路径匹配会开 `match_paths` 配置），所以按 CPU 数
//! 上限缓存起来。与 zed `crates/fuzzy_nucleo/src/matcher.rs` 同构。

use std::sync::Mutex;

/// 池容量上限：并行度有多少就最多留多少，避免长期占住内存。
static MATCHERS: Mutex<Vec<nucleo::Matcher>> = Mutex::new(Vec::new());

/// 长度惩罚系数：候选每字节扣的分。
pub const LENGTH_PENALTY: f64 = 0.01;

fn pool_cap() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(8)
        .max(1)
}

pub fn get_matcher(config: nucleo::Config) -> nucleo::Matcher {
    let mut matchers = MATCHERS.lock().unwrap_or_else(|e| e.into_inner());
    match matchers.pop() {
        Some(mut matcher) => {
            matcher.config = config;
            matcher
        }
        None => nucleo::Matcher::new(config),
    }
}

pub fn return_matcher(matcher: nucleo::Matcher) {
    let mut pool = MATCHERS.lock().unwrap_or_else(|e| e.into_inner());
    if pool.len() < pool_cap() {
        pool.push(matcher);
    }
}

pub fn get_matchers(n: usize, config: nucleo::Config) -> Vec<nucleo::Matcher> {
    let mut matchers: Vec<_> = {
        let mut pool = MATCHERS.lock().unwrap_or_else(|e| e.into_inner());
        let available = pool.len().min(n);
        pool.drain(..available)
            .map(|mut matcher| {
                matcher.config = config.clone();
                matcher
            })
            .collect()
    };
    matchers.resize_with(n, || nucleo::Matcher::new(config.clone()));
    matchers
}

pub fn return_matchers(matchers: Vec<nucleo::Matcher>) {
    let cap = pool_cap();
    let mut pool = MATCHERS.lock().unwrap_or_else(|e| e.into_inner());
    let space = cap.saturating_sub(pool.len());
    pool.extend(matchers.into_iter().take(space));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_and_return_matcher() {
        let config = nucleo::Config::DEFAULT;
        let matcher = get_matcher(config.clone());
        return_matcher(matcher);

        // 归还后应当能再次取到（池未满时）。
        let again = get_matcher(config);
        return_matcher(again);
    }

    #[test]
    fn test_get_matchers_returns_exactly_n() {
        let matchers = get_matchers(4, nucleo::Config::DEFAULT);
        assert_eq!(matchers.len(), 4);
        return_matchers(matchers);
    }
}
