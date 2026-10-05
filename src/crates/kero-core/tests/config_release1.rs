use kero_core::config::{Newline, TriviaKind, default_config, parse, write_canonical};

#[test]
fn parses_repository_mount_refresh_template() {
    let document = parse(default_config()).unwrap();
    assert!(document.nodes.is_empty());
    assert!(document.trivia.len() >= 2);
    assert!(document
        .trivia
        .iter()
        .any(|trivia| matches!(&trivia.kind, TriviaKind::Comment(text) if text.contains("Per-mount refresh override"))));
}

#[test]
fn preserves_comments_blank_lines_crlf_spans_and_duplicates() {
    let document =
        parse("# comment\r\n\r\npolicy enabled\r\n\tentry \"first value\"\r\n\tentry second\r\n")
            .unwrap();
    assert_eq!(document.newline, Newline::Crlf);
    assert_eq!(
        document.trivia[0].kind,
        TriviaKind::Comment(" comment".into())
    );
    assert_eq!(document.trivia[1].kind, TriviaKind::Blank);
    assert_eq!(document.nodes[0].location.line, 3);
    assert_eq!(document.nodes[0].children.len(), 2);
}

#[test]
fn canonical_writer_is_deterministic_and_escapes_values() {
    let document = parse("policy enabled\n\tentry \"a \\\"quote\\\"\\n\"\n").unwrap();
    assert_eq!(
        write_canonical(&document),
        "policy enabled\n\tentry \"a \\\"quote\\\"\\n\"\n"
    );
}

#[test]
fn reports_only_lexical_and_structural_errors() {
    for (input, code) in [
        (" policy enabled\n", "config.lexical.space-indent"),
        (
            "policy enabled\n\t\tentry yes\n",
            "config.structure.indent-jump",
        ),
        ("policy \"unterminated\n", "config.lexical.quote"),
        ("policy \"ok\" trailing\n", "config.lexical.trailing"),
        ("policy \"bad\\q\"\n", "config.lexical.escape"),
    ] {
        assert_eq!(parse(input).unwrap_err().code, code, "{input}");
    }
}
