export const STORAGE_KEY = "xcaliber:desktop:v2";

export function makeId(prefix = "item") {
  const random = globalThis.crypto?.randomUUID?.() ??
    `${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`;
  return `${prefix}-${random}`;
}

export function safeJsonParse(value, fallback) {
  if (typeof value !== "string" || !value.trim()) return fallback;
  try {
    return JSON.parse(value);
  } catch {
    return fallback;
  }
}

export function isLoopbackUrl(value) {
  try {
    const url = new URL(value);
    if (url.protocol !== "http:") return false;
    if (url.username || url.password) return false;
    const host = url.hostname.toLowerCase().replace(/^\[|\]$/g, "");
    return host === "localhost" || host === "127.0.0.1";
  } catch {
    return false;
  }
}

export function formatBytes(value) {
  if (!Number.isFinite(value) || value < 0) return "—";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let amount = value;
  let unit = 0;
  while (amount >= 1000 && unit < units.length - 1) {
    amount /= 1000;
    unit += 1;
  }
  return `${amount.toFixed(unit >= 3 ? 2 : 1)} ${units[unit]}`;
}

export function estimateTokens(text) {
  if (typeof text !== "string" || !text.trim()) return 0;
  return Math.max(1, Math.ceil(text.trim().length / 4));
}

export function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}

export function normaliseBaseUrl(value) {
  return String(value ?? "").trim().replace(/\/+$/, "");
}

export function defaultDesktopState() {
  const workspaceId = "workspace-local";
  const profileId = "profile-k3";
  return {
    schema: 2,
    activeWorkspaceId: workspaceId,
    activeProfileId: profileId,
    workspaces: [
      {
        id: workspaceId,
        name: "Local lab",
        description: "Private inference on this computer",
        profileId,
        createdAt: new Date().toISOString(),
      },
    ],
    profiles: [
      {
        id: profileId,
        name: "Xcaliber K3",
        kind: "xcaliber",
        modelId: "kimi-k3-local",
        apiUrl: "http://127.0.0.1:8000",
        modelDir: "",
        trunkDir: "",
        destination: "",
      },
      {
        id: "profile-local",
        name: "Smaller local model",
        kind: "compatible",
        modelId: "local-model",
        apiUrl: "http://127.0.0.1:11434",
        modelDir: "",
        trunkDir: "",
        destination: "",
      },
    ],
    chats: { [workspaceId]: [] },
    jobs: [],
    metrics: [],
    lastPlan: null,
    settings: {
      theme: "dark",
      density: "comfortable",
      reducedMotion: false,
      onboardingDismissed: false,
      maxTokens: 256,
      temperature: 0.2,
      topP: 0.9,
      reasoningEffort: "max",
    },
  };
}

export function mergeDesktopState(candidate) {
  const defaults = defaultDesktopState();
  if (!candidate || candidate.schema !== 2) return defaults;
  const workspaces = Array.isArray(candidate.workspaces) && candidate.workspaces.length
    ? candidate.workspaces.slice(0, 50)
    : defaults.workspaces;
  const profiles = Array.isArray(candidate.profiles) && candidate.profiles.length
    ? candidate.profiles.slice(0, 50)
    : defaults.profiles;
  return {
    ...defaults,
    ...candidate,
    workspaces,
    profiles,
    chats: candidate.chats && typeof candidate.chats === "object" ? candidate.chats : defaults.chats,
    jobs: Array.isArray(candidate.jobs) ? candidate.jobs.slice(0, 100) : [],
    metrics: Array.isArray(candidate.metrics) ? candidate.metrics.slice(0, 200) : [],
    settings: { ...defaults.settings, ...(candidate.settings ?? {}) },
    activeWorkspaceId: workspaces.some((item) => item.id === candidate.activeWorkspaceId)
      ? candidate.activeWorkspaceId
      : workspaces[0].id,
    activeProfileId: profiles.some((item) => item.id === candidate.activeProfileId)
      ? candidate.activeProfileId
      : profiles[0].id,
  };
}

export function conversationMarkdown(messages, title = "Xcaliber conversation") {
  const lines = [`# ${title}`, ""];
  for (const message of messages ?? []) {
    const role = message.role === "user" ? "You" : "Assistant";
    lines.push(`## ${role}`, "", String(message.content ?? "").trim(), "");
    if (message.reasoning) lines.push("<details><summary>Reasoning</summary>", "", String(message.reasoning).trim(), "", "</details>", "");
  }
  return `${lines.join("\n").trim()}\n`;
}

export function metricsSummary(metrics) {
  const items = Array.isArray(metrics) ? metrics : [];
  const completed = items.filter((item) => item.status === "completed");
  const failed = items.filter((item) => item.status === "error");
  const totalDuration = completed.reduce((sum, item) => sum + (Number(item.durationMs) || 0), 0);
  const totalTokens = completed.reduce((sum, item) => sum + (Number(item.outputTokens) || 0), 0);
  return {
    total: items.length,
    completed: completed.length,
    failed: failed.length,
    averageDurationMs: completed.length ? Math.round(totalDuration / completed.length) : 0,
    outputTokens: totalTokens,
  };
}
