use loquitor::shell::{
    hook_content, insert_hook, is_hook_present, strip_hook, HOOK_END, HOOK_START,
};

#[test]
fn test_hook_content_contains_markers() {
    let content = hook_content("/tmp/lanes");
    assert!(content.contains(HOOK_START));
    assert!(content.contains(HOOK_END));
    assert!(content.contains("/tmp/lanes"));
    assert!(content.contains("__loquitor_hook"));
    assert!(content.contains("alias claude="));
    assert!(content.contains("script -q"));
}

#[test]
fn test_is_hook_present_detects_installed() {
    let zshrc_with_hook = format!(
        "export PATH=/foo\n{}\n# hook body\n{}\nexport BAR=baz",
        HOOK_START, HOOK_END
    );
    assert!(is_hook_present(&zshrc_with_hook));

    let zshrc_without = "export PATH=/foo\nexport BAR=baz";
    assert!(!is_hook_present(zshrc_without));
}

#[test]
fn test_strip_hook_removes_block() {
    let input = format!(
        "export PATH=foo\n{}\nhook stuff\n{}\nexport BAR=baz",
        HOOK_START, HOOK_END
    );
    let result = strip_hook(&input);
    assert!(result.contains("export PATH=foo"));
    assert!(result.contains("export BAR=baz"));
    assert!(!result.contains("hook stuff"));
    assert!(!result.contains(HOOK_START));
    assert!(!result.contains(HOOK_END));
}

#[test]
fn test_strip_hook_on_empty_content() {
    let result = strip_hook("");
    assert_eq!(result, "");
}

#[test]
fn test_strip_hook_with_no_hook() {
    let input = "export PATH=foo\nexport BAR=baz\n";
    let result = strip_hook(input);
    assert_eq!(result, "export PATH=foo\nexport BAR=baz\n");
}

#[test]
fn test_insert_hook_adds_block() {
    let existing = "export PATH=/foo\nexport BAR=baz\n";
    let result = insert_hook(existing, "/tmp/lanes");
    assert!(result.contains("export PATH=/foo"));
    assert!(result.contains("export BAR=baz"));
    assert!(result.contains(HOOK_START));
    assert!(result.contains(HOOK_END));
    assert!(result.contains("/tmp/lanes"));
}

#[test]
fn test_insert_hook_is_idempotent() {
    // Installing twice should produce the same result as installing once
    let existing = "export PATH=/foo\n";
    let once = insert_hook(existing, "/tmp/lanes");
    let twice = insert_hook(&once, "/tmp/lanes");

    // Should have exactly one HOOK_START
    let start_count = twice.matches(HOOK_START).count();
    assert_eq!(
        start_count, 1,
        "Hook should appear exactly once after double install"
    );
}

#[test]
fn test_insert_hook_on_empty_content() {
    let result = insert_hook("", "/tmp/lanes");
    assert!(result.contains(HOOK_START));
    assert!(result.contains(HOOK_END));
}

#[test]
fn test_install_then_strip_returns_to_original() {
    let original = "export PATH=/foo\nexport BAR=baz\n";
    let with_hook = insert_hook(original, "/tmp/lanes");
    let back_to_original = strip_hook(&with_hook);
    assert_eq!(back_to_original, original);
}
