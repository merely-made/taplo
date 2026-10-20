use crate::parser::{parse, parse_with, TomlVersion};
use serde_json::json;

fn v11(src: &str) -> serde_json::Value {
    let p = parse_with(src, TomlVersion::V1_1, None);
    assert!(p.errors.is_empty(), "{src:?}: {:#?}", p.errors);
    assert_eq!(p.clone().into_syntax().to_string(), src);
    let dom = p.into_dom();
    assert!(dom.validate().is_ok(), "{src:?}");
    serde_json::to_value(&dom).unwrap()
}

fn rejected_by_1_0(src: &str) {
    let p = parse(src);
    assert!(
        !p.errors.is_empty() || p.into_dom().validate().is_err(),
        "TOML 1.0 accepted {src:?}"
    );
}

#[test]
fn escape_and_hex_escapes() {
    for (src, value) in [
        ("value = \"\\e\"\n", "\u{1b}"),
        ("value = \"\\x41\"\n", "A"),
        ("value = \"\\xe9\\x00\\xFF\"\n", "\u{e9}\u{0}\u{ff}"),
        ("value = \"\"\"\n\\e[0m\\x7e\"\"\"\n", "\u{1b}[0m~"),
    ] {
        assert_eq!(v11(src), json!({ "value": value }));
        rejected_by_1_0(src);
    }
    for src in [
        "value = \"\\x4\"\n",
        "value = \"\\xZZ\"\n",
        "value = \"\\E\"\n",
    ] {
        assert!(!parse_with(src, TomlVersion::V1_1, None).errors.is_empty());
    }
    // Literal strings never had escapes.
    assert_eq!(v11("value = '\\e\\x41'\n"), json!({ "value": "\\e\\x41" }));
}

#[test]
fn multiline_inline_tables_with_comments_and_trailing_comma() {
    let src = "inline = {\n a = 1, # TOML 1.1\n b = { c = [1, 2], },\n\n}\n";
    assert_eq!(
        v11(src),
        json!({ "inline": { "a": 1, "b": { "c": [1, 2] } } })
    );
    rejected_by_1_0(src);
    rejected_by_1_0("abc = { abc = 123, }\n");
    assert_eq!(
        v11("abc = { abc = 123, }\n"),
        json!({ "abc": { "abc": 123 } })
    );
    assert_eq!(v11("empty = {\n}\n"), json!({ "empty": {} }));
    for src in [
        "t = { a = 1\n b = 2 }\n",
        "t = { , }\n",
        "t = { a = 1,, }\n",
        "t = {\n a = 1,\n",
    ] {
        let p = parse_with(src, TomlVersion::V1_1, None);
        assert!(
            !p.errors.is_empty() || p.into_dom().validate().is_err(),
            "{src:?}"
        );
    }
}

#[test]
fn times_without_seconds() {
    for (src, value) in [
        ("t = 07:32\n", "07:32:00"),
        ("t = 1979-05-27T07:32\n", "1979-05-27T07:32:00"),
        ("t = 1979-05-27 07:32\n", "1979-05-27T07:32:00"),
        ("t = 1979-05-27T07:32Z\n", "1979-05-27T07:32:00Z"),
        ("t = 1979-05-27T07:32-07:00\n", "1979-05-27T07:32:00-07:00"),
    ] {
        assert_eq!(v11(src), json!({ "t": value }), "{src:?}");
        rejected_by_1_0(src);
    }
    // Seconds stay valid in both versions; fractions still need seconds.
    assert_eq!(v11("t = 07:32:01\n"), json!({ "t": "07:32:01" }));
    assert!(parse("t = 07:32:01\n").errors.is_empty());
    assert!(!parse_with("t = 07:32.5\n", TomlVersion::V1_1, None)
        .errors
        .is_empty());
}

#[test]
fn version_1_0_is_the_default() {
    assert_eq!(TomlVersion::default(), TomlVersion::V1_0);
    let src = "a = \"\\e\"\n";
    assert_eq!(
        parse(src).errors,
        parse_with(src, TomlVersion::V1_0, None).errors
    );
}

fn format11(src: &str, options: crate::formatter::Options) -> String {
    let syntax = parse_with(src, TomlVersion::V1_1, None).into_syntax();
    crate::formatter::format_syntax(syntax, options)
}

// A multiline inline table keeps one entry or comment per line, so every comment stays put.
#[test]
fn formatting_lays_out_multiline_inline_tables() {
    for (src, expected) in [
        (
            "inline = {\n a = 1, # one\n   # standalone\n\n\n\n b = 2 # two\n} # after\n",
            "inline = {\n  a = 1, # one\n  # standalone\n\n\n  b = 2, # two\n} # after\n",
        ),
        (
            "inline = { # open\n a = 1\n}\n",
            "inline = { # open\n  a = 1,\n}\n",
        ),
        (
            "outer = {\n inner = {\n x = 1, # deep\n },\n flat = { y = 2 },\n}\n",
            "outer = {\n  inner = {\n    x = 1, # deep\n  },\n  flat = { y = 2 },\n}\n",
        ),
        ("empty = {\n\n}\n", "empty = {}\n"),
        ("single = { a = 1, b = 2 }\n", "single = { a = 1, b = 2 }\n"),
    ] {
        let formatted = format11(src, Default::default());
        assert_eq!(formatted, expected, "{src:?}");
        assert_eq!(v11(&formatted), v11(src), "{src:?}");
        assert_eq!(
            format11(&formatted, Default::default()),
            formatted,
            "{src:?}"
        );
    }
}

#[test]
fn multiline_inline_tables_follow_comma_and_newline_options() {
    let src = "t = {\n a = 1,\n b = 2, # b\n}\n";
    let options = crate::formatter::Options {
        array_trailing_comma: false,
        crlf: true,
        ..Default::default()
    };
    assert_eq!(
        format11(src, options),
        "t = {\r\n  a = 1,\r\n  b = 2 # b\r\n}\r\n"
    );
}
