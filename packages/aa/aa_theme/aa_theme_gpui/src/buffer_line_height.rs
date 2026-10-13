//! 编辑器行高，对齐 zed `crates/theme/src/buffer_line_height.rs`。
//!
//! 行高用**倍数**表达（不是像素），乘到字体行高上即为实际行距——
//! 这样换字体或改字号时行距自动跟随。
//!
//! 目前只有数据结构，尚无消费方：渲染层接入需等编辑器支持可配置行高。

/// 编辑器行的行高倍数。
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum BufferLineHeight {
    /// 更疏的行距。
    #[default]
    Comfortable,
    /// 默认行距。
    Standard,
    /// 自定义倍数（1.0 即字体本身高度，须 >= 1.0）。
    Custom(f32),
}

impl BufferLineHeight {
    /// 行高倍数。
    pub fn value(&self) -> f32 {
        match self {
            BufferLineHeight::Comfortable => 1.618,
            BufferLineHeight::Standard => 1.3,
            BufferLineHeight::Custom(line_height) => *line_height,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comfortable_is_looser_than_standard() {
        assert!(BufferLineHeight::Comfortable.value() > BufferLineHeight::Standard.value());
        assert_eq!(BufferLineHeight::default(), BufferLineHeight::Comfortable);
    }

    #[test]
    fn custom_returns_its_value() {
        assert_eq!(BufferLineHeight::Custom(2.0).value(), 2.0);
    }
}

