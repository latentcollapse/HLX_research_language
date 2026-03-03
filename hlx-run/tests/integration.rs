//! Integration tests for hlx-run binary.
//!
//! Run with: cargo test -p hlx-run --test integration
//! Requires the binary to be built first: cargo build -p hlx-run --release

use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write;
use tempfile::NamedTempFile;

#[allow(deprecated)]
fn hlx_run() -> Command {
    Command::cargo_bin("hlx-run").unwrap()
}

fn write_hlx(src: &str) -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(src.as_bytes()).unwrap();
    f
}

// ── Basic execution ────────────────────────────────────────────────────────

#[test]
fn test_hello_world() {
    let f = write_hlx(
        r#"
fn main() {
    println("hello from hlx");
}
"#,
    );
    hlx_run()
        .arg(f.path())
        .arg("--no-verify")
        .assert()
        .success()
        .stdout(predicate::str::contains("hello from hlx"));
}

#[test]
fn test_function_call_with_arg() {
    let f = write_hlx(
        r#"
fn greet(name) {
    return name;
}
"#,
    );
    hlx_run()
        .arg(f.path())
        .arg("--no-verify")
        .arg("--func")
        .arg("greet")
        .arg("World")
        .assert()
        .success()
        .stdout(predicate::str::contains("World"));
}

#[test]
fn test_arithmetic() {
    let f = write_hlx(
        r#"
fn add(a, b) {
    return a + b;
}
"#,
    );
    hlx_run()
        .arg(f.path())
        .arg("--no-verify")
        .arg("--func")
        .arg("add")
        .arg("3")
        .arg("4")
        .assert()
        .success()
        .stdout(predicate::str::contains("7"));
}

// ── Switch/match ───────────────────────────────────────────────────────────

#[test]
fn test_switch_int() {
    let f = write_hlx(
        r#"
fn classify(x) {
    switch x {
        case 1 => { return "one"; }
        case 2 => { return "two"; }
        default => { return "other"; }
    }
}
"#,
    );
    hlx_run()
        .arg(f.path())
        .arg("--no-verify")
        .arg("--func")
        .arg("classify")
        .arg("2")
        .assert()
        .success()
        .stdout(predicate::str::contains("two"));
}

#[test]
fn test_switch_default() {
    let f = write_hlx(
        r#"
fn classify(x) {
    switch x {
        case 1 => { return "one"; }
        default => { return "other"; }
    }
}
"#,
    );
    hlx_run()
        .arg(f.path())
        .arg("--no-verify")
        .arg("--func")
        .arg("classify")
        .arg("99")
        .assert()
        .success()
        .stdout(predicate::str::contains("other"));
}

#[test]
fn test_switch_string() {
    let f = write_hlx(
        r#"
fn check(s) {
    switch s {
        case "hello" => { return "got hello"; }
        case "world" => { return "got world"; }
        default => { return "got other"; }
    }
}
"#,
    );
    hlx_run()
        .arg(f.path())
        .arg("--no-verify")
        .arg("--func")
        .arg("check")
        .arg("hello")
        .assert()
        .success()
        .stdout(predicate::str::contains("got hello"));
}

// ── Control flow ───────────────────────────────────────────────────────────

#[test]
fn test_for_loop() {
    let f = write_hlx(
        r#"
fn sum_array() {
    let items = [1, 2, 3, 4, 5];
    let total = 0;
    for x in items {
        total += x;
    }
    return total;
}
"#,
    );
    hlx_run()
        .arg(f.path())
        .arg("--no-verify")
        .arg("--func")
        .arg("sum_array")
        .assert()
        .success()
        .stdout(predicate::str::contains("15"));
}

#[test]
fn test_compound_assignment() {
    let f = write_hlx(
        r#"
fn counter() {
    let i = 0;
    i += 5;
    i *= 2;
    i -= 1;
    return i;
}
"#,
    );
    // 0 + 5 = 5, 5 * 2 = 10, 10 - 1 = 9
    hlx_run()
        .arg(f.path())
        .arg("--no-verify")
        .arg("--func")
        .arg("counter")
        .assert()
        .success()
        .stdout(predicate::str::contains("9"));
}

#[test]
fn test_lambda_call() {
    // Tests that lambdas can be stored in variables and called directly.
    // NOTE: map/filter/fold as HOF builtins require VM-level implementation (DEBT-030).
    let f = write_hlx(
        r#"
fn double_each() {
    let double = |x| x * 2;
    let a = double(1);
    let b = double(2);
    let c = double(3);
    return a + b + c;
}
"#,
    );
    // 2 + 4 + 6 = 12
    hlx_run()
        .arg(f.path())
        .arg("--no-verify")
        .arg("--func")
        .arg("double_each")
        .assert()
        .success()
        .stdout(predicate::str::contains("12"));
}

// ── Error cases ────────────────────────────────────────────────────────────

#[test]
fn test_parse_error_reported() {
    let f = write_hlx("fn broken( { }");
    hlx_run()
        .arg(f.path())
        .arg("--no-verify")
        .assert()
        .failure()
        .stderr(predicate::str::is_empty().not());
}

#[test]
fn test_nonexistent_function_fails() {
    let f = write_hlx(r#"fn foo() { return 1; }"#);
    hlx_run()
        .arg(f.path())
        .arg("--no-verify")
        .arg("--func")
        .arg("does_not_exist")
        .assert()
        .failure();
}

// ── bit.hlx smoke test (requires Bitsy/bit.hlx in cwd) ────────────────────

#[test]
#[ignore = "requires Bitsy/bit.hlx in working directory"]
fn test_bit_hlx_repl_step() {
    Command::cargo_bin("hlx-run")
        .unwrap()
        .current_dir("/mnt/d/kilo-workspace/HLXExperimental")
        .arg("Bitsy/bit.hlx")
        .arg("--func")
        .arg("repl_step")
        .arg("Hello Bit")
        .assert()
        .success()
        .stdout(predicate::str::contains("Bit"));
}

#[test]
#[ignore = "requires Bitsy/bit.hlx in working directory"]
fn test_bit_hlx_get_status() {
    Command::cargo_bin("hlx-run")
        .unwrap()
        .current_dir("/mnt/d/kilo-workspace/HLXExperimental")
        .arg("Bitsy/bit.hlx")
        .arg("--func")
        .arg("get_status")
        .assert()
        .success()
        .stdout(predicate::str::contains("level="));
}

#[test]
fn test_migrate_keyword() {
    let f = write_hlx(
        r#"
fn main() {
    println("before");
    migrate worker to cluster_b;
    println("after");
}
"#,
    );
    hlx_run()
        .arg(f.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("before"))
        .stdout(predicate::str::contains("after"));
}
