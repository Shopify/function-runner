use anyhow::{anyhow, bail, Result};
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::LazyLock,
};

pub fn process_with_v1_trampoline<P: AsRef<Path>, Q: AsRef<Path>>(
    wasm_path: P,
    trampolined_path: Q,
) -> Result<()> {
    process_with_trampoline(&TRAMPOLINE_1_0_PATH, wasm_path, trampolined_path)
}

pub fn process_with_v2_trampoline<P: AsRef<Path>, Q: AsRef<Path>>(
    wasm_path: P,
    trampolined_path: Q,
) -> Result<()> {
    process_with_trampoline(&TRAMPOLINE_2_0_PATH, wasm_path, trampolined_path)
}

fn process_with_trampoline<P: AsRef<Path>, Q: AsRef<Path>>(
    trampoline_path: &LazyLock<Result<PathBuf>>,
    wasm_path: P,
    trampolined_path: Q,
) -> Result<()> {
    let trampoline_path = trampoline_path
        .as_ref()
        .map_err(|e| anyhow!("Failed to download trampoline: {e}"))?;
    let status = Command::new(trampoline_path)
        .arg("-i")
        .arg(wasm_path.as_ref())
        .arg("-o")
        .arg(trampolined_path.as_ref())
        .status()?;
    if !status.success() {
        bail!("Trampolining failed");
    }
    Ok(())
}

static TRAMPOLINE_1_0_PATH: LazyLock<Result<PathBuf>> = LazyLock::new(|| trampoline_path("1.0.2"));

static TRAMPOLINE_2_0_PATH: LazyLock<Result<PathBuf>> = LazyLock::new(|| trampoline_path("2.0.0"));

fn trampoline_path(version: &str) -> Result<PathBuf> {
    let binaries_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tmp");
    let path = binaries_path.join(format!("trampoline-{version}"));
    if !path.exists() {
        std::fs::create_dir_all(binaries_path)?;
        download_trampoline(&path, version)?;
    }
    Ok(path)
}

fn download_trampoline(destination: &Path, version: &str) -> Result<()> {
    let target_os = if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        bail!("Unsupported target OS");
    };

    let target_arch = if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "arm"
    } else {
        bail!("Unsupported target architecture");
    };

    let url = format!(
            "https://github.com/Shopify/shopify-function-wasm-api/releases/download/shopify_function_trampoline/v{version}/shopify-function-trampoline-{target_arch}-{target_os}-v{version}.gz",
        );

    let response = reqwest::blocking::get(&url)?;
    if !response.status().is_success() {
        bail!("Failed to download artifact: {}", response.status());
    }
    let bytes = response.bytes()?;
    let mut gz_decoder = flate2::read::GzDecoder::new(bytes.as_ref());
    let mut file = std::fs::File::create(destination)?;
    std::io::copy(&mut gz_decoder, &mut file)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = file.metadata()?.permissions();
        perms.set_mode(0o755);
        file.set_permissions(perms)?;
    }

    Ok(())
}
