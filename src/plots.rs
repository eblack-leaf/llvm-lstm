use std::path::Path;

use anyhow::Result;

/// Generate all EDA plots by invoking the Python script.
///
/// The script reads the JSON files already written by `eda.rs` in `output_dir`
/// and produces matplotlib/seaborn figures alongside them.
pub fn generate_all(output_dir: &Path) -> Result<()> {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script = manifest_dir.join("scripts/plot_eda.py");

    if !script.exists() {
        eprintln!(
            "Warning: scripts/plot_eda.py not found at {}, skipping plots",
            script.display()
        );
        return Ok(());
    }

    // Prefer the project .venv if present, fall back to system python3.
    let venv_python = manifest_dir.join(".venv/bin/python3");
    let python: std::path::PathBuf = if venv_python.exists() {
        venv_python
    } else {
        std::path::PathBuf::from("python3")
    };

    let status = std::process::Command::new(&python)
        .arg(&script)
        .arg(output_dir)
        .status()
        .map_err(|e| anyhow::anyhow!("Failed to launch {}: {e}", python.display()))?;

    if !status.success() {
        eprintln!("Warning: plot_eda.py exited with {status}");
    }

    Ok(())
}
