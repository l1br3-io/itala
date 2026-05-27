//! End-to-end integration tests for the `itala` CLI binary.

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn binary_runs_with_help() {
    Command::cargo_bin("itala").unwrap().arg("--help").assert().success();
}

#[test]
fn help_lists_vault_and_note_subcommands() {
    Command::cargo_bin("itala")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("vault"))
        .stdout(predicate::str::contains("note"));
}

#[test]
fn note_subcommand_requires_action() {
    Command::cargo_bin("itala")
        .unwrap()
        .arg("note")
        .assert()
        .failure();
}

#[test]
fn vault_open_prints_canonical_path() {
    let dir = tempfile::tempdir().unwrap();
    let canonical = dir.path().canonicalize().unwrap();
    let expected = canonical.to_str().unwrap().to_owned();
    Command::cargo_bin("itala")
        .unwrap()
        .args(["vault", "open"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(format!("opened vault {expected}")));
}

#[test]
fn vault_open_missing_path_exits_nonzero() {
    Command::cargo_bin("itala")
        .unwrap()
        .args(["vault", "open", "/definitely/missing/zzz_itala_test"])
        .assert()
        .failure()
        .code(1);
}
