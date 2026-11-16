use anyhow::{anyhow, Result};
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tauri::{AppHandle, Manager};

const WHISTLE_MODEL_URL: &str =
    "https://huggingface.co/Cactus-Compute/whistle/resolve/main/whistle.cact";

#[derive(Serialize, Clone)]
pub struct WhistleStatus {
    pub model_ready: bool,
    pub runtime_ready: bool,
    pub model_path: String,
    pub runtime_path: String,
    pub platform_folder: String,
}

pub fn platform_folder() -> &'static str {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return "macos-arm64";
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return "linux-x86_64";
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    return "linux-arm64";
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return "windows-x86_64";
    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    return "windows-arm64";
    #[allow(unreachable_code)]
    "macos-arm64"
}

fn binary_name() -> &'static str {
    #[cfg(target_os = "windows")]
    return "cactus.exe";
    #[cfg(not(target_os = "windows"))]
    return "cactus";
}

pub fn asset_paths(app: &AppHandle) -> Result<(PathBuf, PathBuf)> {
    let dir = app.path().app_data_dir()?.join("whistle");
    fs::create_dir_all(&dir)?;
    Ok((dir.join("whistle.cact"), dir.join(binary_name())))
}

pub fn get_status(app: &AppHandle) -> Result<WhistleStatus> {
    let (model_path, runtime_path) = asset_paths(app)?;
    Ok(WhistleStatus {
        model_ready: model_path.exists() && fs::metadata(&model_path).map(|m| m.len() > 1_000_000).unwrap_or(false),
        runtime_ready: runtime_path.exists(),
        model_path: model_path.to_string_lossy().to_string(),
        runtime_path: runtime_path.to_string_lossy().to_string(),
        platform_folder: platform_folder().to_string(),
    })
}

pub async fn download_assets(app: &AppHandle) -> Result<WhistleStatus> {
    let (model_path, runtime_path) = asset_paths(app)?;
    let client = reqwest::Client::new();

    // 1. Download whistle.cact (16.9 MB)
    let model_bytes = client
        .get(WHISTLE_MODEL_URL)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    fs::write(&model_path, &model_bytes)?;

    // 2. Download Cactus runtime binary for current platform
    let folder = platform_folder();
    let bin = binary_name();
    let runtime_url = format!(
        "https://huggingface.co/Cactus-Compute/cactus/resolve/main/releases/4.0.0/{}/{}",
        folder, bin
    );
    let bin_bytes = client
        .get(&runtime_url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    fs::write(&runtime_path, &bin_bytes)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&runtime_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&runtime_path, perms)?;
    }

    get_status(app)
}

pub fn transcribe_wav(app: &AppHandle, wav_path: &PathBuf, language: &str) -> Result<String> {
    let (model_path, runtime_path) = asset_paths(app)?;
    if !model_path.exists() || !runtime_path.exists() {
        return Err(anyhow!(
            "Whistle model or Cactus runtime not found. Click 'Download Whistle (16.9 MB)' first."
        ));
    }

    let mut cmd = Command::new(&runtime_path);
    cmd.arg("--model")
        .arg(&model_path)
        .arg("--audio")
        .arg(wav_path);

    if language != "auto" && !language.is_empty() {
        cmd.arg("--language").arg(language);
    }

    let output = cmd.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Cactus Whistle failed: {}", stderr.trim()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    // Handle either plain transcript output or JSON output if returned by the runner
    if stdout.starts_with('{') {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&stdout) {
            if let Some(txt) = val.get("text").and_then(|v| v.as_str()) {
                return Ok(txt.trim().to_string());
            }
            if let Some(txt) = val.get("audio_text").and_then(|v| v.as_str()) {
                return Ok(txt.trim().to_string());
            }
        }
    }
    Ok(stdout)
}
