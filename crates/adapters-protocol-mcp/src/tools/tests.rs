use super::*;

#[test]
fn load_server_configs_merges_dirs_with_project_wins() {
    let user = tempfile::tempdir().expect("tempdir");
    let project = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        user.path().join("user.json"),
        r#"{"servers": [
            {"name": "a", "command": "user-a"},
            {"name": "b", "command": "user-b"}
        ]}"#,
    )
    .expect("write");
    std::fs::write(
        project.path().join("project.json"),
        r#"{"servers": [{"name": "a", "command": "project-a"}]}"#,
    )
    .expect("write");
    let (configs, problems) = load_server_configs(&[user.path().into(), project.path().into()]);
    assert!(problems.is_empty());
    assert_eq!(configs.len(), 2);
    let a = configs.iter().find(|config| config.name == "a").expect("a");
    assert_eq!(a.command, "project-a");
    let b = configs.iter().find(|config| config.name == "b").expect("b");
    assert_eq!(b.command, "user-b");
}

#[test]
fn load_server_configs_reports_problems() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("bad.json"), "{not json").expect("write");
    std::fs::write(
        dir.path().join("slash.json"),
        r#"{"servers": [{"name": "a/b", "command": "x"}]}"#,
    )
    .expect("write");
    std::fs::write(
        dir.path().join("empty.json"),
        r#"{"servers": [{"name": "c"}]}"#,
    )
    .expect("write");
    let (configs, problems) = load_server_configs(&[dir.path().into()]);
    assert!(configs.is_empty());
    assert_eq!(problems.len(), 3);
}

#[test]
fn cap_description_truncates() {
    let long = "a".repeat(600);
    let capped = cap_description(&long);
    assert_eq!(capped.chars().count(), 501);
    assert!(capped.ends_with('…'));
}

#[test]
fn parse_name_rejects_malformed_names() {
    assert_eq!(
        parse_name("mcp/fs/read_file"),
        Some(("fs".into(), "read_file".into()))
    );
    assert_eq!(parse_name("read_file"), None);
    assert_eq!(parse_name("mcp/"), None);
    assert_eq!(parse_name("mcp/fs/"), None);
}
