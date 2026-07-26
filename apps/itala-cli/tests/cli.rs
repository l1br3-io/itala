//! End-to-end CLI tests via `assert_cmd`.
//!
//! These exercise the binary as a user would: invoke `itala` with arguments,
//! check stdout/stderr/exit code. They run against a fresh `tempfile::TempDir`
//! per test so they're independent.

use assert_cmd::Command;
use tempfile::TempDir;

fn cli() -> Command {
    Command::cargo_bin("itala").expect("itala binary built")
}

fn fresh_vault() -> TempDir {
    let tmp = TempDir::new().expect("tempdir");
    cli().arg("init").arg(tmp.path()).assert().success();
    tmp
}

#[test]
fn version_prints() {
    cli()
        .arg("--version")
        .assert()
        .success()
        .stdout("itala 0.1.0\n");
}

#[test]
fn init_creates_vault() {
    let tmp = TempDir::new().expect("tempdir");
    cli()
        .arg("init")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("initialized vault"));
    // The vault file should exist.
    let vault_id_path = tmp.path().join(".itala").join("vault.id");
    assert!(vault_id_path.exists(), "vault.id should exist");
}

#[test]
fn create_then_list() {
    let tmp = fresh_vault();
    cli()
        .arg("notes")
        .arg("create")
        .arg("--vault")
        .arg(tmp.path())
        .arg("--body")
        .arg("first body")
        .arg("Hello World")
        .assert()
        .success()
        .stdout(predicates::str::contains("created: Notes/hello-world-"));
    cli()
        .arg("notes")
        .arg("list")
        .arg("--vault")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("Notes/hello-world-"));
}

#[test]
fn list_on_empty_vault_prints_placeholder() {
    let tmp = fresh_vault();
    cli()
        .arg("notes")
        .arg("list")
        .arg("--vault")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout("(no notes)\n");
}

#[test]
fn create_multiple_then_list() {
    let tmp = fresh_vault();
    for title in ["Alpha", "Beta", "Gamma"] {
        cli()
            .arg("notes")
            .arg("create")
            .arg("--vault")
            .arg(tmp.path())
            .arg("--body")
            .arg("body")
            .arg(title)
            .assert()
            .success();
    }
    let out = cli()
        .arg("notes")
        .arg("list")
        .arg("--vault")
        .arg(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let out = String::from_utf8(out).unwrap();
    assert!(out.contains("Notes/alpha-"));
    assert!(out.contains("Notes/beta-"));
    assert!(out.contains("Notes/gamma-"));
    assert_eq!(out.lines().count(), 3);
}

#[test]
fn show_outputs_markdown() {
    let tmp = fresh_vault();
    let create_out = cli()
        .arg("notes")
        .arg("create")
        .arg("--vault")
        .arg(tmp.path())
        .arg("--body")
        .arg("# Hi\n\nsome body\n")
        .arg("Title")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let create_out = String::from_utf8(create_out).unwrap();
    // "created: Notes/Title-<ulid>.md (id: <ulid>)"
    let path = create_out
        .lines()
        .next()
        .unwrap()
        .strip_prefix("created: ")
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap();

    cli()
        .arg("notes")
        .arg("show")
        .arg("--vault")
        .arg(tmp.path())
        .arg(path)
        .assert()
        .success()
        .stdout(predicates::str::contains("title: Title"))
        .stdout(predicates::str::contains("some body"));
}

#[test]
fn show_unknown_path_errors() {
    let tmp = fresh_vault();
    cli()
        .arg("notes")
        .arg("show")
        .arg("--vault")
        .arg(tmp.path())
        .arg("Notes/nope-01JPTULIDULIDULIDULIDUL.md")
        .assert()
        .failure();
}

#[test]
fn delete_without_yes_fails() {
    let tmp = fresh_vault();
    cli()
        .arg("notes")
        .arg("create")
        .arg("--vault")
        .arg(tmp.path())
        .arg("--body")
        .arg("x")
        .arg("ToDelete")
        .assert()
        .success();
    cli()
        .arg("notes")
        .arg("delete")
        .arg("--vault")
        .arg(tmp.path())
        .arg("Notes/to-delete-01JPTULIDULIDULIDULIDUL.md")
        .assert()
        .failure();
}

#[test]
fn delete_with_yes_removes_note() {
    let tmp = fresh_vault();
    let create_out = cli()
        .arg("notes")
        .arg("create")
        .arg("--vault")
        .arg(tmp.path())
        .arg("--body")
        .arg("x")
        .arg("ToDelete")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let create_out = String::from_utf8(create_out).unwrap();
    let path = create_out
        .lines()
        .next()
        .unwrap()
        .strip_prefix("created: ")
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap();

    cli()
        .arg("notes")
        .arg("delete")
        .arg("--vault")
        .arg(tmp.path())
        .arg("--yes")
        .arg(path)
        .assert()
        .success();

    // Now list should be empty.
    cli()
        .arg("notes")
        .arg("list")
        .arg("--vault")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout("(no notes)\n");
}

#[test]
fn open_on_non_vault_errors() {
    let tmp = TempDir::new().unwrap();
    cli()
        .arg("notes")
        .arg("list")
        .arg("--vault")
        .arg(tmp.path())
        .assert()
        .failure();
}

#[test]
fn snapshot_and_restore_round_trip() {
    let tmp = fresh_vault();
    // Create note with v1.
    let create_out = cli()
        .arg("notes")
        .arg("create")
        .arg("--vault")
        .arg(tmp.path())
        .arg("--body")
        .arg("v1 body")
        .arg("HistoryTest")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let create_out = String::from_utf8(create_out).unwrap();
    let path = create_out
        .lines()
        .next()
        .unwrap()
        .strip_prefix("created: ")
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap();
    let note_ulid = path
        .strip_prefix("Notes/historytest-")
        .unwrap()
        .strip_suffix(".md")
        .unwrap();

    // history list <ulid> shows one entry.
    let list_out = cli()
        .arg("history")
        .arg("list")
        .arg("--vault")
        .arg(tmp.path())
        .arg(note_ulid)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let list_out = String::from_utf8(list_out).unwrap();
    assert!(!list_out.contains("(no history)"), "got: {list_out}");
    let history_id = list_out.lines().next().unwrap().split('\t').next().unwrap();

    // history show <id> prints the snapshot body.
    cli()
        .arg("history")
        .arg("show")
        .arg("--vault")
        .arg(tmp.path())
        .arg(history_id)
        .assert()
        .success()
        .stdout(predicates::str::contains("v1 body"));
}
