use crate::parser::parse;

mod generated {
    mod invalid;
}

mod formatter;
mod toml11;

#[test]
fn cached_parse_matches_and_shares_across_reparses() {
    use crate::parser::parse_with_cache;
    use rowan::{NodeCache, NodeOrToken};
    use std::collections::HashSet;

    let before = "[package]\nname = \"a\"\nversion = \"1\"\n";
    let after = "[package]\nname = \"b\"\nversion = \"1\"\n";
    let mut cache = NodeCache::default();
    let first = parse_with_cache(before, &mut cache);
    let second = parse_with_cache(after, &mut cache);
    let fresh = parse(after);
    assert_eq!(second.green_node, fresh.green_node);
    assert_eq!(second.clone().into_syntax().to_string(), after);
    assert!(second.errors.is_empty() && fresh.errors.is_empty());

    let tokens = |p: &crate::parser::Parse| {
        p.clone()
            .into_syntax()
            .descendants_with_tokens()
            .filter_map(NodeOrToken::into_token)
            .map(|t| t.green() as *const _ as usize)
            .collect::<HashSet<_>>()
    };
    let shared = tokens(&first).intersection(&tokens(&second)).count();
    assert!(shared > 0);
    assert!(tokens(&first).is_disjoint(&tokens(&fresh)));
}

#[test]
fn time_in_arrays() {
    let src = r#"
    a = [00:00:01, 02:03:04]
    "#;

    let errors = parse(src).errors;

    assert!(errors.is_empty(), "{:#?}", errors);
}

#[test]
fn comments_after_tables() {
    let src = r#"
[[array]] # foo
[table] # foo
"#;
    let errors = parse(src).errors;

    assert!(errors.is_empty(), "{:#?}", errors);
}

#[test]
fn dates_in_table_keys() {
    let src = r#"
[2024-01-01]
2024-01-01 = true

[[2024-01-02]]
2024-01-01 = true
"#;
    let errors = parse(src).errors;

    assert!(errors.is_empty(), "{:#?}", errors);
}

#[test]
fn inline_table_with_linebreaks_and_trailing_comma() {
    let src = r#"
cooldowns = { 
    foo = "foo",
    bar = "bar",
}
"#;
    // TOML 1.1 syntax, so it parses only when 1.1 is asked for.
    let errors = crate::parser::parse_with(src, crate::parser::TomlVersion::V1_1, None).errors;

    assert!(errors.is_empty(), "{:#?}", errors);
}

#[test]
fn get_matches_table_key() {
    let src = r#"
name1 = "v"
other = "v2"
"#;

    let root = parse(src).into_dom();

    let matched: Vec<String> = root
        .get_matches("na*")
        .unwrap()
        .map(|(k, _)| match k {
            crate::dom::KeyOrIndex::Key(key) => key.value().to_string(),
            crate::dom::KeyOrIndex::Index(idx) => idx.to_string(),
        })
        .collect();

    assert_eq!(matched, vec!["name1"]);
}

#[test]
fn datetime_with_leading_zeros() {
    let src = r#"
token_expires_at = 0001-01-01T00:00:00Z
"#;

    let errors = parse(src).errors;

    assert!(errors.is_empty(), "{:#?}", errors);
}
