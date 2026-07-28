import assert from "node:assert/strict";
import { access, readFile } from "node:fs/promises";
import test from "node:test";

async function render() {
  const workerUrl = new URL("../dist/server/index.js", import.meta.url);
  workerUrl.searchParams.set("test", `${process.pid}-${Date.now()}`);
  const { default: worker } = await import(workerUrl.href);

  return worker.fetch(
    new Request("http://localhost/", {
      headers: { accept: "text/html" },
    }),
    {
      ASSETS: {
        fetch: async () => new Response("Not found", { status: 404 }),
      },
    },
    {
      waitUntil() {},
      passThroughOnException() {},
    },
  );
}

test("server-renders the finished Vesper landing page", async () => {
  const response = await render();
  assert.equal(response.status, 200);
  assert.match(response.headers.get("content-type") ?? "", /^text\/html\b/i);

  const html = await response.text();
  assert.match(html, /<html lang="ko"/i);
  assert.match(html, /<title>Vesper \| Windows 오디오를 더 정교하게<\/title>/i);
  assert.match(html, /PC SOUND\.[\s\S]*TUNED TO YOUR SYSTEM\./);
  assert.match(html, /class="product-switcher" role="tablist"/);
  assert.match(html, /role="tab" aria-selected="true"/);
  assert.match(html, /Vesper DSP/);
  assert.match(html, /Vesper Woofer/);
  assert.match(html, /class="app-ui app-ui-dsp"/);
  assert.match(html, /class="app-ui app-ui-woofer"/);
  assert.match(html, /기본 입력 장치/);
  assert.match(html, /Audio Device/);
  assert.equal((html.match(/class="app-window-dot"/g) ?? []).length, 4);
  assert.equal((html.match(/class="app-output-actions"/g) ?? []).length, 2);
  assert.match(html, /class="app-slope-label">SLOPE/);
  assert.doesNotMatch(html, /FiiO|EDIFIER/i);
  assert.match(html, /제품 비교하기/);
  assert.match(html, /Switch to English/);
  assert.match(html, /자주 묻는 질문/);
  assert.match(html, /최대 768 kHz · 장치별 16\/24\/32-bit/);
  assert.match(html, /HEADPHONES \+ SPEAKERS/);
  assert.match(html, /헤드폰·스피커 보정/);
  assert.doesNotMatch(html, /최대 192 kHz \/ 24-bit/);
  assert.match(
    html,
    /class="button primary" href="https:\/\/github\.com\/minseokk77\/vesper-dsp/,
  );
  assert.doesNotMatch(html, /class="audio-window"/);
  assert.doesNotMatch(html, /codex-preview|Building your site|react-loading-skeleton/i);
});

test("keeps deployment assets and motion preferences production-ready", async () => {
  const [css, packageJson] = await Promise.all([
    readFile(new URL("../app/globals.css", import.meta.url), "utf8"),
    readFile(new URL("../package.json", import.meta.url), "utf8"),
  ]);

  await access(new URL("../public/og.png", import.meta.url));
  assert.match(css, /prefers-reduced-motion:\s*reduce/);
  assert.match(css, /:focus-visible/);
  assert.match(css, /backdrop-filter/);
  assert.match(packageJson, /cross-env WRANGLER_LOG_PATH=/);
  assert.doesNotMatch(packageJson, /react-loading-skeleton/);
});
