use super::*;

fn chat_shell() -> Shell {
    let mut shell = Shell::new(Size::new(1280, 800), LayoutConfig::default(), 120);
    shell.open_chat().unwrap();
    shell
}

#[test]
fn parses_markdown_spans_and_blocks() {
    let mut shell = chat_shell();
    shell.chat_push_token("# Title\n");
    shell.chat_push_token("- item with **bold** and `code`\n");
    shell.chat_push_token("```\n");
    shell.chat_push_token("let x = 1;\n");
    shell.chat_push_token("```\n");
    shell.chat_finish_stream();
    let lines = shell.visible_chat_lines();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0].scale, 2);
    assert!(lines[1]
        .spans
        .iter()
        .any(|span| span.kind == ChatSpanKind::Bold && span.text == "bold"));
    assert!(lines[1]
        .spans
        .iter()
        .any(|span| span.kind == ChatSpanKind::Code && span.text == "code"));
    assert!(lines[2].code);
}

#[test]
fn streams_and_bounds_a_long_conversation() {
    let mut shell = chat_shell();
    for index in 0..1000 {
        shell.chat_push_token(&format!("token-{index} "));
        if index % 12 == 0 {
            shell.chat_push_token("\n");
        }
    }
    shell.chat_finish_stream();
    assert!(shell.chat_line_count() >= 80);
    assert!(shell.visible_chat_lines().len() <= shell.chat_visible_rows());
    shell.chat_page(-1);
    assert!(shell.chat_scroll() < shell.chat_line_count());
    shell.chat_page(1);
    assert!(shell.chat_scroll() > 0);
}

#[test]
fn wraps_long_paragraphs() {
    let mut shell = chat_shell();
    shell.chat_push_token(&"word ".repeat(100));
    shell.chat_push_token("\n");
    shell.chat_finish_stream();
    assert!(shell.chat_line_count() > 3);
    assert!(shell.visible_chat_lines().iter().all(|line| line
        .spans
        .iter()
        .map(|span| span.text.chars().count())
        .sum::<usize>()
        <= 48));
}
