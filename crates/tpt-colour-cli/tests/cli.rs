//! Drives the binary the way a user would.

use std::process::Command;

fn run(args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_tpt-colour"))
        .args(args)
        .output()
        .expect("binary runs");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

#[test]
fn info_describes_the_builtin_srgb_profile() {
    let (ok, stdout) = run(&["info", "srgb"]);
    assert!(ok);
    assert!(stdout.contains("matrix/TRC"), "{stdout}");
    assert!(stdout.contains("Perceptual: available"), "{stdout}");
}

#[test]
fn convert_prints_three_values() {
    let (ok, stdout) = run(&["convert", "srgb", "display-p3", "0.8", "0.2", "0.1"]);
    assert!(ok);
    assert_eq!(stdout.trim().matches(',').count(), 2, "{stdout}");
}

#[test]
fn gamut_check_reports_out_of_gamut_p3_green() {
    let (ok, stdout) = run(&["gamut-check", "display-p3", "0", "1", "0"]);
    assert!(ok);
    assert!(stdout.contains("inside sRGB: no"), "{stdout}");
}

#[test]
fn bad_input_exits_with_usage() {
    let (ok, _) = run(&["convert", "srgb", "srgb", "2.0", "0", "0"]);
    assert!(!ok);
    let (ok, _) = run(&[]);
    assert!(!ok);
}
