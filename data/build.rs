use std::path::Path;
use std::process::Command;

const VERSION: &str = include_str!("../VERSION");

fn main() {
    let git_hash = Command::new("git")
        .args(["describe", "--always", "--dirty", "--exclude='*'"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|x| String::from_utf8(x.stdout).ok());

    println!("cargo:rerun-if-changed=../VERSION");
    println!("cargo:rustc-env=VERSION={}", VERSION);

    if let Some(hash) = git_hash.as_ref() {
        println!("cargo:rustc-env=GIT_HASH={}", hash);
    }

    if git_hash.is_none() {
        return;
    }

    let Some(git_dir): Option<String> = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|x| String::from_utf8(x.stdout).ok())
    else{ return; };
    // If heads starts pointing at something else (different branch)
    // we need to return
    let head = Path::new(&git_dir).join("HEAD");
    if head.exists() {
        println!("cargo:rerun-if-changed={}", head.display());
    }
    // if the thing head points to (branch) itself changes
    // we need to return
    let Some(head_ref): Option<String> = Command::new("git")
        .args(["symbolic-ref", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|x| String::from_utf8(x.stdout).ok())
    else{ return; };
    let head_ref = Path::new(&git_dir).join(head_ref);
    if head_ref.exists() {
        println!("cargo:rerun-if-changed={}", head_ref.display());
    }
}
