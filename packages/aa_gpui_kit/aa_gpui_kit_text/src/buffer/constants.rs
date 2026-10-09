/// The maximum length of a single insertion operation.
///
/// Fragments larger than this will be split into multiple smaller
/// fragments. This allows us to use relative `u32` offsets instead of `usize`,
/// reducing memory usage.
pub const MAX_INSERTION_LEN: usize = if cfg!(test) { 16 } else { u32::MAX as usize };
