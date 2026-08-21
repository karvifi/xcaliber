import {
  STORAGE_KEY,
  conversationMarkdown,
  defaultDesktopState,
  escapeHtml,
  estimateTokens,
  formatBytes,
  isLoopbackUrl,
  makeId,
  mergeDesktopState,
  metricsSummary,
  normaliseBaseUrl,
  safeJsonParse,
} from "./core.mjs";

const invoke = window.__TAURI__?.core?.invoke;
const $ = (id) => document.getElementById(id);
const viewMeta = {
  overview: ["Local systems", "System overview", "Measure first, then choose a runtime that fits."],
  models: ["Model library", "Models", "Manage honest local profiles without uploading model data."],
  playground: ["Private inference", "Playground", "Chat with the selected loopback model and keep the history local."],
  runtime: ["Operations", "Runtime", "Prepare K3 files and control the fixed local service."],
  monitor: ["Observability", "API monitor", "Inspect requests made by this desktop application."],
  activity: ["Operations", "Activity", "Review CLI and Docker jobs started from this workspace."],
  exports: ["Portability", "Export", "Download conversations, profiles and diagnostic evidence."],
  settings: ["System", "Settings", "Control appearance, privacy and local application data."],
};

let state = mergeDesktopState(safeJsonParse(localStorage.getItem(STORAGE_KEY), null));
let runtimeInfo = null;
let pullArmed = false;
let resetArmed = false;
let newChatArmed = false;
let profileDeleteArmed = null;

function persist() {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
}

function activeWorkspace() {
  return state.workspaces.find((item) => item.id === state.activeWorkspaceId) ?? state.workspaces[0];
}

function activeProfile() {
  const workspace = activeWorkspace();
  return state.profiles.find((item) => item.id === workspace?.profileId)
    ?? state.profiles.find((item) => item.id === state.activeProfileId)
    ?? state.profiles[0];
}

function activeMessages() {
  const id = activeWorkspace().id;
  if (!Array.isArray(state.chats[id])) state.chats[id] = [];
  return state.chats[id];
}

function setActiveMessages(messages) {
  state.chats[activeWorkspace().id] = messages.slice(-200);
  persist();
}

function toast(message, tone = "default") {
  const node = document.createElement("div");
  node.className = `toast ${tone}`;
  node.textContent = message;
  $("toast-region").append(node);
  window.setTimeout(() => node.remove(), 4200);
}

function openView(view, updateLocation = true) {
  if (!viewMeta[view]) return;
  document.querySelectorAll(".nav, .view").forEach((element) => element.classList.remove("active"));
  document.querySelector(`.nav[data-view="${view}"]`)?.classList.add("active");
  $(view).classList.add("active");
  const [eyebrow, title, subtitle] = viewMeta[view];
  $("view-eyebrow").textContent = eyebrow;
  $("view-title").textContent = title;
  $("view-subtitle").textContent = subtitle;
  if (updateLocation && window.location.hash !== `#${view}`) history.replaceState(null, "", `#${view}`);
}

document.querySelectorAll(".nav").forEach((button) => button.addEventListener("click", () => openView(button.dataset.view)));
document.querySelectorAll(".route-button").forEach((button) => button.addEventListener("click", () => openView(button.dataset.target)));
window.addEventListener("hashchange", () => openView(window.location.hash.slice(1), false));

function applyAppearance() {
  const { theme, density, reducedMotion } = state.settings;
  let effectiveTheme = theme;
  if (theme === "system") effectiveTheme = matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
  document.documentElement.dataset.theme = effectiveTheme;
  document.documentElement.dataset.density = density;
  document.documentElement.classList.toggle("reduce-motion", Boolean(reducedMotion));
  $("theme-setting").value = theme;
  $("density-setting").value = density;
  $("motion-setting").checked = Boolean(reducedMotion);
}

function renderWorkspaceOptions() {
  $("workspace-select").innerHTML = state.workspaces
    .map((item) => `<option value="${escapeHtml(item.id)}">${escapeHtml(item.name)}</option>`)
    .join("");
  $("workspace-select").value = state.activeWorkspaceId;
  renderWorkspaceSummary();
}

function renderWorkspaceSummary() {
  const workspace = activeWorkspace();
  const profile = activeProfile();
  const messages = activeMessages();
  const requests = state.metrics.filter((item) => item.workspaceId === workspace.id).length;
  $("workspace-summary-name").textContent = workspace.name;
  $("workspace-message-count").textContent = `${messages.length} message${messages.length === 1 ? "" : "s"}`;
  $("workspace-profile").textContent = profile.name;
  $("workspace-endpoint").textContent = normaliseBaseUrl(profile.apiUrl).replace(/^http:\/\//, "");
  $("workspace-requests").textContent = String(requests);
}

function showWorkspaceEditor(show) {
  $("workspace-creator").hidden = !show;
  $("workspace-error").textContent = "";
  if (show) {
    openView("overview");
    $("workspace-name").focus();
  }
}

$("workspace-add").addEventListener("click", () => showWorkspaceEditor(true));
$("workspace-cancel").addEventListener("click", () => showWorkspaceEditor(false));
$("workspace-save").addEventListener("click", () => {
  const name = $("workspace-name").value.trim();
  if (!name) {
    $("workspace-error").textContent = "Enter a workspace name.";
    return;
  }
  const id = makeId("workspace");
  state.workspaces.push({
    id,
    name,
    description: $("workspace-description").value.trim(),
    profileId: activeProfile().id,
    createdAt: new Date().toISOString(),
  });
  state.chats[id] = [];
  state.activeWorkspaceId = id;
  persist();
  $("workspace-name").value = "";
  $("workspace-description").value = "";
  showWorkspaceEditor(false);
  renderAll();
  toast(`Workspace “${name}” created.`, "success");
});
$("workspace-select").addEventListener("change", (event) => {
  state.activeWorkspaceId = event.target.value;
  state.activeProfileId = activeProfile().id;
  persist();
  syncProfileControls();
  renderAll();
});

function showProfileEditor(show) {
  $("profile-editor").hidden = !show;
  $("profile-error").textContent = "";
  if (show) $("profile-name").focus();
}

function renderProfiles() {
  const selected = activeProfile().id;
  $("profile-grid").innerHTML = state.profiles.map((profile) => {
    const exact = profile.kind === "xcaliber";
    const active = profile.id === selected;
    return `<article class="surface profile-card ${active ? "selected" : ""}">
      <div class="profile-head"><span class="profile-monogram">${escapeHtml(profile.name.slice(0, 2).toUpperCase())}</span><span class="flag ${exact ? "safe" : ""}">${exact ? "XCALIBER K3" : "COMPATIBLE"}</span></div>
      <h2>${escapeHtml(profile.name)}</h2>
      <p>${escapeHtml(profile.modelId)}</p>
      <dl><div><dt>Endpoint</dt><dd>${escapeHtml(normaliseBaseUrl(profile.apiUrl))}</dd></div><div><dt>Storage</dt><dd>${profile.modelDir ? escapeHtml(profile.modelDir) : "Managed outside Xcaliber"}</dd></div></dl>
      <div class="profile-actions"><button class="button ${active ? "primary" : ""}" data-profile-use="${escapeHtml(profile.id)}">${active ? "Selected" : "Use profile"}</button>${state.profiles.length > 1 ? `<button class="text-button danger-text" data-profile-delete="${escapeHtml(profile.id)}">${profileDeleteArmed === profile.id ? "Confirm remove" : "Remove"}</button>` : ""}</div>
    </article>`;
  }).join("");

  const options = state.profiles.map((profile) => `<option value="${escapeHtml(profile.id)}">${escapeHtml(profile.name)}</option>`).join("");
  $("chat-profile").innerHTML = options;
  $("chat-profile").value = selected;
}

function selectProfile(id) {
  const profile = state.profiles.find((item) => item.id === id);
  if (!profile) return;
  state.activeProfileId = id;
  activeWorkspace().profileId = id;
  persist();
  syncProfileControls();
  renderAll();
}

$("profile-grid").addEventListener("click", (event) => {
  const use = event.target.closest("[data-profile-use]");
  if (use) {
    selectProfile(use.dataset.profileUse);
    toast("Model profile selected.", "success");
    return;
  }
  const remove = event.target.closest("[data-profile-delete]");
  if (!remove) return;
  const id = remove.dataset.profileDelete;
  if (profileDeleteArmed !== id) {
    profileDeleteArmed = id;
    renderProfiles();
    toast("Press “Confirm remove” to delete this profile.");
    window.setTimeout(() => {
      if (profileDeleteArmed === id) {
        profileDeleteArmed = null;
        renderProfiles();
      }
    }, 8000);
    return;
  }
  profileDeleteArmed = null;
  state.profiles = state.profiles.filter((item) => item.id !== id);
  for (const workspace of state.workspaces) if (workspace.profileId === id) workspace.profileId = state.profiles[0].id;
  state.activeProfileId = state.profiles[0].id;
  persist();
  syncProfileControls();
  renderAll();
  toast("Model profile removed.");
});

$("profile-toggle").addEventListener("click", () => showProfileEditor(true));
$("profile-cancel").addEventListener("click", () => showProfileEditor(false));
$("onboarding-model").addEventListener("click", () => { openView("models"); showProfileEditor(true); });
$("profile-save").addEventListener("click", () => {
  const name = $("profile-name").value.trim();
  const apiUrl = normaliseBaseUrl($("profile-api-url").value);
  const modelId = $("profile-model-id").value.trim();
  if (!name || !modelId) {
    $("profile-error").textContent = "Profile name and model identifier are required.";
    return;
  }
  if (!isLoopbackUrl(apiUrl)) {
    $("profile-error").textContent = "Use an HTTP loopback URL such as http://127.0.0.1:8000.";
    return;
  }
  const profile = {
    id: makeId("profile"),
    name,
    kind: $("profile-kind").value,
    apiUrl,
    modelId,
    modelDir: $("profile-model-dir").value.trim(),
    trunkDir: $("profile-trunk-dir").value.trim(),
    destination: "",
  };
  state.profiles.push(profile);
  activeWorkspace().profileId = profile.id;
  state.activeProfileId = profile.id;
  persist();
  for (const id of ["profile-name", "profile-model-dir", "profile-trunk-dir"]) $(id).value = "";
  showProfileEditor(false);
  syncProfileControls();
  renderAll();
  toast(`Profile “${name}” saved.`, "success");
});
$("chat-profile").addEventListener("change", (event) => selectProfile(event.target.value));

function syncProfileControls() {
  const profile = activeProfile();
  $("model-dir").value = profile.modelDir ?? "";
  $("trunk-dir").value = profile.trunkDir ?? "";
  $("destination").value = profile.destination ?? profile.modelDir ?? "";
  $("chat-model-label").textContent = profile.name;
  $("chat-endpoint-label").textContent = profile.apiUrl;
  $("composer-model").textContent = profile.modelId;
  $("max-tokens").value = state.settings.maxTokens;
  $("temperature").value = state.settings.temperature;
  $("top-p").value = state.settings.topP;
  $("reasoning-effort").value = state.settings.reasoningEffort;
  $("api-state").textContent = "OFFLINE";
  $("api-state").classList.remove("safe", "error");
}

for (const [inputId, field] of [["model-dir", "modelDir"], ["trunk-dir", "trunkDir"], ["destination", "destination"]]) {
  $(inputId).addEventListener("change", (event) => {
    activeProfile()[field] = event.target.value.trim();
    persist();
    renderProfiles();
  });
}

function operationRequest(action, overrides = {}) {
  return {
    action,
    modelDir: $("model-dir").value.trim() || null,
    destination: $("destination").value.trim() || null,
    trunkDir: $("trunk-dir").value.trim() || null,
    prompt: $("chat-input").value.trim() || null,
    benchmarkFile: $("benchmark-file").value.trim() || null,
    measureIo: $("measure-io").checked,
    maxTokens: Number($("max-tokens").value || 256),
    ...overrides,
  };
}

function setBusy(busy, label = "Idle") {
  document.querySelectorAll("button").forEach((button) => {
    if (!button.classList.contains("nav") && button.id !== "cancel") button.disabled = busy;
  });
  $("cancel").disabled = !busy;
  $("task-state").textContent = label;
  $("runtime-state").textContent = label;
}

function compactSummary(result) {
  const text = result?.stderr || result?.stdout || "";
  return text.trim().split(/\r?\n/)[0]?.slice(0, 180) || `Exit ${result?.exitCode ?? "unknown"}`;
}

async function execute(action, overrides = {}) {
  const job = { id: makeId("job"), workspaceId: activeWorkspace().id, action, startedAt: new Date().toISOString(), status: "running", exitCode: null, summary: "Running" };
  state.jobs.unshift(job);
  state.jobs = state.jobs.slice(0, 100);
  persist();
  renderActivity();
  setBusy(true, `Running ${action}…`);
  try {
    if (!invoke) throw new Error("Open this interface through the Xcaliber Windows application to run local operations.");
    const result = await invoke("execute", { request: operationRequest(action, overrides) });
    job.status = result.success ? "completed" : "error";
    job.exitCode = result.exitCode;
    job.summary = compactSummary(result);
    return result;
  } catch (error) {
    job.status = "error";
    job.summary = String(error).slice(0, 180);
    throw error;
  } finally {
    persist();
    renderActivity();
    setBusy(false);
  }
}

function showResult(result, target = "runtime-output") {
  const chunks = [];
  if (result.stdout) chunks.push(result.stdout.trim());
  if (result.stderr) chunks.push(result.stderr.trim());
  $(target).textContent = chunks.join("\n\n") || `Finished with exit code ${result.exitCode}`;
}

function renderPlan(plan) {
  state.lastPlan = plan;
  persist();
  const hardware = plan.hardware;
  $("ram-stat").textContent = formatBytes(hardware.total_memory_bytes);
  $("ram-available").textContent = `${formatBytes(hardware.available_memory_bytes)} available`;
  const gpu = hardware.gpus?.[0];
  $("gpu-stat").textContent = gpu ? gpu.name : "CPU only";
  $("gpu-detail").textContent = gpu ? `${formatBytes(gpu.free_vram_bytes)} VRAM free` : "No qualified GPU detected";
  $("disk-stat").textContent = formatBytes(hardware.free_bytes_at_model_path);
  $("mode-stat").textContent = plan.recommended_mode;
  $("mode-detail").textContent = `${plan.target_tokens_per_second} token/s target`;
  $("mode-grid").innerHTML = plan.modes.map((mode) => `<article class="mode-card ${mode.id === plan.recommended_mode ? "recommended" : ""}"><div><h3>${escapeHtml(mode.label)}</h3><span class="flag">${mode.preserves_official_semantics ? "EXACT" : "ADAPTIVE"}</span></div><p>${escapeHtml(mode.summary)}</p></article>`).join("");
  $("overview-output").textContent = [
    `model complete: ${plan.model_complete}`,
    `model present: ${formatBytes(plan.model_bytes_present)} / ${formatBytes(plan.expected_model_bytes)}`,
    `docker available: ${hardware.docker_available}`,
    "",
    "blockers:",
    ...(plan.blockers.length ? plan.blockers.map((item) => `- ${item}`) : ["- none"]),
  ].join("\n");
}

async function refreshPlan() {
  try {
    const result = await execute("plan");
    const plan = safeJsonParse(result.stdout, null);
    if (!plan?.hardware) throw new Error("The planner returned an invalid response.");
    renderPlan(plan);
    toast("System plan refreshed.", "success");
  } catch (error) {
    $("overview-output").textContent = String(error);
    toast("System plan could not be refreshed.", "error");
  }
}

$("refresh").addEventListener("click", refreshPlan);
$("plan").addEventListener("click", refreshPlan);
$("onboarding-check").addEventListener("click", refreshPlan);
$("doctor").addEventListener("click", async () => {
  try { showResult(await execute("doctor")); } catch (error) { $("runtime-output").textContent = String(error); }
});
$("pack").addEventListener("click", async () => {
  try { showResult(await execute("pack")); } catch (error) { $("runtime-output").textContent = String(error); }
});
$("pull").addEventListener("click", async () => {
  if (!pullArmed) {
    pullArmed = true;
    $("pull").textContent = "Confirm 1.56 TB model pull";
    $("pull-warning").classList.add("armed");
    window.setTimeout(() => { pullArmed = false; $("pull").textContent = "Prepare 1.56 TB download"; $("pull-warning").classList.remove("armed"); }, 10000);
    return;
  }
  pullArmed = false;
  try { showResult(await execute("pull")); } catch (error) { $("runtime-output").textContent = String(error); }
});

for (const [id, action] of [["docker-start", "docker_start"], ["docker-stop", "docker_stop"], ["docker-status", "docker_status"], ["docker-logs", "docker_logs"]]) {
  $(id).addEventListener("click", async () => {
    try {
      const result = await execute(action);
      showResult(result);
      $("docker-label").textContent = action === "docker_stop" ? "Stopped" : result.success ? "Available" : "Needs attention";
      $("docker-dot").classList.toggle("ready", result.success && action !== "docker_stop");
      if (action === "docker_start") await probeApi().catch(() => {});
    } catch (error) {
      $("runtime-output").textContent = String(error);
      $("docker-label").textContent = "Unavailable";
    }
  });
}

$("cancel").addEventListener("click", async () => {
  try { toast(await invoke("cancel_active")); } catch (error) { toast(String(error), "error"); }
});

function appendMessageNode(message) {
  document.querySelector(".empty-chat")?.remove();
  const article = document.createElement("article");
  article.className = `message ${message.role}`;
  const label = document.createElement("span");
  label.className = "message-role";
  label.textContent = message.role === "user" ? "You" : activeProfile().name;
  const body = document.createElement("div");
  body.className = "message-body";
  body.textContent = message.content ?? "";
  article.append(label, body);
  if (message.reasoning) {
    const details = document.createElement("details");
    details.className = "reasoning";
    const summary = document.createElement("summary");
    summary.textContent = "Reasoning";
    const reasoning = document.createElement("div");
    reasoning.textContent = message.reasoning;
    details.append(summary, reasoning);
    article.append(details);
  }
  $("messages").append(article);
}

function renderMessages() {
  const messages = activeMessages();
  $("messages").replaceChildren();
  if (!messages.length) {
    $("messages").innerHTML = '<div class="empty-chat"><span>XC</span><h3>Start a private conversation</h3><p>Select a local model profile, test its connection, then send a message.</p></div>';
  } else {
    messages.forEach(appendMessageNode);
  }
  $("messages").scrollTop = $("messages").scrollHeight;
  $("chat-summary").querySelector("small").textContent = `${messages.length} message${messages.length === 1 ? "" : "s"}`;
}

async function probeApi() {
  const profile = activeProfile();
  const base = normaliseBaseUrl(profile.apiUrl);
  $("connection-error").textContent = "";
  if (!isLoopbackUrl(base)) throw new Error("The selected profile is not a permitted loopback HTTP endpoint.");
  const requestOptions = { headers: { Authorization: `Bearer ${$("api-key").value}` }, redirect: "error", signal: AbortSignal.timeout(5000) };
  let response = await fetch(`${base}/health`, requestOptions);
  if (response.status === 404) response = await fetch(`${base}/v1/models`, requestOptions);
  if (!response.ok) throw new Error(`Local service returned HTTP ${response.status}.`);
  $("api-state").textContent = "ONLINE";
  $("api-state").classList.add("safe");
  $("api-state").classList.remove("error");
  return true;
}

$("probe-api").addEventListener("click", async () => {
  try { await probeApi(); toast("Local endpoint is online.", "success"); }
  catch (error) {
    $("api-state").textContent = "OFFLINE";
    $("api-state").classList.add("error");
    $("connection-error").textContent = String(error);
  }
});

$("chat-form").addEventListener("submit", async (event) => {
  event.preventDefault();
  const input = $("chat-input").value.trim();
  if (!input) return;
  const profile = activeProfile();
  const base = normaliseBaseUrl(profile.apiUrl);
  if (!isLoopbackUrl(base)) {
    $("connection-error").textContent = "Only local HTTP endpoints are permitted.";
    return;
  }
  const messages = activeMessages();
  messages.push({ role: "user", content: input, createdAt: new Date().toISOString() });
  setActiveMessages(messages);
  appendMessageNode(messages.at(-1));
  $("chat-input").value = "";
  const metric = {
    id: makeId("request"), workspaceId: activeWorkspace().id, startedAt: new Date().toISOString(),
    status: "running", model: profile.modelId, endpoint: "/v1/chat/completions", durationMs: null,
    inputTokens: estimateTokens(input), outputTokens: 0,
  };
  state.metrics.unshift(metric);
  state.metrics = state.metrics.slice(0, 200);
  persist();
  renderMonitor();
  const started = performance.now();
  try {
    const response = await fetch(`${base}/v1/chat/completions`, {
      method: "POST",
      headers: { "Content-Type": "application/json", Authorization: `Bearer ${$("api-key").value}` },
      redirect: "error",
      body: JSON.stringify({
        model: profile.modelId,
        messages: messages.map(({ role, content }) => ({ role, content })),
        stream: false,
        max_tokens: Number($("max-tokens").value || 256),
        temperature: Number($("temperature").value || 0.2),
        top_p: Number($("top-p").value || 0.9),
        reasoning_effort: $("reasoning-effort").value,
      }),
    });
    if (!response.ok) throw new Error(`Local service returned HTTP ${response.status}: ${(await response.text()).slice(0, 500)}`);
    const data = await response.json();
    const assistant = data.choices?.[0]?.message;
    if (!assistant) throw new Error("The local service response has no assistant message.");
    const saved = {
      role: "assistant",
      content: assistant.content || "",
      reasoning: assistant.reasoning_content || assistant.reasoning || "",
      createdAt: new Date().toISOString(),
    };
    messages.push(saved);
    setActiveMessages(messages);
    appendMessageNode(saved);
    metric.status = "completed";
    metric.outputTokens = data.usage?.completion_tokens ?? estimateTokens(saved.content);
    $("api-state").textContent = "ONLINE";
    $("api-state").classList.add("safe");
  } catch (error) {
    metric.status = "error";
    metric.error = String(error).slice(0, 500);
    const failure = { role: "assistant", content: `Connection failed. ${String(error)}`, createdAt: new Date().toISOString(), error: true };
    messages.push(failure);
    setActiveMessages(messages);
    appendMessageNode(failure);
    $("api-state").textContent = "OFFLINE";
    $("api-state").classList.add("error");
  } finally {
    metric.durationMs = Math.round(performance.now() - started);
    persist();
    renderAll();
  }
});

$("chat-input").addEventListener("keydown", (event) => {
  if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) $("chat-form").requestSubmit();
});

$("new-chat").addEventListener("click", () => {
  if (activeMessages().length && !newChatArmed) {
    newChatArmed = true;
    $("new-chat").textContent = "×";
    toast("Press the conversation button again to clear this session.");
    window.setTimeout(() => { newChatArmed = false; $("new-chat").textContent = "+"; }, 8000);
    return;
  }
  newChatArmed = false;
  $("new-chat").textContent = "+";
  setActiveMessages([]);
  renderAll();
  toast("New local conversation started.");
});

$("native-run").addEventListener("click", async () => {
  const prompt = $("chat-input").value.trim() || activeMessages().filter((item) => item.role === "user").at(-1)?.content;
  if (!prompt) { toast("Enter a prompt first.", "error"); return; }
  if (activeProfile().kind !== "xcaliber") { toast("Exact one-shot requires an Xcaliber K3 profile.", "error"); return; }
  try {
    const result = await execute("run", { prompt });
    showResult(result);
    openView("runtime");
  } catch (error) { toast(String(error), "error"); }
});

function formatDuration(value) {
  if (!Number.isFinite(value)) return "running";
  return value < 1000 ? `${value} ms` : `${(value / 1000).toFixed(value < 10000 ? 1 : 0)} s`;
}

function renderMonitor() {
  const summary = metricsSummary(state.metrics);
  $("monitor-total").textContent = String(summary.total);
  $("monitor-completed").textContent = String(summary.completed);
  $("monitor-errors").textContent = String(summary.failed);
  $("monitor-latency").textContent = summary.averageDurationMs ? formatDuration(summary.averageDurationMs) : "—";
  $("monitor-table").innerHTML = state.metrics.length ? state.metrics.map((item) => `<tr><td>${escapeHtml(new Date(item.startedAt).toLocaleTimeString())}</td><td><span class="status-text ${escapeHtml(item.status)}">${escapeHtml(item.status)}</span></td><td>${escapeHtml(item.model)}</td><td>${escapeHtml(item.endpoint)}</td><td>${escapeHtml(formatDuration(item.durationMs))}</td><td>${Number(item.inputTokens || 0) + Number(item.outputTokens || 0)}</td></tr>`).join("") : '<tr><td colspan="6" class="empty-cell">No local API requests recorded.</td></tr>';
}

function renderActivity() {
  $("activity-table").innerHTML = state.jobs.length ? state.jobs.map((job) => `<tr><td>${escapeHtml(new Date(job.startedAt).toLocaleString())}</td><td>${escapeHtml(job.action.replaceAll("_", " "))}</td><td><span class="status-text ${escapeHtml(job.status)}">${escapeHtml(job.status)}</span></td><td>${job.exitCode ?? "—"}</td><td class="summary-cell">${escapeHtml(job.summary)}</td></tr>`).join("") : '<tr><td colspan="5" class="empty-cell">No local operations recorded.</td></tr>';
}

$("clear-monitor").addEventListener("click", () => { state.metrics = []; persist(); renderAll(); toast("API history cleared."); });
$("clear-activity").addEventListener("click", () => { state.jobs = state.jobs.filter((job) => job.status === "running"); persist(); renderActivity(); toast("Completed activity cleared."); });

function downloadText(filename, text, type = "text/plain") {
  const url = URL.createObjectURL(new Blob([text], { type }));
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  document.body.append(link);
  link.click();
  link.remove();
  URL.revokeObjectURL(url);
}

function exportConversation() {
  const workspace = activeWorkspace();
  downloadText(`${workspace.name.toLowerCase().replace(/[^a-z0-9]+/g, "-") || "xcaliber"}-conversation.md`, conversationMarkdown(activeMessages(), `${workspace.name} conversation`), "text/markdown");
  toast("Conversation export prepared.", "success");
}

$("export-chat").addEventListener("click", exportConversation);
$("download-conversation").addEventListener("click", exportConversation);
$("download-diagnostics").addEventListener("click", () => {
  const report = {
    product: "Xcaliber", exportedAt: new Date().toISOString(), localOnly: true,
    runtime: runtimeInfo, workspace: activeWorkspace(), profile: { ...activeProfile(), apiKey: undefined },
    plan: state.lastPlan, requestSummary: metricsSummary(state.metrics), jobs: state.jobs.slice(0, 50),
  };
  downloadText("xcaliber-diagnostics.json", `${JSON.stringify(report, null, 2)}\n`, "application/json");
  toast("Diagnostic report prepared.", "success");
});
$("download-profile").addEventListener("click", () => {
  const { name, kind, modelId, apiUrl, modelDir, trunkDir } = activeProfile();
  downloadText("xcaliber-local-profile.json", `${JSON.stringify({ schema: 1, name, kind, modelId, apiUrl, modelDir, trunkDir, localOnly: true }, null, 2)}\n`, "application/json");
  toast("Connection profile prepared without a password.", "success");
});

for (const [id, setting, parser] of [
  ["max-tokens", "maxTokens", Number], ["temperature", "temperature", Number], ["top-p", "topP", Number], ["reasoning-effort", "reasoningEffort", String],
]) {
  $(id).addEventListener("change", (event) => { state.settings[setting] = parser(event.target.value); persist(); });
}
$("theme-setting").addEventListener("change", (event) => { state.settings.theme = event.target.value; persist(); applyAppearance(); });
$("density-setting").addEventListener("change", (event) => { state.settings.density = event.target.value; persist(); applyAppearance(); });
$("motion-setting").addEventListener("change", (event) => { state.settings.reducedMotion = event.target.checked; persist(); applyAppearance(); });

$("clear-local-data").addEventListener("click", () => {
  if (!resetArmed) {
    resetArmed = true;
    $("clear-local-data").textContent = "Confirm local data reset";
    $("clear-data-warning").classList.add("armed");
    window.setTimeout(() => { resetArmed = false; $("clear-local-data").textContent = "Prepare local data reset"; $("clear-data-warning").classList.remove("armed"); }, 10000);
    return;
  }
  localStorage.removeItem(STORAGE_KEY);
  state = defaultDesktopState();
  persist();
  resetArmed = false;
  syncProfileControls();
  renderAll();
  toast("Xcaliber local application data was reset.", "success");
});

const onboarding = $("onboarding");
$("onboarding-close").addEventListener("click", () => {
  state.settings.onboardingDismissed = true;
  persist();
  onboarding.hidden = true;
});

async function refreshRuntime() {
  if (!invoke) {
    $("runtime-label").textContent = "Browser preview";
    $("runtime-path").textContent = "Operations require the Tauri app";
    return;
  }
  try {
    runtimeInfo = await invoke("runtime_info");
    $("runtime-label").textContent = runtimeInfo.cliAvailable ? "CLI connected" : "CLI missing";
    $("runtime-path").textContent = runtimeInfo.cliPath || "Keep the portable folder intact";
    $("app-version").textContent = `Xcaliber ${runtimeInfo.appVersion}`;
    $("runtime-dot").classList.toggle("ready", runtimeInfo.cliAvailable);
    $("docker-label").textContent = runtimeInfo.dockerAvailable ? "Docker available" : "Docker not detected";
    $("docker-dot").classList.toggle("ready", runtimeInfo.dockerAvailable);
  } catch (error) {
    $("runtime-label").textContent = "Runtime error";
    $("runtime-path").textContent = String(error);
  }
}

function renderAll() {
  renderWorkspaceOptions();
  renderProfiles();
  renderMessages();
  renderMonitor();
  renderActivity();
  renderWorkspaceSummary();
}

applyAppearance();
onboarding.hidden = Boolean(state.settings.onboardingDismissed);
syncProfileControls();
renderAll();
if (state.lastPlan?.hardware) renderPlan(state.lastPlan);
if (viewMeta[window.location.hash.slice(1)]) openView(window.location.hash.slice(1), false);
refreshRuntime();
