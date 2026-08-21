import test from "node:test";
import assert from "node:assert/strict";
import {
  conversationMarkdown,
  defaultDesktopState,
  estimateTokens,
  isLoopbackUrl,
  mergeDesktopState,
  metricsSummary,
  safeJsonParse,
} from "../ui/core.mjs";

test("loopback policy accepts only local HTTP endpoints", () => {
  assert.equal(isLoopbackUrl("http://127.0.0.1:8000"), true);
  assert.equal(isLoopbackUrl("http://localhost:11434"), true);
  assert.equal(isLoopbackUrl("http://127.9.8.7/v1"), false);
  assert.equal(isLoopbackUrl("http://[::1]:8000"), false);
  assert.equal(isLoopbackUrl("https://localhost:8000"), false);
  assert.equal(isLoopbackUrl("http://user:secret@127.0.0.1:8000"), false);
  assert.equal(isLoopbackUrl("http://example.com:8000"), false);
});

test("stored state is bounded and invalid state returns defaults", () => {
  const fallback = defaultDesktopState();
  assert.equal(mergeDesktopState(safeJsonParse("not-json", null)).schema, 2);
  const candidate = { ...fallback, jobs: Array.from({ length: 140 }, (_, id) => ({ id })) };
  assert.equal(mergeDesktopState(candidate).jobs.length, 100);
});

test("conversation export and metrics contain useful values", () => {
  const markdown = conversationMarkdown([{ role: "user", content: "hello" }, { role: "assistant", content: "hi" }], "Test");
  assert.match(markdown, /^# Test/);
  assert.match(markdown, /## Assistant/);
  assert.equal(estimateTokens("12345678"), 2);
  assert.deepEqual(metricsSummary([
    { status: "completed", durationMs: 100, outputTokens: 4 },
    { status: "completed", durationMs: 200, outputTokens: 6 },
    { status: "error" },
  ]), { total: 3, completed: 2, failed: 1, averageDurationMs: 150, outputTokens: 10 });
});
