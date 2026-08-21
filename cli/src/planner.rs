// SPDX-License-Identifier: AGPL-3.0-only
//! Hardware-aware performance envelope for Kimi K3.
//!
//! This planner deliberately separates "can execute" from "can be interactive".
//! Kimi K3 activates 104B parameters per token, so capacity alone is not a speed claim.

use serde::Serialize;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

use crate::{available_memory_bytes, free_space_bytes, human_bytes, total_memory_bytes};

const MODEL_BYTES: u64 = 1_560_936_091_448;
const TRUNK_BYTES: u64 = 108_810_000_000;
const ACTIVE_EXPERT_BYTES: u64 = 25_830_000_000;
const EXACT_STREAM_BYTES_PER_TOKEN: u64 = TRUNK_BYTES + ACTIVE_EXPERT_BYTES;
const INTERACTIVE_TOKENS_PER_SECOND: f64 = 2.0;

#[derive(Debug, Serialize)]
pub struct GpuInfo {
    pub name: String,
    pub total_vram_bytes: u64,
    pub free_vram_bytes: u64,
    pub source: &'static str,
}

#[derive(Debug, Serialize)]
pub struct IoMeasurement {
    pub path: String,
    pub bytes_read: u64,
    pub seconds: f64,
    pub bytes_per_second: f64,
    pub qualification: &'static str,
}

#[derive(Debug, Serialize)]
pub struct HardwareProfile {
    pub logical_cpu_threads: usize,
    pub total_memory_bytes: u64,
    pub available_memory_bytes: u64,
    pub free_bytes_at_model_path: Option<u64>,
    pub gpus: Vec<GpuInfo>,
    pub docker_available: bool,
    pub observed_io: Option<IoMeasurement>,
}

#[derive(Debug, Serialize)]
pub struct ModePlan {
    pub id: &'static str,
    pub label: &'static str,
    pub preserves_official_semantics: bool,
    pub runnable_now: bool,
    pub interactive_target_met: bool,
    pub estimated_tokens_per_second_upper_bound: Option<f64>,
    pub minimum_stream_bandwidth_bytes_per_second: u64,
    pub summary: String,
    pub requirements: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct PerformancePlan {
    pub schema_version: u32,
    pub product: &'static str,
    pub model: &'static str,
    pub model_complete: bool,
    pub model_bytes_present: u64,
    pub expected_model_bytes: u64,
    pub target_tokens_per_second: f64,
    pub hardware: HardwareProfile,
    pub recommended_mode: &'static str,
    pub modes: Vec<ModePlan>,
    pub blockers: Vec<String>,
    pub conclusions: Vec<String>,
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

fn discover_nvidia() -> Vec<GpuInfo> {
    let Ok(output) = Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,memory.total,memory.free",
            "--format=csv,noheader,nounits",
        ])
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let fields: Vec<_> = line.split(',').map(str::trim).collect();
            if fields.len() != 3 {
                return None;
            }
            let total_mib = fields[1].parse::<u64>().ok()?;
            let free_mib = fields[2].parse::<u64>().ok()?;
            Some(GpuInfo {
                name: fields[0].to_string(),
                total_vram_bytes: total_mib.saturating_mul(1_048_576),
                free_vram_bytes: free_mib.saturating_mul(1_048_576),
                source: "nvidia-smi",
            })
        })
        .collect()
}

fn first_weight_file(model_dir: &Path) -> Option<PathBuf> {
    let mut files: Vec<_> = fs::read_dir(model_dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("model-") && name.ends_with(".safetensors"))
        })
        .collect();
    files.sort();
    files.into_iter().next()
}

fn measure_buffered_read(path: &Path) -> Result<IoMeasurement, String> {
    const SAMPLE_BYTES: u64 = 512 * 1024 * 1024;
    const BUFFER_BYTES: usize = 8 * 1024 * 1024;
    let mut file = File::open(path)
        .map_err(|error| format!("cannot open I/O sample {}: {error}", path.display()))?;
    let mut buffer = vec![0u8; BUFFER_BYTES];
    let start = Instant::now();
    let mut read_total = 0u64;
    while read_total < SAMPLE_BYTES {
        let remaining = (SAMPLE_BYTES - read_total).min(BUFFER_BYTES as u64) as usize;
        let count = file
            .read(&mut buffer[..remaining])
            .map_err(|error| format!("cannot read I/O sample {}: {error}", path.display()))?;
        if count == 0 {
            break;
        }
        read_total = read_total.saturating_add(count as u64);
    }
    if read_total == 0 {
        return Err(format!("I/O sample is empty: {}", path.display()));
    }
    let seconds = start.elapsed().as_secs_f64().max(0.000_001);
    Ok(IoMeasurement {
        path: path.display().to_string(),
        bytes_read: read_total,
        seconds,
        bytes_per_second: read_total as f64 / seconds,
        qualification: "observed buffered sequential read; cache and thermal state can change it",
    })
}

fn estimate_upper_bound(io: Option<&IoMeasurement>, bytes_per_token: u64) -> Option<f64> {
    io.map(|sample| sample.bytes_per_second / bytes_per_token as f64)
}

#[allow(clippy::too_many_arguments)]
pub fn build_plan(
    model_dir: Option<&Path>,
    model_complete: bool,
    model_bytes_present: u64,
    measure_io: bool,
    benchmark_file: Option<&Path>,
) -> Result<PerformancePlan, String> {
    let logical_cpu_threads = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1);
    let total_memory = total_memory_bytes();
    let available_memory = available_memory_bytes();
    let free_at_model = model_dir.and_then(|path| free_space_bytes(path).ok());
    let io_path = benchmark_file
        .map(Path::to_path_buf)
        .or_else(|| model_dir.and_then(first_weight_file));
    let observed_io = if measure_io {
        let path = io_path
            .ok_or("--measure-io needs --benchmark-file PATH or at least one model shard")?;
        Some(measure_buffered_read(&path)?)
    } else {
        None
    };
    let gpus = discover_nvidia();
    let docker_available = command_available("docker");
    let exact_upper = estimate_upper_bound(observed_io.as_ref(), EXACT_STREAM_BYTES_PER_TOKEN);
    let adaptive_upper = estimate_upper_bound(observed_io.as_ref(), ACTIVE_EXPERT_BYTES);
    let interactive_exact_bandwidth =
        (EXACT_STREAM_BYTES_PER_TOKEN as f64 * INTERACTIVE_TOKENS_PER_SECOND) as u64;
    let interactive_adaptive_bandwidth =
        (ACTIVE_EXPERT_BYTES as f64 * INTERACTIVE_TOKENS_PER_SECOND) as u64;

    let enough_internal_capacity = free_at_model
        .map(|free| free.saturating_add(model_bytes_present) >= MODEL_BYTES)
        .unwrap_or(false);
    let adaptive_memory_floor = 40_000_000_000u64;
    let adaptive_runnable =
        model_complete && docker_available && total_memory >= adaptive_memory_floor;
    let exact_interactive = exact_upper
        .is_some_and(|tokens_per_second| tokens_per_second >= INTERACTIVE_TOKENS_PER_SECOND);
    let adaptive_interactive = adaptive_upper
        .is_some_and(|tokens_per_second| tokens_per_second >= INTERACTIVE_TOKENS_PER_SECOND);

    let exact_summary = match exact_upper {
        Some(tps) => format!(
            "Observed storage gives at most {tps:.3} token/s before CPU work; the exact streamed engine reads about {} per token.",
            human_bytes(EXACT_STREAM_BYTES_PER_TOKEN)
        ),
        None => format!(
            "Execution is exact but streams about {} per token. Run with --measure-io on the model drive for a hardware upper bound.",
            human_bytes(EXACT_STREAM_BYTES_PER_TOKEN)
        ),
    };
    let adaptive_summary = match adaptive_upper {
        Some(tps) => format!(
            "With the quantized dense path resident, expert streaming alone caps decode near {tps:.3} token/s before CPU work and cache misses. This mode changes numerical precision.",
        ),
        None => format!(
            "Keeps a quantized dense path resident and streams up to {} of experts per token. Faster than exact trunk streaming, but not official-bit-exact.",
            human_bytes(ACTIVE_EXPERT_BYTES)
        ),
    };

    let mut blockers = Vec::new();
    if !model_complete {
        blockers.push("the complete official 96-shard checkpoint is not present".to_string());
    }
    if !enough_internal_capacity && !model_complete {
        blockers.push(format!(
            "the selected model drive cannot currently hold the {} checkpoint",
            human_bytes(MODEL_BYTES)
        ));
    }
    if total_memory < adaptive_memory_floor {
        blockers.push(format!(
            "adaptive dense-resident mode needs about 40 GB RAM; this host reports {}",
            human_bytes(total_memory)
        ));
    }
    if !docker_available {
        blockers.push(
            "Docker is not installed, so the persistent adaptive/API engine cannot start"
                .to_string(),
        );
    }

    let recommended_mode = if adaptive_runnable {
        "adaptive-local"
    } else if model_complete {
        "exact-streamed"
    } else {
        "smaller-surrogate"
    };
    Ok(PerformancePlan {
        schema_version: 1,
        product: "xcaliber",
        model: "moonshotai/Kimi-K3",
        model_complete,
        model_bytes_present,
        expected_model_bytes: MODEL_BYTES,
        target_tokens_per_second: INTERACTIVE_TOKENS_PER_SECOND,
        hardware: HardwareProfile {
            logical_cpu_threads,
            total_memory_bytes: total_memory,
            available_memory_bytes: available_memory,
            free_bytes_at_model_path: free_at_model,
            gpus,
            docker_available,
            observed_io,
        },
        recommended_mode,
        modes: vec![
            ModePlan {
                id: "exact-streamed",
                label: "Exact official K3",
                preserves_official_semantics: true,
                runnable_now: model_complete,
                interactive_target_met: exact_interactive,
                estimated_tokens_per_second_upper_bound: exact_upper,
                minimum_stream_bandwidth_bytes_per_second: interactive_exact_bandwidth,
                summary: exact_summary,
                requirements: vec![
                    "complete 1.56 TB checkpoint".to_string(),
                    "about 109 GB packed trunk in addition to the checkpoint".to_string(),
                    format!(
                        "at least {} effective streaming bandwidth for 2 token/s before compute",
                        human_bytes(interactive_exact_bandwidth)
                    ),
                ],
            },
            ModePlan {
                id: "adaptive-local",
                label: "Adaptive K3 architecture",
                preserves_official_semantics: false,
                runnable_now: adaptive_runnable,
                interactive_target_met: adaptive_interactive,
                estimated_tokens_per_second_upper_bound: adaptive_upper,
                minimum_stream_bandwidth_bytes_per_second: interactive_adaptive_bandwidth,
                summary: adaptive_summary,
                requirements: vec![
                    "complete checkpoint and Docker/Colibri runtime".to_string(),
                    "roughly 40 GB RAM minimum for the quantized dense path and runtime".to_string(),
                    "optional Vulkan hot-expert tier; small VRAM cannot hold the whole model".to_string(),
                ],
            },
            ModePlan {
                id: "smaller-surrogate",
                label: "Practical smaller local model",
                preserves_official_semantics: false,
                runnable_now: false,
                interactive_target_met: false,
                estimated_tokens_per_second_upper_bound: None,
                minimum_stream_bandwidth_bytes_per_second: 0,
                summary: "A distilled or smaller model is the only credible interactive path on a 32 GB laptop. Xcaliber does not mislabel such a model as official Kimi K3.".to_string(),
                requirements: vec![
                    "a separately licensed smaller checkpoint".to_string(),
                    "a validated tokenizer/chat compatibility layer".to_string(),
                ],
            },
            ModePlan {
                id: "heterogeneous-cluster",
                label: "Pool several ordinary systems",
                preserves_official_semantics: true,
                runnable_now: false,
                interactive_target_met: false,
                estimated_tokens_per_second_upper_bound: None,
                minimum_stream_bandwidth_bytes_per_second: 0,
                summary: "Expert and layer sharding can aggregate RAM, SSD bandwidth, and GPUs, but K3 expert workers and failover are not implemented in this release.".to_string(),
                requirements: vec![
                    "enough aggregate storage for the checkpoint".to_string(),
                    "fast stable LAN and topology-aware placement".to_string(),
                    "K3-specific distributed correctness tests".to_string(),
                ],
            },
        ],
        blockers,
        conclusions: vec![
            "Sparse activation reduces arithmetic, not checkpoint capacity.".to_string(),
            "Caching and prefetching help only when expert locality produces real cache hits.".to_string(),
            "Speculative decoding is lossless only when every accepted token is verified by the exact model; its draft must also be much cheaper than verification.".to_string(),
            "No software setting can turn a 6 GB GPU and 32 GB RAM into hundreds of GB/s of effective model bandwidth.".to_string(),
        ],
    })
}

pub fn print_human(plan: &PerformancePlan) {
    println!("Xcaliber performance plan");
    println!("  CPU threads : {}", plan.hardware.logical_cpu_threads);
    println!(
        "  RAM         : {} total / {} available",
        human_bytes(plan.hardware.total_memory_bytes),
        human_bytes(plan.hardware.available_memory_bytes)
    );
    if let Some(free) = plan.hardware.free_bytes_at_model_path {
        println!("  model drive : {} free", human_bytes(free));
    }
    for gpu in &plan.hardware.gpus {
        println!(
            "  GPU         : {} / {} VRAM free",
            gpu.name,
            human_bytes(gpu.free_vram_bytes)
        );
    }
    if let Some(io) = &plan.hardware.observed_io {
        println!(
            "  observed I/O: {}/s ({})",
            human_bytes(io.bytes_per_second as u64),
            io.qualification
        );
    }
    println!("  recommended : {}", plan.recommended_mode);
    println!();
    for mode in &plan.modes {
        let exact = if mode.preserves_official_semantics {
            "exact"
        } else {
            "not exact"
        };
        println!("{} [{}]", mode.label, exact);
        println!("  {}", mode.summary);
        println!(
            "  runnable now: {} | 2 token/s target: {}",
            if mode.runnable_now { "yes" } else { "no" },
            if mode.interactive_target_met {
                "met by observed I/O upper bound"
            } else {
                "not demonstrated"
            }
        );
    }
    if !plan.blockers.is_empty() {
        println!("\nBlockers:");
        for blocker in &plan.blockers {
            println!("  - {blocker}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        estimate_upper_bound, IoMeasurement, ACTIVE_EXPERT_BYTES, EXACT_STREAM_BYTES_PER_TOKEN,
    };

    #[test]
    fn bandwidth_bound_uses_bytes_per_token() {
        let io = IoMeasurement {
            path: "sample".to_string(),
            bytes_read: 1,
            seconds: 1.0,
            bytes_per_second: EXACT_STREAM_BYTES_PER_TOKEN as f64 * 2.0,
            qualification: "test",
        };
        assert_eq!(
            estimate_upper_bound(Some(&io), EXACT_STREAM_BYTES_PER_TOKEN),
            Some(2.0)
        );
        assert!(
            estimate_upper_bound(Some(&io), ACTIVE_EXPERT_BYTES).is_some_and(|value| value > 2.0)
        );
    }
}
