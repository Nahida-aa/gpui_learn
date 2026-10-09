use super::*;

#[test]
fn test_fs_embed_iter_and_get() {
    let inherent_names: Vec<_> = FsEmbedTestAssets::iter().collect();
    assert!(
        inherent_names.iter().any(|name| name == "util.rs"),
        "iter should list files matching the include globs, got {inherent_names:?}"
    );
    assert!(
        !inherent_names.iter().any(|name| name.ends_with(".toml")),
        "iter should filter files excluded by the globs, got {inherent_names:?}"
    );

    let trait_names: Vec<_> =
        <FsEmbedTestAssets as crate::__rust_embed::RustEmbed>::iter().collect();
    assert_eq!(inherent_names, trait_names);

    let file = FsEmbedTestAssets::get("util.rs").expect("util.rs should be readable");
    assert!(
        std::str::from_utf8(&file.data)
            .expect("util.rs should be utf-8")
            .contains("fs_embed"),
        "get should read the real file contents"
    );
    assert!(
        FsEmbedTestAssets::get("Cargo.toml").is_none(),
        "get should filter files excluded by the globs"
    );
}

// 注意：原文件里有两段 `crate::fs_embed! { ... }`，它们必须留在 crate 根
// 或者能被 `fs_embed!` 宏展开到的地方。最简单是在 `tests.rs` 里 `use crate::fs_embed;`
// 之后直接调用 `fs_embed! { ... }`；`fs_embed!` 是 `#[macro_export]`，通过
// `use crate::fs_embed;` 引入后即可正常展开到当前模块。

#[cfg(not(feature = "debug-embed"))]
crate::fs_embed! {
    struct FsEmbedTestAssets,
    crate_relative = "src",
    root_relative = "crates/util/src",
    include = ["*.rs"],
    exclude = ["test/**/*"],
}

#[cfg(feature = "debug-embed")]
crate::fs_embed! {
    struct FsEmbedTestAssets,
    crate_relative = "src",
    root_relative = "this-path-must-not-be-read",
    include = ["*.rs"],
    exclude = ["test/**/*"],
}

#[test]
fn test_parse_os_release() {
    let os_release =
        "NAME=\"Ubuntu\"\nID=ubuntu\nVERSION_ID=\"24.04\"\nPRETTY_NAME=\"Ubuntu 24.04 LTS\"\n";
    assert_eq!(
        parse_os_release(os_release),
        Some("ubuntu 24.04".to_string())
    );

    // VERSION_ID may be absent (e.g. rolling releases like Arch).
    assert_eq!(parse_os_release("ID=arch\n"), Some("arch".to_string()));

    // Without an ID there is nothing usable to report.
    assert_eq!(parse_os_release("VERSION_ID=1\n"), None);
    assert_eq!(parse_os_release(""), None);
}

#[test]
fn test_extend_sorted() {
    let mut vec = vec![];

    extend_sorted(&mut vec, vec![21, 17, 13, 8, 1, 0], 5, |a, b| b.cmp(a));
    assert_eq!(vec, &[21, 17, 13, 8, 1]);

    extend_sorted(&mut vec, vec![101, 19, 17, 8, 2], 8, |a, b| b.cmp(a));
    assert_eq!(vec, &[101, 21, 19, 17, 13, 8, 2, 1]);

    extend_sorted(&mut vec, vec![1000, 19, 17, 9, 5], 8, |a, b| b.cmp(a));
    assert_eq!(vec, &[1000, 101, 21, 19, 17, 13, 9, 8]);
}

#[test]
fn test_truncate_to_bottom_n_sorted_by() {
    let mut vec: Vec<u32> = vec![5, 2, 3, 4, 1];
    truncate_to_bottom_n_sorted_by(&mut vec, 10, &u32::cmp);
    assert_eq!(vec, &[1, 2, 3, 4, 5]);

    vec = vec![5, 2, 3, 4, 1];
    truncate_to_bottom_n_sorted_by(&mut vec, 5, &u32::cmp);
    assert_eq!(vec, &[1, 2, 3, 4, 5]);

    vec = vec![5, 2, 3, 4, 1];
    truncate_to_bottom_n_sorted_by(&mut vec, 4, &u32::cmp);
    assert_eq!(vec, &[1, 2, 3, 4]);

    vec = vec![5, 2, 3, 4, 1];
    truncate_to_bottom_n_sorted_by(&mut vec, 1, &u32::cmp);
    assert_eq!(vec, &[1]);

    vec = vec![5, 2, 3, 4, 1];
    truncate_to_bottom_n_sorted_by(&mut vec, 0, &u32::cmp);
    assert!(vec.is_empty());
}

#[test]
fn test_iife() {
    fn option_returning_function() -> Option<()> { None }

    let foo = maybe!({
        option_returning_function()?;
        Some(())
    });

    assert_eq!(foo, None);
}

#[test]
fn test_truncate_and_trailoff() {
    assert_eq!(truncate_and_trailoff("", 5), "");
    assert_eq!(truncate_and_trailoff("aaaaaa", 7), "aaaaaa");
    assert_eq!(truncate_and_trailoff("aaaaaa", 6), "aaaaaa");
    assert_eq!(truncate_and_trailoff("aaaaaa", 5), "aaaaa…");
    assert_eq!(truncate_and_trailoff("èèèèèè", 7), "èèèèèè");
    assert_eq!(truncate_and_trailoff("èèèèèè", 6), "èèèèèè");
    assert_eq!(truncate_and_trailoff("èèèèèè", 5), "èèèèè…");
}

#[test]
fn test_truncate_and_remove_front() {
    assert_eq!(truncate_and_remove_front("", 5), "");
    assert_eq!(truncate_and_remove_front("aaaaaa", 7), "aaaaaa");
    assert_eq!(truncate_and_remove_front("aaaaaa", 6), "aaaaaa");
    assert_eq!(truncate_and_remove_front("aaaaaa", 5), "…aaaaa");
    assert_eq!(truncate_and_remove_front("èèèèèè", 7), "èèèèèè");
    assert_eq!(truncate_and_remove_front("èèèèèè", 6), "èèèèèè");
    assert_eq!(truncate_and_remove_front("èèèèèè", 5), "…èèèèè");
}

#[test]
fn test_numeric_prefix_str_method() {
    let target = "1a";
    assert_eq!(
        NumericPrefixWithSuffix::from_numeric_prefixed_str(target),
        NumericPrefixWithSuffix(Some(1), "a")
    );

    let target = "12ab";
    assert_eq!(
        NumericPrefixWithSuffix::from_numeric_prefixed_str(target),
        NumericPrefixWithSuffix(Some(12), "ab")
    );

    let target = "12_ab";
    assert_eq!(
        NumericPrefixWithSuffix::from_numeric_prefixed_str(target),
        NumericPrefixWithSuffix(Some(12), "_ab")
    );

    let target = "1_2ab";
    assert_eq!(
        NumericPrefixWithSuffix::from_numeric_prefixed_str(target),
        NumericPrefixWithSuffix(Some(1), "_2ab")
    );

    let target = "1.2";
    assert_eq!(
        NumericPrefixWithSuffix::from_numeric_prefixed_str(target),
        NumericPrefixWithSuffix(Some(1), ".2")
    );

    let target = "1.2_a";
    assert_eq!(
        NumericPrefixWithSuffix::from_numeric_prefixed_str(target),
        NumericPrefixWithSuffix(Some(1), ".2_a")
    );

    let target = "12.2_a";
    assert_eq!(
        NumericPrefixWithSuffix::from_numeric_prefixed_str(target),
        NumericPrefixWithSuffix(Some(12), ".2_a")
    );

    let target = "12a.2_a";
    assert_eq!(
        NumericPrefixWithSuffix::from_numeric_prefixed_str(target),
        NumericPrefixWithSuffix(Some(12), "a.2_a")
    );
}

#[test]
fn test_numeric_prefix_with_suffix() {
    let mut sorted = vec!["1-abc", "10", "11def", "2", "21-abc"];
    sorted.sort_by_key(|s| NumericPrefixWithSuffix::from_numeric_prefixed_str(s));
    assert_eq!(sorted, ["1-abc", "2", "10", "11def", "21-abc"]);

    for numeric_prefix_less in ["numeric_prefix_less", "aaa", "~™£"] {
        assert_eq!(
            NumericPrefixWithSuffix::from_numeric_prefixed_str(numeric_prefix_less),
            NumericPrefixWithSuffix(None, numeric_prefix_less),
            "String without numeric prefix `{numeric_prefix_less}` should not be converted into NumericPrefixWithSuffix"
        )
    }
}

#[test]
fn test_word_consists_of_emojis() {
    let words_to_test = vec![
        ("👨‍👩‍👧‍👧👋🥒", true),
        ("👋", true),
        ("!👋", false),
        ("👋!", false),
        ("👋 ", false),
        (" 👋", false),
        ("Test", false),
    ];

    for (text, expected_result) in words_to_test {
        assert_eq!(word_consists_of_emojis(text), expected_result);
    }
}

#[test]
fn test_truncate_lines_and_trailoff() {
    let text = r#"Line 1
Line 2
Line 3"#;

    assert_eq!(
        truncate_lines_and_trailoff(text, 2),
        r#"Line 1
…"#
    );

    assert_eq!(
        truncate_lines_and_trailoff(text, 3),
        r#"Line 1
Line 2
…"#
    );

    assert_eq!(
        truncate_lines_and_trailoff(text, 4),
        r#"Line 1
Line 2
Line 3"#
    );
}

#[test]
fn test_expanded_and_wrapped_usize_range() {
    // Neither wrap
    assert_eq!(
        expanded_and_wrapped_usize_range(2..4, 1, 1, 8).collect::<Vec<usize>>(),
        (1..5).collect::<Vec<usize>>()
    );
    // Start wraps
    assert_eq!(
        expanded_and_wrapped_usize_range(2..4, 3, 1, 8).collect::<Vec<usize>>(),
        ((0..5).chain(7..8)).collect::<Vec<usize>>()
    );
    // Start wraps all the way around
    assert_eq!(
        expanded_and_wrapped_usize_range(2..4, 5, 1, 8).collect::<Vec<usize>>(),
        (0..8).collect::<Vec<usize>>()
    );
    // Start wraps all the way around and past 0
    assert_eq!(
        expanded_and_wrapped_usize_range(2..4, 10, 1, 8).collect::<Vec<usize>>(),
        (0..8).collect::<Vec<usize>>()
    );
    // End wraps
    assert_eq!(
        expanded_and_wrapped_usize_range(3..5, 1, 4, 8).collect::<Vec<usize>>(),
        (0..1).chain(2..8).collect::<Vec<usize>>()
    );
    // End wraps all the way around
    assert_eq!(
        expanded_and_wrapped_usize_range(3..5, 1, 5, 8).collect::<Vec<usize>>(),
        (0..8).collect::<Vec<usize>>()
    );
    // End wraps all the way around and past the end
    assert_eq!(
        expanded_and_wrapped_usize_range(3..5, 1, 10, 8).collect::<Vec<usize>>(),
        (0..8).collect::<Vec<usize>>()
    );
    // Both start and end wrap
    assert_eq!(
        expanded_and_wrapped_usize_range(3..5, 4, 4, 8).collect::<Vec<usize>>(),
        (0..8).collect::<Vec<usize>>()
    );
}

#[test]
fn test_wrapped_usize_outward_from() {
    // No wrapping
    assert_eq!(
        wrapped_usize_outward_from(4, 2, 2, 10).collect::<Vec<usize>>(),
        vec![4, 5, 3, 6, 2]
    );
    // Wrapping at end
    assert_eq!(
        wrapped_usize_outward_from(8, 2, 3, 10).collect::<Vec<usize>>(),
        vec![8, 9, 7, 0, 6, 1]
    );
    // Wrapping at start
    assert_eq!(
        wrapped_usize_outward_from(1, 3, 2, 10).collect::<Vec<usize>>(),
        vec![1, 2, 0, 3, 9, 8]
    );
    // All values wrap around
    assert_eq!(
        wrapped_usize_outward_from(5, 10, 10, 8).collect::<Vec<usize>>(),
        vec![5, 6, 4, 7, 3, 0, 2, 1]
    );
    // None before / after
    assert_eq!(
        wrapped_usize_outward_from(3, 0, 0, 8).collect::<Vec<usize>>(),
        vec![3]
    );
    // Starting point already wrapped
    assert_eq!(
        wrapped_usize_outward_from(15, 2, 2, 10).collect::<Vec<usize>>(),
        vec![5, 6, 4, 7, 3]
    );
    // wrap_length of 0
    assert_eq!(
        wrapped_usize_outward_from(4, 2, 2, 0).collect::<Vec<usize>>(),
        Vec::<usize>::new()
    );
}

#[test]
fn test_split_with_ranges() {
    let input = "hi";
    let result = split_str_with_ranges(input, &|c| c == ' ');

    assert_eq!(result.len(), 1);
    assert_eq!(result[0], (0..2, "hi"));

    let input = "héllo🦀world";
    let result = split_str_with_ranges(input, &|c| c == '🦀');

    assert_eq!(result.len(), 2);
    assert_eq!(result[0], (0..6, "héllo")); // 'é' is 2 bytes
    assert_eq!(result[1], (10..15, "world")); // '🦀' is 4 bytes
}

#[test]
fn test_merge_json_values() {
    use serde_json::json;

    let mut target = json!({
        "unchanged": 1,
        "replaced_scalar": "old",
        "replaced_array": ["default-1", "default-2"],
        "nulled": true,
        "array_becomes_object": [1, 2],
        "object_becomes_scalar": { "x": 1 },
        "nested": {
            "kept": true,
            "overridden": 2,
            "args": ["--default"],
            "deeper": { "list": [1, 2], "other": "kept" },
        },
    });
    let source = json!({
        "replaced_scalar": "new",
        "replaced_array": ["default-2", "user"],
        "nulled": null,
        "array_becomes_object": { "y": 2 },
        "object_becomes_scalar": 3,
        "inserted": ["brand-new"],
        "nested": {
            "overridden": 20,
            "args": ["--user"],
            "deeper": { "list": [3] },
            "inserted": { "z": true },
        },
    });

    merge_json_value_into(source, &mut target);

    assert_eq!(
        target,
        json!({
            "unchanged": 1,
            "replaced_scalar": "new",
            "replaced_array": ["default-2", "user"],
            "nulled": null,
            "array_becomes_object": { "y": 2 },
            "object_becomes_scalar": 3,
            "inserted": ["brand-new"],
            "nested": {
                "kept": true,
                "overridden": 20,
                "args": ["--user"],
                "deeper": { "list": [3], "other": "kept" },
                "inserted": { "z": true },
            },
        })
    );
}

#[test]
fn test_union_json_values() {
    use serde_json::json;

    let mut target = json!({
        "unchanged": 1,
        "replaced_scalar": "old",
        "unioned_array": ["shared", "first"],
        "nested": {
            "kept": true,
            "plugins": [{ "name": "first-plugin" }],
            "scalar": 2,
        },
    });
    let source = json!({
        "replaced_scalar": "new",
        "unioned_array": ["shared", "second"],
        "inserted": ["brand-new"],
        "nested": {
            "plugins": [{ "name": "second-plugin" }],
            "scalar": 20,
        },
    });

    union_json_value_into(source, &mut target);

    assert_eq!(
        target,
        json!({
            "unchanged": 1,
            "replaced_scalar": "new",
            "unioned_array": ["shared", "first", "second"],
            "inserted": ["brand-new"],
            "nested": {
                "kept": true,
                "plugins": [{ "name": "first-plugin" }, { "name": "second-plugin" }],
                "scalar": 20,
            },
        })
    );
}
