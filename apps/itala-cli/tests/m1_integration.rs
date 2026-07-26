//! M1 end-to-end integration test.
//!
//! Exercises every public CLI surface added in M1 as a single coherent
//! narrative:
//!
//! 1. `init` a fresh vault in a tempdir.
//! 2. Create three notes, two of which link to the third.
//! 3. `notes list` shows all three.
//! 4. `notes show` prints one note's Markdown.
//! 5. `notes move` the linked note → verify inbound links were rewritten.
//! 6. `history list` + `history show` round-trip the snapshot body.
//! 7. `notes delete` one referrer; list reflects the change.
//! 8. Re-list the vault from a fresh process — persistence is implicit since
//!    every CLI invocation starts a new process.

use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

fn cli() -> Command {
    Command::cargo_bin("itala").expect("itala binary built")
}

/// Extract the `Notes/<slug>-<ulid>.md` path from a `created:` output line.
fn extract_created_path(stdout: &[u8]) -> String {
    let s = String::from_utf8(stdout.to_owned()).expect("utf8");
    let line = s.lines().next().expect("first line");
    line.strip_prefix("created: ")
        .expect("created: prefix")
        .split_whitespace()
        .next()
        .expect("path token")
        .to_owned()
}

/// Extract the ULID from a note path's `Notes/<slug>-<ulid>.md` form.
fn ulid_from_path(path: &str) -> &str {
    path.strip_suffix(".md")
        .unwrap_or(path)
        .rsplit('-')
        .next()
        .unwrap_or("")
}

#[test]
#[allow(clippy::too_many_lines)]
fn m1_full_flow() {
    let tmp = TempDir::new().expect("tempdir");

    // 1. Init the vault.
    cli()
        .arg("init")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(contains("initialized vault"));
    assert!(tmp.path().join(".itala/vault.id").exists());

    // 2. Create three notes; Referrer-A and Referrer-B link to "Target".
    let target_out = cli()
        .arg("notes")
        .arg("create")
        .arg("--vault")
        .arg(tmp.path())
        .arg("--body")
        .arg("the target body")
        .arg("Target")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let target_path = extract_created_path(&target_out);
    let target_ulid = ulid_from_path(&target_path).to_owned();

    let _ = cli()
        .arg("notes")
        .arg("create")
        .arg("--vault")
        .arg(tmp.path())
        .arg("--body")
        .arg("see [[target]] in body")
        .arg("ReferrerA")
        .assert()
        .success();
    let _ = cli()
        .arg("notes")
        .arg("create")
        .arg("--vault")
        .arg(tmp.path())
        .arg("--body")
        .arg("aliased [[target|the target]]")
        .arg("ReferrerB")
        .assert()
        .success();

    // 3. List shows all three.
    let list_out = cli()
        .arg("notes")
        .arg("list")
        .arg("--vault")
        .arg(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let list_out = String::from_utf8(list_out).unwrap();
    assert!(list_out.contains("Notes/target-"));
    assert!(list_out.contains("Notes/referrera-"));
    assert!(list_out.contains("Notes/referrerb-"));
    assert_eq!(list_out.lines().count(), 3);

    // 4. Show the target note.
    cli()
        .arg("notes")
        .arg("show")
        .arg("--vault")
        .arg(tmp.path())
        .arg(&target_path)
        .assert()
        .success()
        .stdout(contains("the target body"));

    // 5. Move Target; both referrers should have inbound links rewritten.
    let dest_ulid = itala_kernel::VaultId::new().to_string();
    let dest_path = format!("Notes/renamed-{dest_ulid}.md");
    let move_out = cli()
        .arg("notes")
        .arg("move")
        .arg("--vault")
        .arg(tmp.path())
        .arg(&target_path)
        .arg(&dest_path)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let move_out = String::from_utf8(move_out).unwrap();
    assert!(
        move_out.contains("rewrote 2 note(s)"),
        "expected 2 rewrites, got: {move_out}"
    );

    // Both referrers should now contain `[[renamed` instead of `[[target`.
    // Find their current paths and show their bodies.
    let list_after_move = cli()
        .arg("notes")
        .arg("list")
        .arg("--vault")
        .arg(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let list_after_move = String::from_utf8(list_after_move).unwrap();
    for line in list_after_move.lines() {
        let path = line.split('\t').nth(1).unwrap();
        if path.contains("Notes/referrera-") {
            let body = cli()
                .arg("notes")
                .arg("show")
                .arg("--vault")
                .arg(tmp.path())
                .arg(path)
                .assert()
                .success()
                .get_output()
                .stdout
                .clone();
            let body = String::from_utf8(body).unwrap();
            assert!(
                body.contains("[[renamed]]"),
                "expected [[renamed]] in body: {body}"
            );
            assert!(!body.contains("[[target]]"));
        }
        if path.contains("Notes/referrerb-") {
            let body = cli()
                .arg("notes")
                .arg("show")
                .arg("--vault")
                .arg(tmp.path())
                .arg(path)
                .assert()
                .success()
                .get_output()
                .stdout
                .clone();
            let body = String::from_utf8(body).unwrap();
            assert!(
                body.contains("[[renamed|the target]]"),
                "expected [[renamed|the target]] in body: {body}"
            );
        }
    }

    // 6. history list <ulid> + history show <id>.
    let history_list_out = cli()
        .arg("history")
        .arg("list")
        .arg("--vault")
        .arg(tmp.path())
        .arg(&target_ulid)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let history_list_out = String::from_utf8(history_list_out).unwrap();
    assert!(
        !history_list_out.contains("(no history)"),
        "expected at least one snapshot, got: {history_list_out}"
    );
    let history_id = history_list_out
        .lines()
        .next()
        .unwrap()
        .split('\t')
        .next()
        .unwrap()
        .to_owned();

    cli()
        .arg("history")
        .arg("show")
        .arg("--vault")
        .arg(tmp.path())
        .arg(&history_id)
        .assert()
        .success()
        .stdout(contains("the target body"));

    // 7. Delete one referrer; list reflects the change.
    let referrer_a_path = list_after_move
        .lines()
        .find(|l| l.contains("Notes/referrera-"))
        .expect("referrer-a present")
        .split('\t')
        .nth(1)
        .unwrap()
        .to_owned();
    cli()
        .arg("notes")
        .arg("delete")
        .arg("--vault")
        .arg(tmp.path())
        .arg("--yes")
        .arg(&referrer_a_path)
        .assert()
        .success();

    let final_list = cli()
        .arg("notes")
        .arg("list")
        .arg("--vault")
        .arg(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let final_list = String::from_utf8(final_list).unwrap();
    assert!(!final_list.contains("Notes/referrera-"));
    assert!(final_list.contains("Notes/referrerb-"));
    assert!(final_list.contains("Notes/renamed-"));
    assert_eq!(final_list.lines().count(), 2);

    // 8. .itala/history/ contains the snapshot files on disk.
    let history_dir = tmp.path().join(".itala/history");
    assert!(history_dir.exists(), "history dir should exist");
    let snapshot_files: Vec<_> = std::fs::read_dir(&history_dir)
        .unwrap()
        .flatten()
        .filter_map(|e| std::fs::read_dir(e.path()).ok())
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        !snapshot_files.is_empty(),
        "expected snapshot files under {history_dir:?}"
    );
}

#[test]
fn m1_persistence_across_invocations() {
    // Each CLI invocation is a fresh process; running two commands back-to-
    // back against the same vault proves on-disk persistence.
    let tmp = TempDir::new().expect("tempdir");
    cli().arg("init").arg(tmp.path()).assert().success();

    cli()
        .arg("notes")
        .arg("create")
        .arg("--vault")
        .arg(tmp.path())
        .arg("--body")
        .arg("persisted body")
        .arg("Persist")
        .assert()
        .success();

    // Second process invocation sees the note from the first.
    let list = cli()
        .arg("notes")
        .arg("list")
        .arg("--vault")
        .arg(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let list = String::from_utf8(list).unwrap();
    assert!(list.contains("Notes/persist-"));

    // And the history snapshot is also on disk.
    let history_dirs: Vec<_> = std::fs::read_dir(tmp.path().join(".itala/history"))
        .unwrap()
        .flatten()
        .collect();
    assert_eq!(history_dirs.len(), 1, "one note dir expected");
}
