//! Integratsioonitestid: loo → näita → paki lahti → võrdle.

use std::fs;
use std::path::Path;
use std::process::Command;

fn pakk() -> Command {
    Command::new(env!("CARGO_BIN_EXE_pakk"))
}

fn setup(root: &Path) {
    fs::create_dir_all(root.join("andmed/sygav")).unwrap();
    fs::write(root.join("andmed/tekst.txt"), "tere maailm\n".repeat(1000)).unwrap();
    fs::write(root.join("andmed/sygav/arv.bin"), [7u8; 4096]).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink("tekst.txt", root.join("andmed/viide.txt")).unwrap();
}

fn roundtrip(mode: &[&str]) {
    let tmp = tempdir(&format!("rt-{}", mode.join("-").replace("--", "")));
    setup(&tmp);
    let arhiiv = tmp.join("proov.pakk");

    let mut cmd = pakk();
    cmd.arg("c").args(mode).arg(&arhiiv).arg("andmed").current_dir(&tmp);
    assert!(cmd.status().unwrap().success(), "loomine ebaõnnestus: {mode:?}");

    let out = tmp.join("valjund");
    let status = pakk()
        .args(["x", arhiiv.to_str().unwrap(), "-C", out.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(status.success(), "lahtipakkimine ebaõnnestus: {mode:?}");

    let originaal = fs::read(tmp.join("andmed/tekst.txt")).unwrap();
    let taastatud = fs::read(out.join("andmed/tekst.txt")).unwrap();
    assert_eq!(originaal, taastatud);
    assert_eq!(
        fs::read(tmp.join("andmed/sygav/arv.bin")).unwrap(),
        fs::read(out.join("andmed/sygav/arv.bin")).unwrap()
    );
    #[cfg(unix)]
    assert!(out.join("andmed/viide.txt").symlink_metadata().unwrap().is_symlink());
}

#[test]
fn ringreis_vaikimisi() {
    roundtrip(&[]);
}

#[test]
fn ringreis_fast() {
    roundtrip(&["--fast"]);
}

#[test]
fn ringreis_max() {
    roundtrip(&["--max"]);
}

#[test]
fn naitab_sisu() {
    let tmp = tempdir("list");
    setup(&tmp);
    let arhiiv = tmp.join("proov.pakk");
    assert!(pakk()
        .args(["c", arhiiv.to_str().unwrap(), "andmed"])
        .current_dir(&tmp)
        .status()
        .unwrap()
        .success());

    let out = pakk().args(["l", arhiiv.to_str().unwrap()]).output().unwrap();
    assert!(out.status.success());
    let tekst = String::from_utf8_lossy(&out.stdout);
    assert!(tekst.contains("andmed/tekst.txt"), "sisu puudub:\n{tekst}");
    assert!(tekst.contains("kokku:"));
}

#[test]
fn tundmatu_vorming_annab_vea() {
    let tmp = tempdir("viga");
    let vale = tmp.join("vale.pakk");
    fs::write(&vale, b"see pole arhiiv, vaid tekst").unwrap();
    let out = pakk().args(["l", vale.to_str().unwrap()]).output().unwrap();
    assert!(!out.status.success());
}

fn tempdir(nimi: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("pakk-test-{nimi}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}
