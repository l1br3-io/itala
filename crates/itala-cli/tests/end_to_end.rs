//! End-to-end integration tests for the `itala` CLI binary.

use assert_cmd::Command;
use predicates::prelude::*;
use tokio::io::AsyncBufReadExt as _;

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

/// Copy `tests/fixtures/sample-vault/` into a fresh `TempDir` and return it.
///
/// The caller must keep the returned `TempDir` alive for the test's duration.
fn build_sample_vault() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample-vault");
    copy_dir_recursive(&src, dir.path());
    // Sanity-check: the CRLF fixture must not have been git-normalized to LF.
    let crlf = dir.path().join("05-crlf.md");
    let bytes = std::fs::read(&crlf).unwrap();
    assert!(
        bytes.windows(2).any(|w| w == b"\r\n"),
        "05-crlf.md was normalized by git; check .gitattributes"
    );
    dir
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) {
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let dst_path = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            std::fs::create_dir_all(&dst_path).unwrap();
            copy_dir_recursive(&entry.path(), &dst_path);
        } else {
            std::fs::copy(entry.path(), &dst_path).unwrap();
        }
    }
}

// ---------------------------------------------------------------------------
// Step 1 — smoke binary
// ---------------------------------------------------------------------------

#[test]
fn binary_runs_with_help() {
    Command::cargo_bin("itala")
        .unwrap()
        .arg("--help")
        .assert()
        .success();
}

// ---------------------------------------------------------------------------
// Step 2 — clap structure
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Step 3 — vault open
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Step 4 — note list
// ---------------------------------------------------------------------------

#[test]
fn note_list_outputs_id_and_optional_path_per_note_sorted_by_id() {
    let dir = build_sample_vault();
    let output = Command::cargo_bin("itala")
        .unwrap()
        .args(["note", "list", "--vault"])
        .arg(dir.path())
        .assert()
        .success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    // fixtures/sample-vault has 5 notes (including 03-nested/note.md)
    assert_eq!(lines.len(), 5, "expected 5 lines, got: {stdout:?}");
    let ids: Vec<&str> = lines
        .iter()
        .map(|line| {
            let id = line.split('\t').next().unwrap();
            assert!(
                id.starts_with("note_"),
                "id must start with note_, got: {id}"
            );
            id
        })
        .collect();
    // Output is sorted by NoteId (stable ULID-based identifier).
    for window in ids.windows(2) {
        assert!(
            window[0] <= window[1],
            "ids out of order: {} > {}",
            window[0],
            window[1]
        );
    }
}

// ---------------------------------------------------------------------------
// Step 5 — note read
// ---------------------------------------------------------------------------

/// Helper: run `note list` on a single-note vault and return the note's ID.
fn sole_note_id(vault_dir: &std::path::Path) -> String {
    let output = Command::cargo_bin("itala")
        .unwrap()
        .args(["note", "list", "--vault"])
        .arg(vault_dir)
        .assert()
        .success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    stdout
        .lines()
        .next()
        .unwrap()
        .split('\t')
        .next()
        .unwrap()
        .to_owned()
}

#[test]
fn read_with_frontmatter_prints_yaml_separator_body() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("note.md"),
        b"---\ntitle: Hello\n---\nbody line\n",
    )
    .unwrap();
    let id = sole_note_id(dir.path());
    Command::cargo_bin("itala")
        .unwrap()
        .args(["note", "read", "--vault"])
        .arg(dir.path())
        .arg(&id)
        .assert()
        .success()
        .stdout(predicate::str::contains("title:"))
        .stdout(predicate::str::contains("Hello"))
        .stdout(predicate::str::contains("---\n"))
        .stdout(predicate::str::contains("body line\n"));
}

#[test]
fn read_note_without_explicit_frontmatter_backfills_id_and_emits_body() {
    // F5.5: a note with no frontmatter is backfilled with `id:` on first
    // access, so the CLI output now includes that field before the separator.
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("bare.md"), b"# Just a heading\nbody here\n").unwrap();
    let id = sole_note_id(dir.path());
    let output = Command::cargo_bin("itala")
        .unwrap()
        .args(["note", "read", "--vault"])
        .arg(dir.path())
        .arg(&id)
        .assert()
        .success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    // The backfilled id field is present.
    assert!(
        stdout.contains("id:"),
        "expected backfilled `id:` in output, got: {stdout:?}"
    );
    // The separator is present before the body.
    assert!(
        stdout.contains("---\n"),
        "expected `---` separator in output, got: {stdout:?}"
    );
    assert!(
        stdout.contains("# Just a heading"),
        "expected body in output"
    );
}

#[test]
fn read_preserves_crlf_in_body() {
    let dir = tempfile::tempdir().unwrap();
    // Write a note with CRLF in both frontmatter and body.
    std::fs::write(
        dir.path().join("crlf.md"),
        b"---\r\ntitle: CrLf\r\n---\r\nbody\r\nline2\r\n",
    )
    .unwrap();
    let id = sole_note_id(dir.path());
    let output = Command::cargo_bin("itala")
        .unwrap()
        .args(["note", "read", "--vault"])
        .arg(dir.path())
        .arg(&id)
        .assert()
        .success();
    let bytes = &output.get_output().stdout;
    assert!(
        bytes.windows(2).any(|w| w == b"\r\n"),
        "expected CRLF (\\r\\n) in output"
    );
}

#[test]
fn read_bad_id_fails_before_io_with_nonzero_exit() {
    // Use /tmp as vault — ID parse should fail before any I/O.
    Command::cargo_bin("itala")
        .unwrap()
        .args(["note", "read", "--vault", "/tmp", "not-a-note-id"])
        .assert()
        .failure()
        .code(1);
}

// ---------------------------------------------------------------------------
// Step 6 — note watch
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn note_watch_emits_line_on_file_change() {
    let dir = build_sample_vault();
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_itala"))
        .args(["note", "watch", "--vault"])
        .arg(dir.path())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = tokio::io::BufReader::new(stdout).lines();
    // Give the watcher time to arm before writing.
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    tokio::fs::write(dir.path().join("touch.md"), b"---\ntitle: t\n---\nhi\n")
        .await
        .unwrap();
    let line = tokio::time::timeout(std::time::Duration::from_secs(5), reader.next_line())
        .await
        .expect("watch emitted a line within 5s")
        .unwrap()
        .unwrap();
    assert!(
        line.contains("touch.md"),
        "expected touch.md in event line, got: {line}"
    );
    child.start_kill().unwrap();
    let _ = child.wait().await;
}

// ---------------------------------------------------------------------------
// Step 8 — error formatting
// ---------------------------------------------------------------------------

#[test]
fn error_output_is_single_line_no_backtrace() {
    let output = Command::cargo_bin("itala")
        .unwrap()
        .args(["vault", "open", "/definitely/missing/zzz_itala_test"])
        .assert()
        .failure()
        .code(1);
    let stderr = std::str::from_utf8(&output.get_output().stderr).unwrap();
    assert_eq!(
        stderr.lines().count(),
        1,
        "expected single-line error, got: {stderr:?}"
    );
    assert!(
        !stderr.contains("backtrace"),
        "stderr must not contain backtrace"
    );
}
