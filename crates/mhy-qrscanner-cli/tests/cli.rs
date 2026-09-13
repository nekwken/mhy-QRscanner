use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

fn create_cmd(data_dir: &str, extra: &[&str]) -> Command {
    let mut cmd = Command::cargo_bin("mhyqr").unwrap();
    cmd.args([
        "--data-dir",
        data_dir,
        "device",
        "create",
        "--account",
        "acct-1",
    ]);
    cmd.args(extra);
    cmd
}

#[test]
fn create_refuses_overwrite_without_force() {
    let dir = tempdir().unwrap();
    let data_dir = dir.path().to_str().unwrap();

    create_cmd(data_dir, &[]).assert().success();
    create_cmd(data_dir, &[])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
    create_cmd(data_dir, &["--force"]).assert().success();
}

#[test]
fn create_and_show_device_profile() {
    let dir = tempdir().unwrap();
    let data_dir = dir.path().to_str().unwrap();

    Command::cargo_bin("mhyqr")
        .unwrap()
        .args([
            "--data-dir",
            data_dir,
            "device",
            "create",
            "--account",
            "acct-1",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("created device profile"));

    Command::cargo_bin("mhyqr")
        .unwrap()
        .args([
            "--data-dir",
            data_dir,
            "device",
            "show",
            "--account",
            "acct-1",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("device_id="))
        .stdout(predicate::str::contains("device_fp="));
}

#[test]
fn create_uses_selected_template() {
    let dir = tempdir().unwrap();
    let data_dir = dir.path().to_str().unwrap();

    create_cmd(data_dir, &["--template", "samsung-s24"])
        .assert()
        .success()
        .stdout(predicate::str::contains("template=samsung-s24"))
        .stdout(predicate::str::contains("model=SM-S9280"));
}

#[test]
fn create_rejects_unknown_template() {
    let dir = tempdir().unwrap();
    let data_dir = dir.path().to_str().unwrap();

    create_cmd(data_dir, &["--template", "nope"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown template"));
}

#[test]
fn login_game_requires_a_qr_source() {
    let dir = tempdir().unwrap();
    let data_dir = dir.path().to_str().unwrap();
    create_cmd(data_dir, &[]).assert().success();

    Command::cargo_bin("mhyqr")
        .unwrap()
        .args([
            "--data-dir",
            data_dir,
            "qr",
            "login-game",
            "--account",
            "acct-1",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--url"))
        .stderr(predicate::str::contains("--capture-screen"));
}

#[test]
fn login_game_rejects_conflicting_sources() {
    let dir = tempdir().unwrap();
    let data_dir = dir.path().to_str().unwrap();
    create_cmd(data_dir, &[]).assert().success();

    Command::cargo_bin("mhyqr")
        .unwrap()
        .args([
            "--data-dir",
            data_dir,
            "qr",
            "login-game",
            "--account",
            "acct-1",
            "--image",
            "whatever.png",
            "--capture-screen",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("mutually exclusive"));
}

#[test]
fn login_game_rejects_non_game_url() {
    let dir = tempdir().unwrap();
    let data_dir = dir.path().to_str().unwrap();
    create_cmd(data_dir, &[]).assert().success();

    Command::cargo_bin("mhyqr")
        .unwrap()
        .args([
            "--data-dir",
            data_dir,
            "qr",
            "login-game",
            "--account",
            "acct-1",
            "--url",
            "https://user.mihoyo.com/login-platform/mobile.html?tk=abc#/login/qr",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not a V1 game QR URL"));
}

#[test]
fn cancel_requires_a_session() {
    let dir = tempdir().unwrap();
    let data_dir = dir.path().to_str().unwrap();
    create_cmd(data_dir, &[]).assert().success();

    Command::cargo_bin("mhyqr")
        .unwrap()
        .args([
            "--data-dir",
            data_dir,
            "qr",
            "cancel",
            "--account",
            "acct-1",
            "--url",
            "ticket-only",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no session"));
}

#[test]
fn qr_help_lists_scan_login_game_and_cancel() {
    Command::cargo_bin("mhyqr")
        .unwrap()
        .args(["qr", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("login-game"))
        .stdout(predicate::str::contains("cancel"));
}
