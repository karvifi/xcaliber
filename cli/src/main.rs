// SPDX-License-Identifier: AGPL-3.0-only
//! Local-only control CLI for Kimi K3.

use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

mod local_api;
mod planner;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const MODEL_REPO: &str = "moonshotai/Kimi-K3";
const EXPECTED_SHARDS: usize = 96;
const EXPECTED_BYTES: u64 = 1_560_936_091_448;
const DOWNLOAD_RESERVE_BYTES: u64 = 20_000_000_000;
const SHARD_SIZES: &str = include_str!("../assets/shard_sizes.txt");

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
enum Level {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Serialize)]
struct Check {
    level: Level,
    name: String,
    detail: String,
}

#[derive(Debug, Serialize)]
struct DoctorReport {
    product: &'static str,
    version: &'static str,
    local_only: bool,
    ready: bool,
    model_dir: Option<String>,
    cpu_threads: usize,
    total_memory_bytes: u64,
    model_bytes_present: u64,
    expected_model_bytes: u64,
    free_bytes_at_model_path: Option<u64>,
    checks: Vec<Check>,
}

#[derive(Default)]
struct CommonOptions {
    model_dir: Option<PathBuf>,
    json: bool,
}

fn usage() {
    println!(
        "xcaliber {VERSION} - local-only Kimi K3\n\n\
         Usage:\n\
           xcaliber requirements\n\
           xcaliber doctor [--model-dir PATH] [--json]\n\
           xcaliber plan [--model-dir PATH] [--measure-io] [--benchmark-file PATH] [--json]\n\
           xcaliber pull --destination PATH [--revision SHA] [--workers N] [--skip-checksum]\n\
           xcaliber pack --model-dir PATH --destination PATH\n\
           xcaliber run --model-dir PATH --trunk PATH [engine options]\n\
           xcaliber chat --prompt TEXT [--api-url URL] [--model NAME] [--api-key KEY] [--max-tokens N] [--json]\n\n\
         No Moonshot/Kimi API key is used. Model weights remain on the selected drive.\n\
         `run` is the native CPU reference. `chat` accepts loopback HTTP only."
    );
}

fn requirements() {
    println!("Kimi K3 local requirements");
    println!(
        "  checkpoint: {} bytes across {} shards",
        EXPECTED_BYTES, EXPECTED_SHARDS
    );
    println!("  checkpoint only: 1.56 TB free on one or more supported model drives");
    println!("  low-memory native mode: about 109 GB more for a packed dense trunk");
    println!("  practical target: a fast external/internal SSD with at least 1.70 TB free");
    println!("  RAM: 8 GB can execute the exact streamed reference, but measured decode was about 72.5 s/token");
    println!(
        "  adaptive dense-resident mode: about 40 GB RAM minimum; changes numerical precision"
    );
    println!(
        "  interactive exact target: requires workstation-class aggregate memory/storage bandwidth"
    );
    println!("  GPU: optional; the Docker/Colibri runtime can use Vulkan when built for it");
    println!("  network: required only while pulling the public model");
    println!("  API key: not required");
}

fn parse_common(args: &[OsString]) -> Result<CommonOptions, String> {
    let mut out = CommonOptions::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].to_string_lossy().as_ref() {
            "--model-dir" => {
                i += 1;
                out.model_dir = Some(PathBuf::from(
                    args.get(i).ok_or("--model-dir needs a path")?,
                ));
            }
            "--json" => out.json = true,
            other => return Err(format!("unknown option for doctor: {other}")),
        }
        i += 1;
    }
    if out.model_dir.is_none() {
        out.model_dir = env::var_os("K3_MODEL_DIR").map(PathBuf::from);
    }
    Ok(out)
}

fn shard_sizes() -> BTreeMap<&'static str, u64> {
    SHARD_SIZES
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            Some((fields.next()?, fields.next()?.parse().ok()?))
        })
        .collect()
}

fn model_bytes_present(dir: &Path) -> u64 {
    shard_sizes()
        .into_iter()
        .map(|(name, expected)| {
            fs::metadata(dir.join(name))
                .map(|meta| meta.len().min(expected))
                .unwrap_or(0)
        })
        .sum()
}

fn push(checks: &mut Vec<Check>, level: Level, name: &str, detail: impl Into<String>) {
    checks.push(Check {
        level,
        name: name.to_string(),
        detail: detail.into(),
    });
}

fn validate_model(dir: &Path, checks: &mut Vec<Check>) -> bool {
    if !dir.is_dir() {
        push(
            checks,
            Level::Fail,
            "model directory",
            format!("{} does not exist", dir.display()),
        );
        return false;
    }
    push(
        checks,
        Level::Pass,
        "model directory",
        dir.display().to_string(),
    );

    let mut valid = true;
    let config_path = dir.join("config.json");
    match fs::read_to_string(&config_path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
    {
        Some(config) if config.get("model_type").and_then(Value::as_str) == Some("kimi_k3") => {
            push(checks, Level::Pass, "model config", "model_type is kimi_k3");
        }
        Some(_) => {
            push(
                checks,
                Level::Fail,
                "model config",
                "config.json is not a Kimi K3 config",
            );
            valid = false;
        }
        None => {
            push(
                checks,
                Level::Fail,
                "model config",
                "config.json is missing or invalid",
            );
            valid = false;
        }
    }

    for name in [
        "model.safetensors.index.json",
        "tiktoken.model",
        "tokenizer_config.json",
    ] {
        if dir.join(name).is_file() {
            push(checks, Level::Pass, name, "present");
        } else {
            push(checks, Level::Fail, name, "missing");
            valid = false;
        }
    }

    let expected = shard_sizes();
    let mut good = 0usize;
    let mut wrong = Vec::new();
    let mut total = 0u64;
    for (name, size) in expected {
        match fs::metadata(dir.join(name)) {
            Ok(meta) if meta.len() == size => {
                good += 1;
                total = total.saturating_add(size);
            }
            Ok(meta) => wrong.push(format!("{name}: {} of {size} bytes", meta.len())),
            Err(_) => {}
        }
    }
    if good == EXPECTED_SHARDS && total == EXPECTED_BYTES {
        push(
            checks,
            Level::Pass,
            "checkpoint shards",
            format!("{good} shards, {total} bytes"),
        );
    } else {
        let detail = if wrong.is_empty() {
            format!("{good}/{EXPECTED_SHARDS} complete shards; {total}/{EXPECTED_BYTES} bytes")
        } else {
            format!(
                "{good}/{EXPECTED_SHARDS} complete; size errors: {}",
                wrong.join(", ")
            )
        };
        push(checks, Level::Fail, "checkpoint shards", detail);
        valid = false;
    }
    valid
}

fn doctor_report(model_dir: Option<&Path>) -> DoctorReport {
    let mut checks = Vec::new();
    let memory = total_memory_bytes();
    let threads = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1);
    push(
        &mut checks,
        Level::Pass,
        "CPU",
        format!("{threads} logical threads available"),
    );
    if memory > 0 {
        let level = if memory >= 8_000_000_000 {
            Level::Pass
        } else {
            Level::Fail
        };
        push(&mut checks, level, "RAM", human_bytes(memory));
    } else {
        push(
            &mut checks,
            Level::Warn,
            "RAM",
            "could not read total memory",
        );
    }

    let engine = engine_path();
    match &engine {
        Some(path) => push(
            &mut checks,
            Level::Pass,
            "native engine",
            path.display().to_string(),
        ),
        None => push(
            &mut checks,
            Level::Fail,
            "native engine",
            "xcaliber-engine executable not found",
        ),
    }

    let (model_valid, present, free) = if let Some(dir) = model_dir {
        let valid = validate_model(dir, &mut checks);
        let present = model_bytes_present(dir);
        let free = free_space_bytes(dir).ok();
        if let Some(bytes) = free {
            push(
                &mut checks,
                Level::Pass,
                "free space at model path",
                human_bytes(bytes),
            );
        }
        (valid, present, free)
    } else {
        push(
            &mut checks,
            Level::Fail,
            "model directory",
            "pass --model-dir or set K3_MODEL_DIR",
        );
        (false, 0, None)
    };

    let ready = model_valid && engine.is_some() && memory >= 8_000_000_000;
    DoctorReport {
        product: "xcaliber",
        version: VERSION,
        local_only: true,
        ready,
        model_dir: model_dir.map(|path| path.display().to_string()),
        cpu_threads: threads,
        total_memory_bytes: memory,
        model_bytes_present: present,
        expected_model_bytes: EXPECTED_BYTES,
        free_bytes_at_model_path: free,
        checks,
    }
}

fn print_report(report: &DoctorReport, json: bool) -> Result<(), String> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(report).map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    println!("Xcaliber local readiness");
    for check in &report.checks {
        let label = match check.level {
            Level::Pass => "PASS",
            Level::Warn => "WARN",
            Level::Fail => "FAIL",
        };
        println!("  {label:<4}  {:<22} {}", check.name, check.detail);
    }
    println!(
        "\nResult: {}",
        if report.ready { "READY" } else { "NOT READY" }
    );
    Ok(())
}

fn performance_plan(args: &[OsString]) -> Result<i32, String> {
    let mut model_dir = env::var_os("K3_MODEL_DIR").map(PathBuf::from);
    let mut benchmark_file = None;
    let mut measure_io = false;
    let mut json = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].to_string_lossy().as_ref() {
            "--model-dir" => {
                i += 1;
                model_dir = Some(PathBuf::from(
                    args.get(i).ok_or("--model-dir needs a path")?,
                ));
            }
            "--benchmark-file" => {
                i += 1;
                benchmark_file = Some(PathBuf::from(
                    args.get(i).ok_or("--benchmark-file needs a path")?,
                ));
            }
            "--measure-io" => measure_io = true,
            "--json" => json = true,
            other => return Err(format!("unknown plan option: {other}")),
        }
        i += 1;
    }
    let present = model_dir.as_deref().map(model_bytes_present).unwrap_or(0);
    let model_complete = if let Some(dir) = model_dir.as_deref() {
        let mut checks = Vec::new();
        validate_model(dir, &mut checks)
    } else {
        false
    };
    let plan = planner::build_plan(
        model_dir.as_deref(),
        model_complete,
        present,
        measure_io,
        benchmark_file.as_deref(),
    )?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&plan).map_err(|error| error.to_string())?
        );
    } else {
        planner::print_human(&plan);
    }
    Ok(if plan.blockers.is_empty() { 0 } else { 2 })
}

fn command_exists(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn has_arg(args: &[OsString], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

fn resolve_revision() -> Result<String, String> {
    let output = Command::new("hf")
        .args(["models", "info", MODEL_REPO])
        .output()
        .map_err(|e| format!("could not run `hf models info`: {e}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    parse_revision(&String::from_utf8_lossy(&output.stdout))
        .ok_or_else(|| "could not resolve the immutable model revision".to_string())
}

fn parse_revision(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let (key, value) = line.trim().split_once(':')?;
        if key.trim_matches(['"', '\'', ' ']) != "sha" {
            return None;
        }
        let sha = value.trim().trim_matches(['"', '\'', ',', ' ']);
        (sha.len() == 40 && sha.bytes().all(|b| b.is_ascii_hexdigit())).then(|| sha.to_string())
    })
}

fn pull(args: &[OsString]) -> Result<i32, String> {
    let mut destination = None;
    let mut revision = None;
    let mut workers = "16".to_string();
    let mut skip_checksum = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].to_string_lossy().as_ref() {
            "--destination" => {
                i += 1;
                destination = Some(PathBuf::from(
                    args.get(i).ok_or("--destination needs a path")?,
                ));
            }
            "--revision" => {
                i += 1;
                revision = Some(
                    args.get(i)
                        .ok_or("--revision needs a SHA")?
                        .to_string_lossy()
                        .to_string(),
                );
            }
            "--workers" => {
                i += 1;
                workers = args
                    .get(i)
                    .ok_or("--workers needs a number")?
                    .to_string_lossy()
                    .to_string();
                workers
                    .parse::<u16>()
                    .map_err(|_| "--workers must be a number")?;
            }
            "--skip-checksum" => skip_checksum = true,
            other => return Err(format!("unknown pull option: {other}")),
        }
        i += 1;
    }
    let destination = destination.ok_or("pull needs --destination PATH")?;
    fs::create_dir_all(&destination)
        .map_err(|e| format!("cannot create {}: {e}", destination.display()))?;
    let present = model_bytes_present(&destination);
    let remaining = EXPECTED_BYTES.saturating_sub(present);
    let required = remaining.saturating_add(DOWNLOAD_RESERVE_BYTES);
    let free =
        free_space_bytes(&destination).map_err(|e| format!("cannot read free space: {e}"))?;
    if free < required {
        return Err(format!(
            "not enough storage at {}: {} free, {} still required (download plus safety reserve)",
            destination.display(),
            human_bytes(free),
            human_bytes(required)
        ));
    }
    if !command_exists("hf") {
        return Err("the `hf` command is required. Install `huggingface_hub`; no login or API key is needed for this public model".to_string());
    }
    let revision = match revision {
        Some(value) if value.len() == 40 && value.bytes().all(|b| b.is_ascii_hexdigit()) => value,
        Some(_) => return Err("--revision must be a 40-character commit SHA".to_string()),
        None => resolve_revision()?,
    };
    println!(
        "Pulling {MODEL_REPO}@{revision} to {}",
        destination.display()
    );
    println!("The transfer resumes if interrupted. No API key is required.");
    let status = Command::new("hf")
        .env("HF_XET_HIGH_PERFORMANCE", "1")
        .args([
            "download",
            MODEL_REPO,
            "--revision",
            &revision,
            "--local-dir",
        ])
        .arg(&destination)
        .args(["--max-workers", &workers])
        .status()
        .map_err(|e| format!("could not start hf download: {e}"))?;
    if !status.success() {
        return Ok(status.code().unwrap_or(1));
    }
    let mut checks = Vec::new();
    if !validate_model(&destination, &mut checks) {
        let report = doctor_report(Some(&destination));
        print_report(&report, false)?;
        return Err("download finished but the checkpoint is incomplete or invalid".to_string());
    }
    if !skip_checksum {
        println!("Verifying every shard against Hub metadata; this re-reads the full checkpoint.");
        let status = Command::new("hf")
            .args([
                "cache",
                "verify",
                MODEL_REPO,
                "--revision",
                &revision,
                "--local-dir",
            ])
            .arg(&destination)
            .arg("--fail-on-missing-files")
            .status()
            .map_err(|e| format!("could not start checksum verification: {e}"))?;
        if !status.success() {
            return Ok(status.code().unwrap_or(1));
        }
    } else {
        println!("WARNING: Hub checksum verification was skipped; file sizes were still checked.");
    }
    println!("Model pull and verification complete.");
    Ok(0)
}

fn pack(args: &[OsString]) -> Result<i32, String> {
    let mut model_dir = None;
    let mut destination = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].to_string_lossy().as_ref() {
            "--model-dir" => {
                i += 1;
                model_dir = Some(PathBuf::from(
                    args.get(i).ok_or("--model-dir needs a path")?,
                ));
            }
            "--destination" => {
                i += 1;
                destination = Some(PathBuf::from(
                    args.get(i).ok_or("--destination needs a path")?,
                ));
            }
            other => return Err(format!("unknown pack option: {other}")),
        }
        i += 1;
    }
    let model_dir = model_dir.ok_or("pack needs --model-dir PATH")?;
    let destination = destination.ok_or("pack needs --destination PATH")?;
    let mut checks = Vec::new();
    if !validate_model(&model_dir, &mut checks) {
        return Err(
            "the source model is incomplete; run `xcaliber doctor --model-dir PATH`".to_string(),
        );
    }
    fs::create_dir_all(&destination)
        .map_err(|e| format!("cannot create {}: {e}", destination.display()))?;
    let free = free_space_bytes(&destination).map_err(|e| e.to_string())?;
    if free < 120_000_000_000 {
        return Err(format!(
            "packed trunk needs about 109 GB; only {} is free",
            human_bytes(free)
        ));
    }
    let script = pack_tool_path()
        .ok_or("tools/pack_trunk.py was not found next to the Xcaliber installation")?;
    let (python, prefix) = if command_exists("python") {
        ("python", Vec::<&str>::new())
    } else if command_exists("py") {
        ("py", vec!["-3"])
    } else {
        return Err("Python 3 is required for the one-time trunk pack".to_string());
    };
    let mut command = Command::new(python);
    command
        .args(prefix)
        .arg(script)
        .arg(&model_dir)
        .arg(&destination);
    let status = command
        .status()
        .map_err(|e| format!("could not start trunk pack: {e}"))?;
    Ok(status.code().unwrap_or(1))
}

fn run_native(args: &[OsString]) -> Result<i32, String> {
    let mut model_dir = env::var_os("K3_MODEL_DIR").map(PathBuf::from);
    let mut forwarded = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--model-dir" {
            i += 1;
            model_dir = Some(PathBuf::from(
                args.get(i).ok_or("--model-dir needs a path")?,
            ));
        } else {
            forwarded.push(args[i].clone());
        }
        i += 1;
    }
    let model_dir = model_dir.ok_or("run needs --model-dir PATH or K3_MODEL_DIR")?;
    let mut checks = Vec::new();
    if !validate_model(&model_dir, &mut checks) {
        return Err("model is incomplete; run `xcaliber doctor --model-dir PATH`".to_string());
    }
    let engine = engine_path()
        .ok_or("native engine not found; keep xcaliber-engine.exe next to xcaliber.exe")?;
    let trunk = forwarded
        .windows(2)
        .find(|pair| pair[0] == "--trunk")
        .map(|pair| PathBuf::from(&pair[1]))
        .or_else(|| env::var_os("K3_TRUNK_DIR").map(PathBuf::from));
    if total_memory_bytes() < 120_000_000_000 && trunk.is_none() {
        return Err("this machine needs a packed trunk for low-memory inference. Run `xcaliber pack`, then pass --trunk PATH".to_string());
    }
    if let Some(path) = trunk {
        if !has_arg(&forwarded, "--trunk") {
            forwarded.push("--trunk".into());
            forwarded.push(path.into_os_string());
        }
    }
    if !has_arg(&forwarded, "--preset") && !has_arg(&forwarded, "--trunk-gb") {
        forwarded.push("--preset".into());
        forwarded.push("laptop".into());
    }
    if !has_arg(&forwarded, "--incremental") {
        forwarded.push("--incremental".into());
    }
    if !has_arg(&forwarded, "--spec") {
        // Lossless n-gram drafts help repeated code/text and are always verified
        // by the exact model. Users can still disable them with `--spec 0`.
        forwarded.push("--spec".into());
        forwarded.push("4".into());
    }
    if (has_arg(&forwarded, "--prompt") || has_arg(&forwarded, "--prompt-file"))
        && !has_arg(&forwarded, "--tok")
    {
        forwarded.push("--tok".into());
        forwarded.push(model_dir.clone().into_os_string());
    }
    let status = Command::new(engine)
        .arg(model_dir)
        .args(forwarded)
        .status()
        .map_err(|e| format!("could not start native engine: {e}"))?;
    Ok(status.code().unwrap_or(1))
}

fn engine_path() -> Option<PathBuf> {
    if let Some(path) = env::var_os("XCALIBER_ENGINE").map(PathBuf::from) {
        if path.is_file() {
            return Some(path);
        }
    }
    let dir = env::current_exe().ok()?.parent()?.to_path_buf();
    let names: &[&str] = if cfg!(windows) {
        &["xcaliber-engine.exe", "k3.exe"]
    } else {
        &["xcaliber-engine", "k3"]
    };
    names
        .iter()
        .map(|name| dir.join(name))
        .find(|path| path.is_file())
}

fn pack_tool_path() -> Option<PathBuf> {
    if let Some(path) = env::var_os("XCALIBER_PACK_TOOL").map(PathBuf::from) {
        if path.is_file() {
            return Some(path);
        }
    }
    let exe = env::current_exe().ok()?;
    let dir = exe.parent()?;
    [
        dir.join("tools/pack_trunk.py"),
        dir.join("../tools/pack_trunk.py"),
    ]
    .into_iter()
    .find(|path| path.is_file())
}

fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit + 1 < UNITS.len() {
        value /= 1000.0;
        unit += 1;
    }
    format!("{value:.2} {}", UNITS[unit])
}

#[cfg(windows)]
#[repr(C)]
struct MemoryStatusEx {
    length: u32,
    memory_load: u32,
    total_phys: u64,
    avail_phys: u64,
    total_page_file: u64,
    avail_page_file: u64,
    total_virtual: u64,
    avail_virtual: u64,
    avail_extended_virtual: u64,
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn GlobalMemoryStatusEx(status: *mut MemoryStatusEx) -> i32;
    fn GetDiskFreeSpaceExW(
        path: *const u16,
        available: *mut u64,
        total: *mut u64,
        free: *mut u64,
    ) -> i32;
}

#[cfg(windows)]
fn total_memory_bytes() -> u64 {
    let mut status: MemoryStatusEx = unsafe { std::mem::zeroed() };
    status.length = std::mem::size_of::<MemoryStatusEx>() as u32;
    if unsafe { GlobalMemoryStatusEx(&mut status) } == 0 {
        0
    } else {
        status.total_phys
    }
}

#[cfg(windows)]
fn available_memory_bytes() -> u64 {
    let mut status: MemoryStatusEx = unsafe { std::mem::zeroed() };
    status.length = std::mem::size_of::<MemoryStatusEx>() as u32;
    // SAFETY: `status` points to a writable `MemoryStatusEx` with the required length set.
    if unsafe { GlobalMemoryStatusEx(&mut status) } == 0 {
        0
    } else {
        status.avail_phys
    }
}

#[cfg(not(windows))]
fn total_memory_bytes() -> u64 {
    fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|text| {
            text.lines().find_map(|line| {
                line.strip_prefix("MemTotal:")
                    .and_then(|tail| tail.split_whitespace().next())
                    .and_then(|value| value.parse::<u64>().ok())
            })
        })
        .map(|kb| kb.saturating_mul(1024))
        .unwrap_or(0)
}

#[cfg(not(windows))]
fn available_memory_bytes() -> u64 {
    fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|text| {
            text.lines().find_map(|line| {
                line.strip_prefix("MemAvailable:")
                    .and_then(|tail| tail.split_whitespace().next())
                    .and_then(|value| value.parse::<u64>().ok())
            })
        })
        .map(|kb| kb.saturating_mul(1024))
        .unwrap_or_else(total_memory_bytes)
}

fn existing_ancestor(path: &Path) -> io::Result<PathBuf> {
    let mut current = path.to_path_buf();
    loop {
        if current.exists() {
            return Ok(current);
        }
        if !current.pop() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "no existing parent path",
            ));
        }
    }
}

#[cfg(windows)]
fn free_space_bytes(path: &Path) -> io::Result<u64> {
    use std::os::windows::ffi::OsStrExt;
    let existing = existing_ancestor(path)?;
    let wide: Vec<u16> = existing.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut available = 0u64;
    let mut total = 0u64;
    let mut free = 0u64;
    if unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut available, &mut total, &mut free) } == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(available)
    }
}

#[cfg(unix)]
fn free_space_bytes(path: &Path) -> io::Result<u64> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let existing = existing_ancestor(path)?;
    let raw = CString::new(existing.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains NUL"))?;
    let mut stats: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(raw.as_ptr(), &mut stats) } != 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok((stats.f_bavail as u64).saturating_mul(stats.f_frsize as u64))
    }
}

fn run() -> Result<i32, String> {
    let mut args = env::args_os();
    let _program = args.next();
    let Some(command) = args.next() else {
        usage();
        return Ok(0);
    };
    let rest: Vec<OsString> = args.collect();
    match command.to_string_lossy().as_ref() {
        "help" | "--help" | "-h" => {
            usage();
            Ok(0)
        }
        "version" | "--version" | "-V" => {
            println!("xcaliber {VERSION}");
            Ok(0)
        }
        "requirements" => {
            requirements();
            Ok(0)
        }
        "doctor" => {
            let options = parse_common(&rest)?;
            let report = doctor_report(options.model_dir.as_deref());
            print_report(&report, options.json)?;
            Ok(if report.ready { 0 } else { 2 })
        }
        "plan" => performance_plan(&rest),
        "pull" => pull(&rest),
        "pack" => pack(&rest),
        "run" => run_native(&rest),
        "chat" => local_api::run(&rest),
        other => Err(format!("unknown command: {other}")),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code.clamp(0, 255) as u8),
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{human_bytes, parse_revision, shard_sizes, EXPECTED_BYTES, EXPECTED_SHARDS};

    #[test]
    fn published_shard_table_is_complete_and_totals_exactly() {
        let shards = shard_sizes();
        assert_eq!(shards.len(), EXPECTED_SHARDS);
        assert_eq!(shards.values().copied().sum::<u64>(), EXPECTED_BYTES);
    }

    #[test]
    fn revision_parser_accepts_only_a_full_sha() {
        assert_eq!(
            parse_revision("sha: 0123456789abcdef0123456789abcdef01234567"),
            Some("0123456789abcdef0123456789abcdef01234567".to_string())
        );
        assert_eq!(parse_revision("sha: main"), None);
    }

    #[test]
    fn byte_format_is_decimal_and_readable() {
        assert_eq!(human_bytes(1_560_936_091_448), "1.56 TB");
    }
}
