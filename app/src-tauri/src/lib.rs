// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tauri::path::BaseDirectory;
use tauri::Manager;

const MAX_CAPTURE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Default)]
struct AppState {
    active_pid: Arc<AtomicU32>,
}

struct ActiveOperation(Arc<AtomicU32>);

impl Drop for ActiveOperation {
    fn drop(&mut self) {
        self.0.store(0, Ordering::Release);
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OperationRequest {
    action: String,
    model_dir: Option<String>,
    destination: Option<String>,
    trunk_dir: Option<String>,
    prompt: Option<String>,
    benchmark_file: Option<String>,
    measure_io: Option<bool>,
    max_tokens: Option<u16>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OperationResult {
    action: String,
    exit_code: i32,
    success: bool,
    stdout: String,
    stderr: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeInfo {
    app_version: &'static str,
    cli_path: Option<String>,
    cli_available: bool,
    docker_available: bool,
    active_pid: u32,
    local_only: bool,
}

fn nonempty(value: Option<String>, label: &str) -> Result<String, String> {
    let value = value.unwrap_or_default();
    if value.trim().is_empty() {
        Err(format!("{label} is required"))
    } else {
        Ok(value)
    }
}

fn command_available(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn bundled_resource(app: Option<&tauri::AppHandle>, relative: &str) -> Option<PathBuf> {
    app.and_then(|handle| {
        handle
            .path()
            .resolve(relative, BaseDirectory::Resource)
            .ok()
    })
    .filter(|candidate| candidate.is_file())
}

fn cli_path(app: Option<&tauri::AppHandle>) -> Option<PathBuf> {
    if let Some(path) = env::var_os("XCALIBER_CLI").map(PathBuf::from) {
        if path.is_file() {
            return Some(path);
        }
    }
    if let Some(path) = bundled_resource(
        app,
        if cfg!(windows) {
            "runtime/xcaliber.exe"
        } else {
            "runtime/xcaliber"
        },
    ) {
        return Some(path);
    }
    let executable = env::current_exe().ok()?;
    let directory = executable.parent()?;
    let names: &[&str] = if cfg!(windows) {
        &["xcaliber.exe", "../windows-cli/xcaliber.exe"]
    } else {
        &["xcaliber", "../windows-cli/xcaliber"]
    };
    names
        .iter()
        .map(|name| directory.join(name))
        .find(|candidate| candidate.is_file())
}

fn compose_path(app: Option<&tauri::AppHandle>) -> Option<PathBuf> {
    if let Some(path) = bundled_resource(app, "docker/compose.yaml") {
        return Some(path);
    }
    let executable = env::current_exe().ok()?;
    let directory = executable.parent()?;
    [
        directory.join("docker/compose.yaml"),
        directory.join("../docker/compose.yaml"),
    ]
    .into_iter()
    .find(|candidate| candidate.is_file())
}

fn require_path(path: String, label: &str) -> Result<PathBuf, String> {
    let candidate = PathBuf::from(path);
    if candidate.as_os_str().is_empty() {
        return Err(format!("{label} is required"));
    }
    Ok(candidate)
}

fn build_arguments(request: &OperationRequest) -> Result<Vec<String>, String> {
    let mut args = Vec::new();
    match request.action.as_str() {
        "requirements" => args.push("requirements".to_string()),
        "doctor" => {
            args.push("doctor".to_string());
            if let Some(model) = request.model_dir.as_deref().filter(|path| !path.is_empty()) {
                args.extend(["--model-dir".to_string(), model.to_string()]);
            }
            args.push("--json".to_string());
        }
        "plan" => {
            args.push("plan".to_string());
            if let Some(model) = request.model_dir.as_deref().filter(|path| !path.is_empty()) {
                args.extend(["--model-dir".to_string(), model.to_string()]);
            }
            if request.measure_io.unwrap_or(false) {
                args.push("--measure-io".to_string());
            }
            if let Some(file) = request
                .benchmark_file
                .as_deref()
                .filter(|path| !path.is_empty())
            {
                args.extend(["--benchmark-file".to_string(), file.to_string()]);
            }
            args.push("--json".to_string());
        }
        "pull" => {
            let destination = require_path(
                nonempty(request.destination.clone(), "model destination")?,
                "model destination",
            )?;
            args.extend([
                "pull".to_string(),
                "--destination".to_string(),
                destination.display().to_string(),
            ]);
        }
        "pack" => {
            let model = require_path(
                nonempty(request.model_dir.clone(), "model directory")?,
                "model directory",
            )?;
            let destination = require_path(
                nonempty(request.destination.clone(), "trunk destination")?,
                "trunk destination",
            )?;
            args.extend([
                "pack".to_string(),
                "--model-dir".to_string(),
                model.display().to_string(),
                "--destination".to_string(),
                destination.display().to_string(),
            ]);
        }
        "run" => {
            let model = require_path(
                nonempty(request.model_dir.clone(), "model directory")?,
                "model directory",
            )?;
            let trunk = require_path(
                nonempty(request.trunk_dir.clone(), "packed trunk directory")?,
                "packed trunk directory",
            )?;
            let prompt = nonempty(request.prompt.clone(), "prompt")?;
            let max_tokens = request.max_tokens.unwrap_or(32).clamp(1, 4096);
            args.extend([
                "run".to_string(),
                "--model-dir".to_string(),
                model.display().to_string(),
                "--trunk".to_string(),
                trunk.display().to_string(),
                "--prompt".to_string(),
                prompt,
                "--gen".to_string(),
                max_tokens.to_string(),
            ]);
        }
        _ => return Err("operation is not allowed".to_string()),
    }
    Ok(args)
}

fn build_docker_arguments(action: &str, compose: &Path) -> Result<Vec<String>, String> {
    let mut args = vec![
        "compose".to_string(),
        "-f".to_string(),
        compose.display().to_string(),
    ];
    match action {
        "docker_start" => args.extend(["up", "-d"].map(str::to_string)),
        "docker_stop" => args.extend(["down"].map(str::to_string)),
        "docker_status" => args.extend(["ps"].map(str::to_string)),
        "docker_logs" => args.extend(["logs", "--tail", "200"].map(str::to_string)),
        _ => return Err("Docker operation is not allowed".to_string()),
    }
    Ok(args)
}

fn bounded_text(bytes: &[u8]) -> String {
    let start = bytes.len().saturating_sub(MAX_CAPTURE_BYTES);
    let text = String::from_utf8_lossy(&bytes[start..]);
    if start == 0 {
        text.into_owned()
    } else {
        format!("[earlier output omitted]\n{text}")
    }
}

#[tauri::command]
fn runtime_info(app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> RuntimeInfo {
    let cli = cli_path(Some(&app));
    RuntimeInfo {
        app_version: env!("CARGO_PKG_VERSION"),
        cli_path: cli
            .as_deref()
            .map(Path::display)
            .map(|path| path.to_string()),
        cli_available: cli.is_some(),
        docker_available: command_available("docker"),
        active_pid: state.active_pid.load(Ordering::Acquire),
        local_only: true,
    }
}

#[tauri::command]
async fn execute(
    app: tauri::AppHandle,
    request: OperationRequest,
    state: tauri::State<'_, AppState>,
) -> Result<OperationResult, String> {
    let docker_action = request.action.starts_with("docker_");
    let (program, args, model_environment) = if docker_action {
        if !command_available("docker") {
            return Err("Docker is not installed or is not running".to_string());
        }
        let compose = compose_path(Some(&app)).ok_or(
            "docker/compose.yaml was not found next to the app. Keep the portable folder intact.",
        )?;
        let model = nonempty(request.model_dir.clone(), "official model directory")?;
        (
            PathBuf::from("docker"),
            build_docker_arguments(&request.action, &compose)?,
            Some(model),
        )
    } else {
        (
            cli_path(Some(&app))
                .ok_or("xcaliber CLI was not found. Keep xcaliber.exe next to this app or set XCALIBER_CLI.")?,
            build_arguments(&request)?,
            None,
        )
    };
    let action = request.action;
    let active = Arc::clone(&state.active_pid);
    active
        .compare_exchange(0, u32::MAX, Ordering::AcqRel, Ordering::Acquire)
        .map_err(|_| "another Xcaliber operation is already running".to_string())?;

    tauri::async_runtime::spawn_blocking(move || {
        let _active_operation = ActiveOperation(Arc::clone(&active));
        let mut command = Command::new(program);
        command
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(model) = model_environment {
            command.env("MODEL_DIR", model);
        }
        let child = command
            .spawn()
            .map_err(|error| format!("could not start {action}: {error}"))?;
        active.store(child.id(), Ordering::Release);
        let output = child
            .wait_with_output()
            .map_err(|error| format!("could not wait for xcaliber: {error}"));
        let output = output?;
        let exit_code = output.status.code().unwrap_or(1);
        Ok(OperationResult {
            action,
            exit_code,
            success: output.status.success(),
            stdout: bounded_text(&output.stdout),
            stderr: bounded_text(&output.stderr),
        })
    })
    .await
    .map_err(|error| {
        state.active_pid.store(0, Ordering::Release);
        format!("Xcaliber operation task failed: {error}")
    })?
}

#[tauri::command]
fn cancel_active(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let pid = state.active_pid.load(Ordering::Acquire);
    if pid == 0 || pid == u32::MAX {
        return Err("no cancellable Xcaliber operation is running".to_string());
    }
    #[cfg(windows)]
    let status = Command::new("taskkill.exe")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .status()
        .map_err(|error| format!("could not cancel process {pid}: {error}"))?;
    #[cfg(not(windows))]
    let status = Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .status()
        .map_err(|error| format!("could not cancel process {pid}: {error}"))?;
    if status.success() {
        Ok(format!("cancellation requested for process {pid}"))
    } else {
        Err(format!("the operating system did not cancel process {pid}"))
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            runtime_info,
            execute,
            cancel_active
        ])
        .run(tauri::generate_context!())
        .expect("Xcaliber desktop failed to start");
}

#[cfg(test)]
mod tests {
    use super::{build_arguments, build_docker_arguments, OperationRequest};
    use std::path::Path;

    fn request(action: &str) -> OperationRequest {
        OperationRequest {
            action: action.to_string(),
            model_dir: None,
            destination: None,
            trunk_dir: None,
            prompt: None,
            benchmark_file: None,
            measure_io: None,
            max_tokens: None,
        }
    }

    #[test]
    fn arbitrary_commands_are_refused() {
        assert!(build_arguments(&request("shell")).is_err());
    }

    #[test]
    fn doctor_is_json_and_does_not_require_a_model() {
        let args = build_arguments(&request("doctor")).expect("doctor should be allowed");
        assert_eq!(args, ["doctor", "--json"]);
    }

    #[test]
    fn run_requires_model_trunk_and_prompt() {
        assert!(build_arguments(&request("run")).is_err());
    }

    #[test]
    fn docker_commands_are_fixed_and_shell_free() {
        let args = build_docker_arguments("docker_start", Path::new("compose.yaml"))
            .expect("start is allowed");
        assert_eq!(
            args,
            ["compose", "-f", "compose.yaml", "up", "-d"]
        );
        assert!(build_docker_arguments("docker_exec", Path::new("compose.yaml")).is_err());
    }
}
