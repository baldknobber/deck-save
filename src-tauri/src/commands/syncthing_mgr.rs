use serde::Serialize;
use std::path::PathBuf;
use std::process::Command;

/// Platform-specific Syncthing binary name.
#[cfg(target_os = "windows")]
const SYNCTHING_BIN: &str = "syncthing.exe";
#[cfg(not(target_os = "windows"))]
const SYNCTHING_BIN: &str = "syncthing";

/// GitHub API URL for latest Syncthing release.
const RELEASES_URL: &str = "https://api.github.com/repos/syncthing/syncthing/releases/latest";

/// Determine the expected archive name suffix for this platform.
fn platform_archive_suffix() -> &'static str {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    { "-linux-amd64" }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    { "-windows-amd64" }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    { "-linux-arm64" }
    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64"),
    )))]
    { "-unknown" }
}

/// Directory where we store the managed Syncthing binary.
fn syncthing_dir(app_data_dir: &std::path::Path) -> PathBuf {
    app_data_dir.join("syncthing")
}

fn syncthing_bin_path(app_data_dir: &std::path::Path) -> PathBuf {
    syncthing_dir(app_data_dir).join(SYNCTHING_BIN)
}

/// Check if syncthing is on the system PATH.
fn system_syncthing() -> Option<PathBuf> {
    which_syncthing()
}

#[cfg(target_os = "windows")]
fn which_syncthing() -> Option<PathBuf> {
    Command::new("where")
        .arg(SYNCTHING_BIN)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| {
            String::from_utf8(o.stdout)
                .ok()
                .and_then(|s| s.lines().next().map(|l| PathBuf::from(l.trim())))
        })
}

#[cfg(not(target_os = "windows"))]
fn which_syncthing() -> Option<PathBuf> {
    Command::new("which")
        .arg(SYNCTHING_BIN)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| {
            String::from_utf8(o.stdout)
                .ok()
                .map(|s| PathBuf::from(s.trim()))
        })
}

#[derive(Serialize)]
pub struct SyncthingInfo {
    pub installed: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub managed: bool,
}

#[derive(Serialize)]
pub struct InstallResult {
    pub version: String,
    pub path: String,
}

#[tauri::command]
pub fn check_syncthing_installed(
    state: tauri::State<'_, crate::AppState>,
) -> Result<SyncthingInfo, String> {
    let app_dir = &state.app_data_dir;

    // Check managed binary first
    let managed_path = syncthing_bin_path(app_dir);
    if managed_path.exists() {
        let version = get_version(&managed_path);
        return Ok(SyncthingInfo {
            installed: true,
            path: Some(managed_path.to_string_lossy().to_string()),
            version,
            managed: true,
        });
    }

    // Check system PATH
    if let Some(sys_path) = system_syncthing() {
        let version = get_version(&sys_path);
        return Ok(SyncthingInfo {
            installed: true,
            path: Some(sys_path.to_string_lossy().to_string()),
            version,
            managed: false,
        });
    }

    Ok(SyncthingInfo {
        installed: false,
        path: None,
        version: None,
        managed: false,
    })
}

fn get_version(bin: &std::path::Path) -> Option<String> {
    Command::new(bin)
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| {
            String::from_utf8(o.stdout)
                .ok()
                .and_then(|s| {
                    // "syncthing v1.28.0 ..."
                    s.split_whitespace().nth(1).map(|v| v.to_string())
                })
        })
}

#[tauri::command]
pub fn install_syncthing(
    state: tauri::State<'_, crate::AppState>,
) -> Result<InstallResult, String> {
    let app_dir = &state.app_data_dir;
    let suffix = platform_archive_suffix();
    if suffix == "-unknown" {
        return Err("Unsupported platform for Syncthing auto-install".into());
    }

    // Fetch latest release info from GitHub
    let resp: serde_json::Value = ureq::get(RELEASES_URL)
        .set("User-Agent", "DeckSave")
        .call()
        .map_err(|e| format!("Failed to fetch Syncthing releases: {e}"))?
        .into_json()
        .map_err(|e| format!("Failed to parse release JSON: {e}"))?;

    let tag = resp["tag_name"]
        .as_str()
        .ok_or("No tag_name in release")?
        .to_string();

    let assets = resp["assets"]
        .as_array()
        .ok_or("No assets in release")?;

    // Find the right asset for our platform
    let asset_url = assets
        .iter()
        .filter_map(|a| {
            let name = a["name"].as_str()?;
            let url = a["browser_download_url"].as_str()?;
            if name.contains(suffix) && (name.ends_with(".tar.gz") || name.ends_with(".zip")) {
                Some(url.to_string())
            } else {
                None
            }
        })
        .next()
        .ok_or_else(|| format!("No Syncthing asset found for platform suffix '{suffix}'"))?;

    // Download to temp file
    let tmp_dir = std::env::temp_dir();
    let archive_name = asset_url.rsplit('/').next().unwrap_or("syncthing-archive");
    let tmp_path = tmp_dir.join(archive_name);

    let resp = ureq::get(&asset_url)
        .set("User-Agent", "DeckSave")
        .call()
        .map_err(|e| format!("Failed to download Syncthing: {e}"))?;

    let mut bytes = Vec::new();
    resp.into_reader()
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Failed to read download: {e}"))?;

    std::fs::write(&tmp_path, &bytes)
        .map_err(|e| format!("Failed to write temp file: {e}"))?;

    // Extract binary
    let target_dir = syncthing_dir(app_dir);
    std::fs::create_dir_all(&target_dir)
        .map_err(|e| format!("Failed to create syncthing dir: {e}"))?;

    let target_bin = target_dir.join(SYNCTHING_BIN);

    if archive_name.ends_with(".tar.gz") {
        extract_tar_gz(&tmp_path, &target_bin)?;
    } else {
        extract_zip(&tmp_path, &target_bin)?;
    }

    // Clean up temp
    let _ = std::fs::remove_file(&tmp_path);

    // Make executable on Linux
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&target_bin, std::fs::Permissions::from_mode(0o755));
    }

    let version = get_version(&target_bin).unwrap_or_else(|| tag.clone());

    Ok(InstallResult {
        version,
        path: target_bin.to_string_lossy().to_string(),
    })
}

fn extract_tar_gz(archive: &std::path::Path, target_bin: &std::path::Path) -> Result<(), String> {
    use std::io::Read;
    let file = std::fs::File::open(archive)
        .map_err(|e| format!("Cannot open archive: {e}"))?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut tar = tar::Archive::new(gz);

    for entry_result in tar.entries().map_err(|e| format!("Cannot read tar: {e}"))? {
        let mut entry = entry_result.map_err(|e| format!("Tar entry error: {e}"))?;
        let path = entry.path().map_err(|e| format!("Path error: {e}"))?;
        let path_str = path.to_string_lossy();
        // The binary is at syncthing-<platform>/syncthing
        if path_str.ends_with("/syncthing") || path_str == "syncthing" {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).map_err(|e| format!("Read error: {e}"))?;
            std::fs::write(target_bin, buf)
                .map_err(|e| format!("Write error: {e}"))?;
            return Ok(());
        }
    }
    Err("syncthing binary not found in tar.gz archive".into())
}

fn extract_zip(archive: &std::path::Path, target_bin: &std::path::Path) -> Result<(), String> {
    use std::io::Read;
    let file = std::fs::File::open(archive)
        .map_err(|e| format!("Cannot open archive: {e}"))?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|e| format!("Cannot read zip: {e}"))?;

    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| format!("Zip entry error: {e}"))?;
        let name = entry.name().to_string();
        if name.ends_with(SYNCTHING_BIN) {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).map_err(|e| format!("Read error: {e}"))?;
            std::fs::write(target_bin, buf)
                .map_err(|e| format!("Write error: {e}"))?;
            return Ok(());
        }
    }
    Err("syncthing binary not found in zip archive".into())
}

#[tauri::command]
pub fn start_syncthing(
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    let app_dir = &state.app_data_dir;

    // Determine which binary to use
    let bin = {
        let managed = syncthing_bin_path(app_dir);
        if managed.exists() {
            managed
        } else if let Some(sys) = system_syncthing() {
            sys
        } else {
            return Err("Syncthing is not installed".into());
        }
    };

    // Check if already running by trying the API
    let already_running = ureq::get("http://127.0.0.1:8384/rest/system/ping")
        .call()
        .is_ok();
    if already_running {
        return Ok(());
    }

    let child = Command::new(&bin)
        .arg("serve")
        .arg("--no-browser")
        .arg("--gui-address=http://127.0.0.1:8384")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to start Syncthing: {e}"))?;

    let mut guard = state.syncthing_process.lock().map_err(|e| e.to_string())?;
    *guard = Some(child);

    Ok(())
}

#[tauri::command]
pub fn stop_syncthing(
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    let mut guard = state.syncthing_process.lock().map_err(|e| e.to_string())?;
    if let Some(ref mut child) = *guard {
        let _ = child.kill();
        let _ = child.wait();
    }
    *guard = None;
    Ok(())
}
