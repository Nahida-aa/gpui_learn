#[cold]
#[inline(never)]
#[track_caller]
pub(crate) fn panic_char_boundary(text: &str, offset: usize) -> ! {
    if offset > text.len() {
        panic!(
            "byte index {} is out of bounds of `{:?}` (length: {})",
            offset,
            text,
            text.len()
        );
    }
    // find the character
    let char_start = text.floor_char_boundary(offset);
    // `char_start` must be less than len and a char boundary
    let ch = text.get(char_start..).unwrap().chars().next().unwrap();
    let char_range = char_start..char_start + ch.len_utf8();
    panic!(
        "byte index {} is not a char boundary; it is inside {:?} (bytes {:?})",
        offset, ch, char_range,
    );
}

#[cold]
#[inline(never)]
#[track_caller]
pub(crate) fn log_err_char_boundary(text: &str, offset: usize) {
    if offset >= text.len() {
        log::error!(
            "byte index {} is out of bounds of `{:?}` (length: {})",
            offset,
            text,
            text.len()
        );
        return;
    }
    // find the character
    let char_start = text.floor_char_boundary(offset);
    // `char_start` must be less than len and a char boundary
    let ch = text.get(char_start..).unwrap().chars().next().unwrap();
    let char_range = char_start..char_start + ch.len_utf8();
    log::error!(
        "byte index {} is not a char boundary; it is inside {:?} (bytes {:?})",
        offset,
        ch,
        char_range,
    );
}
