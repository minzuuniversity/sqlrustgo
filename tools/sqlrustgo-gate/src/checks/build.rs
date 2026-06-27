//! Build check: `cargo build --all`

use crate::workspace::workspace_root;
use std::process::Command;

pub fn check() -> anyhow::Result<()> {
    println!("[check] cargo build --all ...");

    let root = workspace_root();
    println!("[check] workspace root: {}", root.display());

    let status = Command::new("cargo")
        .args(["build", "--all"])
        .current_dir(&root)
        .spawn()?
        .wait()?;

    if !status.success() {
        anyhow::bail!("build failed with exit code: {:?}", status.code());
    }

    println!("[check] build OK");
    Ok(())
}
