use super::*;

#[test]
fn parses_short_sha1_oid_by_padding_trailing_nibbles() {
    let oid = "abc1234".parse::<Oid>().expect("failed to parse oid");

    assert_eq!(oid.as_bytes().len(), 20);
    assert_eq!(oid.display_short(), "abc1234");
    assert_eq!(oid.to_string(), "abc1234000000000000000000000000000000000");
}

#[test]
fn parses_full_sha256_oid() {
    let sha = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let oid = sha.parse::<Oid>().expect("failed to parse oid");

    assert_eq!(oid.as_bytes().len(), 32);
    assert_eq!(oid.to_string(), sha);
}

#[test]
fn rejects_invalid_oid_lengths() {
    assert!("".parse::<Oid>().is_err());
    assert!("a".repeat(41).parse::<Oid>().is_err());
    assert!("a".repeat(63).parse::<Oid>().is_err());
}
