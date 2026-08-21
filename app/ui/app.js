const invoke = window.__TAURI__?.core?.invoke;
const $ = (id) => document.getElementById(id);
let plan = null;
let messages = [];

const titles = {
  overview: "System overview",
  setup: "Model setup",
  chat: "Local chat",
  limits: "Performance truth",
};

function formatBytes(value) {
  if (!Number.isFinite(value)) return "—";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let n = value;
  let i = 0;
  while (n >= 1000 && i < units.length - 1) { n /= 1000; i += 1; }
  return `${n.toFixed(i >= 3 ? 2 : 1)} ${units[i]}`;
}

function saved(id) {
  const element = $(id);
  const value = localStorage.getItem(`xcaliber:${id}`);
  if (value !== null) element.value = value;
  element.addEventListener("change", () => localStorage.setItem(`xcaliber:${id}`, element.value));
}

for (const id of ["model-dir", "trunk-dir", "destination", "benchmark-file", "api-url", "api-model"]) saved(id);

document.querySelectorAll(".nav").forEach((button) => {
  button.addEventListener("click", () => {
    document.querySelectorAll(".nav, .view").forEach((element) => element.classList.remove("active"));
    button.classList.add("active");
    $(button.dataset.view).classList.add("active");
    $("view-title").textContent = titles[button.dataset.view];
  });
});

function openView(view) {
  const button = document.querySelector(`.nav[data-view="${view}"]`);
  if (button) button.click();
}

const onboarding = $("onboarding");
onboarding.hidden = localStorage.getItem("xcaliber:onboarding-dismissed") === "1";
$("onboarding-close").addEventListener("click", () => {
  localStorage.setItem("xcaliber:onboarding-dismissed", "1");
  onboarding.hidden = true;
});
$("onboarding-chat").addEventListener("click", () => openView("chat"));
$("open-local-chat").addEventListener("click", () => openView("chat"));

function request(action, overrides = {}) {
  return {
    action,
    modelDir: $("model-dir").value.trim() || null,
    destination: $("destination").value.trim() || null,
    trunkDir: $("trunk-dir").value.trim() || null,
    prompt: $("chat-input").value.trim() || null,
    benchmarkFile: $("benchmark-file").value.trim() || null,
    measureIo: $("measure-io").checked,
    maxTokens: Number($("max-tokens").value || 32),
    ...overrides,
  };
}

function setBusy(busy, label = "Idle") {
  document.querySelectorAll("button").forEach((button) => {
    if (!button.classList.contains("nav")) button.disabled = busy;
  });
  $("cancel").disabled = !busy;
  $("task-state").textContent = label;
  $("setup-state").textContent = label;
}

async function execute(action, overrides = {}) {
  if (!invoke) throw new Error("Tauri runtime is unavailable; open this interface through Xcaliber.");
  setBusy(true, `Running ${action}…`);
  try {
    return await invoke("execute", { request: request(action, overrides) });
  } finally {
    setBusy(false);
  }
}

function renderPlan(value) {
  plan = value;
  const hardware = value.hardware;
  $("ram-stat").textContent = formatBytes(hardware.total_memory_bytes);
  $("ram-available").textContent = `${formatBytes(hardware.available_memory_bytes)} currently available`;
  const gpu = hardware.gpus[0];
  $("gpu-stat").textContent = gpu ? gpu.name : "No qualified GPU";
  $("gpu-detail").textContent = gpu ? `${formatBytes(gpu.free_vram_bytes)} VRAM free` : "CPU path remains available";
  $("disk-stat").textContent = formatBytes(hardware.free_bytes_at_model_path);
  $("mode-stat").textContent = value.recommended_mode;
  $("mode-detail").textContent = `${value.target_tokens_per_second} token/s target`;
  $("mode-grid").innerHTML = value.modes.map((mode) => `
    <article class="mode-card ${mode.id === value.recommended_mode ? "recommended" : ""}">
      <div class="mode-head"><h3>${mode.label}</h3><span class="pill">${mode.preserves_official_semantics ? "EXACT" : "ADAPTIVE"}</span></div>
      <p>${mode.summary}</p>
    </article>`).join("");
  const output = [
    `model complete: ${value.model_complete}`,
    `model present: ${formatBytes(value.model_bytes_present)} / ${formatBytes(value.expected_model_bytes)}`,
    `docker available: ${hardware.docker_available}`,
    "",
    "blockers:",
    ...(value.blockers.length ? value.blockers.map((item) => `- ${item}`) : ["- none"]),
  ].join("\n");
  $("overview-output").textContent = output;
  $("limits-output").textContent = value.conclusions.map((item) => `- ${item}`).join("\n");
}

async function refreshPlan() {
  try {
    const result = await execute("plan");
    renderPlan(JSON.parse(result.stdout));
  } catch (error) {
    $("overview-output").textContent = String(error);
  }
}

$("onboarding-check").addEventListener("click", refreshPlan);

async function refreshRuntime() {
  if (!invoke) {
    $("runtime-label").textContent = "Browser preview only";
    $("runtime-path").textContent = "Open through the Tauri application";
    return;
  }
  try {
    const info = await invoke("runtime_info");
    $("runtime-label").textContent = info.cliAvailable ? "CLI connected" : "CLI missing";
    $("runtime-path").textContent = info.cliPath || "Place xcaliber.exe next to the app";
    $("app-version").textContent = `Xcaliber ${info.appVersion}`;
    $("runtime-dot").classList.toggle("ready", info.cliAvailable);
  } catch (error) {
    $("runtime-label").textContent = "Runtime error";
    $("runtime-path").textContent = String(error);
  }
}

function showResult(result, target = "setup-output") {
  const chunks = [];
  if (result.stdout) chunks.push(result.stdout.trim());
  if (result.stderr) chunks.push(result.stderr.trim());
  $(target).textContent = chunks.join("\n\n") || `Finished with exit code ${result.exitCode}`;
}

$("refresh").addEventListener("click", refreshPlan);
$("plan").addEventListener("click", refreshPlan);
$("doctor").addEventListener("click", async () => {
  try { showResult(await execute("doctor")); } catch (error) { $("setup-output").textContent = String(error); }
});
$("pull").addEventListener("click", async () => {
  if (!confirm("This requests the complete 1.56 TB checkpoint. Continue only if the selected drive has enough free space.")) return;
  try { showResult(await execute("pull")); } catch (error) { $("setup-output").textContent = String(error); }
});
$("pack").addEventListener("click", async () => {
  try { showResult(await execute("pack")); } catch (error) { $("setup-output").textContent = String(error); }
});
$("cancel").addEventListener("click", async () => {
  try { $("setup-output").textContent = await invoke("cancel_active"); } catch (error) { $("setup-output").textContent = String(error); }
});

function appendMessage(role, content, reasoning = "") {
  document.querySelector(".empty-chat")?.remove();
  const node = document.createElement("div");
  node.className = `message ${role}`;
  node.textContent = content;
  if (reasoning) {
    const detail = document.createElement("details");
    detail.className = "reasoning";
    const summary = document.createElement("summary");
    summary.textContent = "Reasoning";
    const body = document.createElement("div");
    body.textContent = reasoning;
    detail.append(summary, body);
    node.append(detail);
  }
  $("messages").append(node);
  $("messages").scrollTop = $("messages").scrollHeight;
}

async function probeApi() {
  const base = $("api-url").value.replace(/\/$/, "");
  const response = await fetch(`${base}/health`, { signal: AbortSignal.timeout(5000) });
  if (!response.ok) throw new Error(`Local service returned HTTP ${response.status}`);
  $("api-state").textContent = "ONLINE";
  $("api-state").classList.add("safe");
}

$("probe-api").addEventListener("click", async () => {
  try { await probeApi(); } catch (error) { $("api-state").textContent = "OFFLINE"; alert(String(error)); }
});

for (const [id, action] of [
  ["docker-start", "docker_start"],
  ["docker-stop", "docker_stop"],
  ["docker-status", "docker_status"],
  ["docker-logs", "docker_logs"],
]) {
  $(id).addEventListener("click", async () => {
    try {
      showResult(await execute(action), "overview-output");
      if (action === "docker_start") await probeApi().catch(() => {});
    } catch (error) {
      $("overview-output").textContent = String(error);
    }
  });
}

$("chat-form").addEventListener("submit", async (event) => {
  event.preventDefault();
  const input = $("chat-input").value.trim();
  if (!input) return;
  const user = { role: "user", content: input };
  messages.push(user);
  appendMessage("user", input);
  $("chat-input").value = "";
  try {
    const base = $("api-url").value.replace(/\/$/, "");
    const response = await fetch(`${base}/v1/chat/completions`, {
      method: "POST",
      headers: { "Content-Type": "application/json", "Authorization": `Bearer ${$("api-key").value}` },
      body: JSON.stringify({
        model: $("api-model").value,
        messages,
        stream: false,
        max_tokens: Number($("max-tokens").value || 32),
        reasoning_effort: $("reasoning-effort").value,
      }),
    });
    if (!response.ok) throw new Error(`Local service returned HTTP ${response.status}: ${await response.text()}`);
    const data = await response.json();
    const assistant = data.choices?.[0]?.message;
    if (!assistant) throw new Error("Local service response has no assistant message");
    messages.push(assistant);
    appendMessage("assistant", assistant.content || "", assistant.reasoning_content || assistant.reasoning || "");
    $("api-state").textContent = "ONLINE";
    $("api-state").classList.add("safe");
  } catch (error) {
    appendMessage("assistant", `Connection error: ${error}`);
  }
});

$("native-run").addEventListener("click", async () => {
  try {
    const result = await execute("run");
    showResult(result, "overview-output");
    appendMessage("assistant", result.stdout || result.stderr);
  } catch (error) {
    appendMessage("assistant", String(error));
  }
});

setBusy(false);
refreshRuntime();
refreshPlan();
