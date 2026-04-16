<script lang="ts">
  import { dndzone, SHADOW_PLACEHOLDER_ITEM_ID } from 'svelte-dnd-action';
  import { onMount } from 'svelte';

  // ── Types ─────────────────────────────────────────────────────────────────

  type ConnectionType = 'local' | 'lan' | 'api';

  interface ProviderRow {
    id: string;
    label: string;
    connectionType: ConnectionType;
    keyName?: string;
    urlKey?: string;
    modelKey?: string;
    /** If set, clicking the status dot shows this info text instead of a key/URL input. */
    infoText?: string;
    keyExpanded: boolean;
    keyDraft: string;
    urlExpanded: boolean;
    urlDraft: string;
    modelDraft: string;
    infoExpanded: boolean;
    enabled: boolean;
  }

  interface PullProgress {
    status: string;
    pct: number;
    done: boolean;
    error?: boolean;
  }

  type ServiceKey = 'suggestion' | 'transcription' | 'sentiment';

  interface ServiceDef {
    key: ServiceKey;
    label: string;
    settingsField: string;
    rows: Omit<ProviderRow, 'keyExpanded' | 'keyDraft' | 'urlExpanded' | 'urlDraft' | 'modelDraft' | 'enabled'>[];
  }

  // ── Static definitions ────────────────────────────────────────────────────

  const SERVICES: ServiceDef[] = [
    {
      key: 'suggestion', label: 'Suggestions', settingsField: 'suggestion_order',
      rows: [
        { id: 'groq',        label: 'Groq',        connectionType: 'api',   keyName: 'groq'        },
        { id: 'groq2',       label: 'Groq #2',     connectionType: 'api',   keyName: 'groq2'       },
        { id: 'mistral',     label: 'Mistral',     connectionType: 'api',   keyName: 'mistral'     },
        { id: 'claude_cli',  label: 'Claude CLI',  connectionType: 'local', infoText: 'Requires Claude CLI.\nInstall: npm i -g @anthropic-ai/claude-code\nThen run: claude login' },
        { id: 'claude_api',  label: 'Claude API',  connectionType: 'api',   keyName: 'anthropic'   },
        { id: 'ollama',      label: 'Ollama',      connectionType: 'local', urlKey: 'ollama',      modelKey: 'ollama'      },
        { id: 'open_router', label: 'OpenRouter',  connectionType: 'api',   keyName: 'openrouter', modelKey: 'openrouter'  },
        { id: 'qwen',        label: 'Qwen',        connectionType: 'api',   keyName: 'qwen'        },
        { id: 'cerebras',    label: 'Cerebras',    connectionType: 'api',   keyName: 'cerebras'    },
        { id: 'deep_seek',   label: 'DeepSeek',    connectionType: 'api',   keyName: 'deepseek'    },
        { id: 'lan_ollama',  label: 'LAN Ollama (Bonsai)', connectionType: 'lan', urlKey: 'lan_ollama', modelKey: 'lan_ollama' },
        { id: 'gemma',       label: 'Gemma',       connectionType: 'api',   keyName: 'gemini'      },
        { id: 'gemini',      label: 'Gemini',      connectionType: 'api',   keyName: 'gemini'      },
      ],
    },
    {
      key: 'transcription', label: 'Transcription', settingsField: 'transcription_order',
      rows: [
        { id: 'whisper_local',  label: 'Whisper',         connectionType: 'local', urlKey: 'whisper'  },
        { id: 'deepgram',       label: 'Deepgram',        connectionType: 'api',   keyName: 'deepgram' },
        { id: 'groq_whisper_2', label: 'Groq Whisper #2', connectionType: 'api',   keyName: 'groq2'   },
        { id: 'groq_whisper',   label: 'Groq Whisper',    connectionType: 'api',   keyName: 'groq'    },
        { id: 'gemini',         label: 'Gemini',          connectionType: 'api',   keyName: 'gemini'  },
      ],
    },
    {
      key: 'sentiment', label: 'Sentiment', settingsField: 'sentiment_order',
      rows: [
        { id: 'ollama_vision', label: 'Ollama Vision', connectionType: 'local', urlKey: 'ollama'   },
        { id: 'gemini_vision', label: 'Gemini Vision', connectionType: 'api',   keyName: 'gemini'  },
        { id: 'claude_vision', label: 'Claude Vision', connectionType: 'api',   keyName: 'anthropic' },
      ],
    },
  ];

  const STORAGE_KEY    = 'provider-settings';
  const API_KEYS_STORAGE = 'provider-api-keys';

  // ── State ─────────────────────────────────────────────────────────────────

  let configuredKeys  = $state<Record<string, boolean>>({});
  let serverUrls      = $state<Record<string, string>>({});
  let serverModels    = $state<Record<string, string>>({});
  let localKeys       = $state<Record<string, string>>(loadLocalKeys());

  // Available (pulled) models per urlKey, null = not yet fetched
  let availableModels = $state<Record<string, string[] | null>>({});
  let probeStatus     = $state<Record<string, 'pending' | 'ok' | 'fail' | 'untested'>>({});
  let pullProgress    = $state<Record<string, PullProgress>>({});

  function loadLocalKeys(): Record<string, string> {
    try { return JSON.parse(localStorage.getItem(API_KEYS_STORAGE) ?? '{}'); } catch { return {}; }
  }

  function buildRows(svc: ServiceDef): ProviderRow[] {
    const saved = loadSavedState(svc);
    return svc.rows.map(r => ({
      ...r,
      keyExpanded: false, keyDraft: '',
      urlExpanded: false, urlDraft: '',
      modelDraft: '', infoExpanded: false,
      enabled: saved.enabled.has(r.id) ? saved.enabled.get(r.id)! : true,
    })).sort((a, b) => {
      const ai = saved.order.indexOf(a.id), bi = saved.order.indexOf(b.id);
      if (ai === -1 && bi === -1) return 0;
      if (ai === -1) return 1; if (bi === -1) return -1;
      return ai - bi;
    });
  }

  function loadSavedState(svc: ServiceDef): { order: string[]; enabled: Map<string, boolean> } {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (!raw) return { order: [], enabled: new Map() };
      const saved = JSON.parse(raw) as Record<string, unknown>;
      const order = Array.isArray(saved[svc.settingsField]) ? saved[svc.settingsField] as string[] : [];
      const enabledArr = Array.isArray(saved[`${svc.key}_enabled`]) ? saved[`${svc.key}_enabled`] as string[] : null;
      const enabled = new Map<string, boolean>();
      if (enabledArr) svc.rows.forEach(r => enabled.set(r.id, enabledArr.includes(r.id)));
      return { order, enabled };
    } catch { return { order: [], enabled: new Map() }; }
  }

  let lists = $state<Record<ServiceKey, ProviderRow[]>>({
    suggestion:    buildRows(SERVICES[0]),
    transcription: buildRows(SERVICES[1]),
    sentiment:     buildRows(SERVICES[2]),
  });

  // Claude CLI install state
  let cliStatus = $state<'unknown' | 'ok' | 'fail' | 'installing'>('unknown');
  let cliVersion = $state('');
  let cliInstallLog = $state<string[]>([]);

  async function probeCli() {
    cliStatus = 'unknown';
    try {
      const res = await fetch('/api/probe?target=claude_cli');
      if (res.ok) {
        const data = await res.json() as { ok: boolean; url: string; error?: string };
        cliStatus = data.ok ? 'ok' : 'fail';
        cliVersion = data.ok ? data.url : (data.error ?? 'Not installed');
      } else { cliStatus = 'fail'; cliVersion = 'Check failed'; }
    } catch { cliStatus = 'fail'; cliVersion = 'Check failed'; }
  }

  async function installCli() {
    cliStatus = 'installing';
    cliInstallLog = ['Launching npm install…'];
    try {
      const res = await fetch('/api/claude-cli/install', { method: 'POST' });
      if (!res.body) { cliInstallLog = [...cliInstallLog, 'No response body']; cliStatus = 'fail'; return; }
      const reader = res.body.getReader();
      const decoder = new TextDecoder();
      let buf = '';
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        buf += decoder.decode(value, { stream: true });
        const lines = buf.split('\n'); buf = lines.pop() ?? '';
        for (const raw of lines) {
          const line = raw.startsWith('data: ') ? raw.slice(6) : raw;
          if (!line.trim()) continue;
          try {
            const evt = JSON.parse(line) as { status?: string; log?: string; error?: string; success?: boolean };
            if (evt.success) {
              cliInstallLog = [...cliInstallLog, '✓ Done!'];
              probeCli();
            } else if (evt.error) {
              cliInstallLog = [...cliInstallLog, `Error: ${evt.error}`];
              cliStatus = 'fail';
            } else if (evt.status) {
              cliInstallLog = [...cliInstallLog, evt.status];
            } else if (evt.log) {
              cliInstallLog = [...cliInstallLog, evt.log];
            }
          } catch {}
        }
      }
    } catch (e) { cliInstallLog = [...cliInstallLog, `Error: ${e}`]; cliStatus = 'fail'; }
  }

  let saving = $state(false);
  let savedOk = $state(false);
  let expanded = $state<Record<ServiceKey, boolean>>({
    suggestion: true, transcription: false, sentiment: false,
  });

  // ── Helpers ───────────────────────────────────────────────────────────────

  function effectiveUrl(row: ProviderRow): string {
    return row.urlKey ? (serverUrls[row.urlKey] ?? '') : '';
  }

  function isKeyConfigured(row: ProviderRow): boolean {
    return !row.keyName || configuredKeys[row.keyName] === true;
  }

  function isReady(row: ProviderRow): boolean {
    if (row.keyName && !configuredKeys[row.keyName]) return false;
    if (row.urlKey && !effectiveUrl(row)) return false;
    return true;
  }

  /** True if we have a model list and the given name isn't in it (fuzzy: ignores :tag). */
  function modelNotFound(urlKey: string, name: string): boolean {
    const list = availableModels[urlKey];
    if (!list || !list.length || !name.trim()) return false;
    const base = name.split(':')[0].toLowerCase();
    return !list.some(m => m.toLowerCase() === name.toLowerCase() || m.split(':')[0].toLowerCase() === base);
  }

  // ── Persistence ───────────────────────────────────────────────────────────

  function buildPayload() {
    const payload: Record<string, unknown> = {};
    for (const svc of SERVICES) {
      const rows = lists[svc.key].filter(r => r.id !== SHADOW_PLACEHOLDER_ITEM_ID);
      const enabledRows = rows.filter(r => r.enabled);
      payload[svc.settingsField] = enabledRows.map(r => r.id);
      payload[`${svc.key}_enabled`] = enabledRows.map(r => r.id);
      payload[`${svc.key}_full_order`] = rows.map(r => r.id);
    }
    localStorage.setItem(STORAGE_KEY, JSON.stringify(payload));
    return payload;
  }

  async function saveOrder() {
    const payload = buildPayload();
    saving = true; savedOk = false;
    try {
      await fetch('/api/settings', {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          suggestion_order:    payload['suggestion_order'],
          transcription_order: payload['transcription_order'],
          sentiment_order:     payload['sentiment_order'],
        }),
      });
      savedOk = true;
      setTimeout(() => { savedOk = false; }, 1500);
    } catch {}
    saving = false;
  }

  async function submitKey(svcKey: ServiceKey, row: ProviderRow) {
    const key = row.keyDraft.trim();
    if (!key || !row.keyName) return;
    await fetch('/api/settings', {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ api_keys: { [row.keyName]: key } }),
    }).catch(() => {});
    localKeys[row.keyName] = key;
    localStorage.setItem(API_KEYS_STORAGE, JSON.stringify(localKeys));
    configuredKeys[row.keyName] = true;
    lists[svcKey] = lists[svcKey].map(r =>
      r.id === row.id ? { ...r, keyExpanded: false, keyDraft: '' } : r
    );
  }

  async function submitUrl(svcKey: ServiceKey, row: ProviderRow) {
    const url = row.urlDraft.trim();
    if (!row.urlKey) return;
    await fetch('/api/settings', {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ urls: { [row.urlKey]: url } }),
    }).catch(() => {});
    serverUrls[row.urlKey] = url;
    probeStatus[row.urlKey] = 'untested';
    availableModels[row.urlKey] = null;
    lists[svcKey] = lists[svcKey].map(r =>
      r.id === row.id ? { ...r, urlExpanded: false, urlDraft: '' } : r
    );
  }

  async function submitModel(row: ProviderRow) {
    if (!row.modelKey) return;
    await fetch('/api/settings', {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ models: { [row.modelKey]: row.modelDraft.trim() } }),
    }).catch(() => {});
    serverModels[row.modelKey] = row.modelDraft.trim();
  }

  // ── Probe + model list ────────────────────────────────────────────────────

  async function probe(urlKey: string) {
    probeStatus[urlKey] = 'pending';
    try {
      const res = await fetch(`/api/probe?target=${encodeURIComponent(urlKey)}`);
      if (res.ok) {
        const data = await res.json() as { ok: boolean };
        probeStatus[urlKey] = data.ok ? 'ok' : 'fail';
        if (data.ok && (urlKey === 'ollama' || urlKey === 'lan_ollama')) {
          await fetchModels(urlKey);
        }
      } else {
        probeStatus[urlKey] = 'fail';
      }
    } catch { probeStatus[urlKey] = 'fail'; }
  }

  async function fetchModels(urlKey: string) {
    try {
      const res = await fetch(`/api/ollama/models?target=${encodeURIComponent(urlKey)}`);
      if (res.ok) {
        const data = await res.json() as { models: string[] };
        availableModels[urlKey] = data.models ?? [];
        // Pre-fill model draft from server if row has no draft yet
        for (const svc of SERVICES) {
          lists[svc.key] = lists[svc.key].map(r =>
            (r.urlKey === urlKey || r.modelKey === urlKey) && !r.modelDraft
              ? { ...r, modelDraft: serverModels[r.modelKey ?? ''] ?? '' }
              : r
          );
        }
      }
    } catch {}
  }

  // ── Pull a model (local Ollama only) ──────────────────────────────────────

  async function pullModel(modelName: string) {
    if (!modelName.trim()) return;
    pullProgress[modelName] = { status: 'starting…', pct: 0, done: false };

    try {
      const res = await fetch('/api/ollama/pull', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ model: modelName }),
      });
      if (!res.body) { pullProgress[modelName] = { status: 'No response body', pct: 0, done: true, error: true }; return; }

      const reader = res.body.getReader();
      const decoder = new TextDecoder();
      let buf = '';

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        buf += decoder.decode(value, { stream: true });
        const lines = buf.split('\n');
        buf = lines.pop() ?? '';
        for (const raw of lines) {
          // SSE format: "data: {...}" or raw JSON line
          const line = raw.startsWith('data: ') ? raw.slice(6) : raw;
          if (!line.trim()) continue;
          try {
            const evt = JSON.parse(line) as {
              status?: string; error?: string;
              total?: number; completed?: number;
            };
            if (evt.error) {
              pullProgress[modelName] = { status: `Error: ${evt.error}`, pct: 0, done: true, error: true };
            } else {
              const pct = evt.total ? Math.round(((evt.completed ?? 0) / evt.total) * 100) : 0;
              const done = evt.status === 'success';
              pullProgress[modelName] = { status: evt.status ?? 'pulling', pct, done };
              if (done) {
                // Refresh model list so it shows up in the combobox
                await fetchModels('ollama');
              }
            }
          } catch {}
        }
      }
    } catch (e) {
      pullProgress[modelName] = { status: `Error: ${e}`, pct: 0, done: true, error: true };
    }
  }

  function resetAll() {
    for (const svc of SERVICES) lists[svc.key] = buildRows(svc);
    saveOrder();
  }

  // ── Mount ─────────────────────────────────────────────────────────────────

  onMount(async () => {
    const payload = buildPayload();
    const body: Record<string, unknown> = {
      suggestion_order:    payload['suggestion_order'],
      transcription_order: payload['transcription_order'],
      sentiment_order:     payload['sentiment_order'],
    };
    const stored = loadLocalKeys();
    if (Object.keys(stored).length > 0) body['api_keys'] = stored;
    fetch('/api/settings', {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    }).catch(() => {});

    // Fetch configured key status + current URLs + models
    try {
      const res = await fetch('/api/settings');
      if (res.ok) {
        const data = await res.json() as {
          configured_keys: Record<string, boolean>;
          urls: Record<string, string>;
          models: Record<string, string>;
        };
        configuredKeys = data.configured_keys ?? {};
        serverUrls     = data.urls ?? {};
        serverModels   = data.models ?? {};
        // Pre-fill model drafts
        for (const svc of SERVICES) {
          lists[svc.key] = lists[svc.key].map(r => ({
            ...r,
            modelDraft: r.modelKey ? (serverModels[r.modelKey] ?? '') : '',
          }));
        }
      }
    } catch {}

    // Background: try to fetch model lists for Ollama instances that have a URL
    if (serverUrls['ollama'])     fetchModels('ollama');
    if (serverUrls['lan_ollama']) fetchModels('lan_ollama');
  });
</script>

<!-- ── Template ──────────────────────────────────────────────────────────── -->

<div class="po-panel">
  <div class="po-header">
    <span class="po-title">Providers</span>
    <div class="po-actions">
      {#if savedOk}<span class="po-saved">saved</span>{/if}
      {#if saving}<span class="po-saving">…</span>{/if}
      <button class="po-reset" onclick={resetAll}>Reset</button>
    </div>
  </div>
  <p class="po-hint">Drag to reorder · toggle to skip</p>

  {#each SERVICES as svc}
    {@const rows = lists[svc.key]}
    <div class="po-section">
      <button class="po-section-toggle" onclick={() => { expanded[svc.key] = !expanded[svc.key]; }}>
        <span class="po-section-label">{svc.label}</span>
        <span class="po-section-chevron">{expanded[svc.key] ? '▴' : '▾'}</span>
      </button>

      {#if expanded[svc.key]}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="po-list"
          use:dndzone={{ items: rows, flipDurationMs: 120 }}
          onconsider={(e) => { lists[svc.key] = e.detail.items; }}
          onfinalize={(e) => { lists[svc.key] = e.detail.items; saveOrder(); }}
        >
          {#each rows as row (row.id)}
            {@const keyOk = isKeyConfigured(row)}
            {@const ready = isReady(row)}
            <div class="po-item" class:po-item-disabled={!row.enabled}>

              <!-- ── Main row ────────────────────────────────────────── -->
              <div class="po-row">
                <span class="po-handle" class:po-handle-disabled={!row.enabled}>⠿</span>

                <!-- Status dot -->
                {#if row.urlKey && (row.connectionType === 'local' || row.connectionType === 'lan')}
                  <button class="po-status" class:po-status-ok={ready}
                    title={ready ? `Connected · ${effectiveUrl(row)}` : 'Click to configure URL'}
                    onclick={() => {
                      lists[svc.key] = lists[svc.key].map(r =>
                        r.id === row.id ? { ...r, urlExpanded: !r.urlExpanded, urlDraft: effectiveUrl(row), keyExpanded: false } : r
                      );
                    }}
                  ></button>
                {:else if row.keyName}
                  <button class="po-status" class:po-status-ok={keyOk}
                    title={keyOk ? 'Key configured' : 'No key — click to add'}
                    onclick={() => {
                      if (!keyOk) lists[svc.key] = lists[svc.key].map(r =>
                        r.id === row.id ? { ...r, keyExpanded: !r.keyExpanded, urlExpanded: false } : r
                      );
                    }}
                  ></button>
                {:else if row.infoText}
                  <button class="po-status po-status-info"
                    class:po-status-ok={row.id === 'claude_cli' && cliStatus === 'ok'}
                    class:po-status-fail={row.id === 'claude_cli' && cliStatus === 'fail'}
                    title={row.id === 'claude_cli'
                      ? (cliStatus === 'ok' ? `Claude CLI: ${cliVersion}` : 'Click to check / install Claude CLI')
                      : 'Click for setup info'}
                    onclick={() => {
                      lists[svc.key] = lists[svc.key].map(r =>
                        r.id === row.id ? { ...r, infoExpanded: !r.infoExpanded } : r
                      );
                      if (row.id === 'claude_cli' && !row.infoExpanded && cliStatus === 'unknown') probeCli();
                    }}
                  ></button>
                {:else}
                  <span class="po-status po-status-ok po-status-nokey"></span>
                {/if}

                <!-- Connection type badge -->
                <span class="po-badge po-badge-{row.connectionType}">
                  {row.connectionType}
                </span>

                <span class="po-label">{row.label}</span>

                <!-- Toggle -->
                <button class="po-toggle" class:po-toggle-on={row.enabled}
                  title={row.enabled ? 'Disable' : 'Enable'}
                  onclick={() => {
                    lists[svc.key] = lists[svc.key].map(r =>
                      r.id === row.id ? { ...r, enabled: !r.enabled } : r
                    );
                    saveOrder();
                  }}
                >
                  <span class="po-toggle-thumb"></span>
                </button>
              </div>

              <!-- ── URL config row ──────────────────────────────────── -->
              {#if row.urlExpanded && row.urlKey}
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div class="po-sub-row" onclick={(e) => e.stopPropagation()}>
                  <input class="po-url-input" type="text"
                    placeholder={row.connectionType === 'lan'
                      ? 'http://192.168.x.x:11434'
                      : row.id === 'whisper_local' ? 'http://localhost:8000' : 'http://localhost:11434'}
                    bind:value={row.urlDraft}
                    onkeydown={(e) => { if (e.key === 'Enter') submitUrl(svc.key, row); }}
                  />
                  <!-- Test button -->
                  {#if row.urlKey !== 'whisper'}
                    <button class="po-probe-btn"
                      class:po-probe-ok={probeStatus[row.urlKey] === 'ok'}
                      class:po-probe-fail={probeStatus[row.urlKey] === 'fail'}
                      class:po-probe-pending={probeStatus[row.urlKey] === 'pending'}
                      onclick={async () => {
                        const url = row.urlDraft.trim();
                        if (url && url !== effectiveUrl(row)) {
                          await submitUrl(svc.key, row);
                          lists[svc.key] = lists[svc.key].map(r =>
                            r.id === row.id ? { ...r, urlExpanded: true, urlDraft: url } : r
                          );
                        }
                        if (row.urlKey) probe(row.urlKey);
                      }}
                    >
                      {probeStatus[row.urlKey] === 'pending' ? '…'
                        : probeStatus[row.urlKey] === 'ok' ? '✓'
                        : probeStatus[row.urlKey] === 'fail' ? '✗'
                        : 'Test'}
                    </button>
                  {/if}
                  <button class="po-sub-save" disabled={!row.urlDraft.trim()}
                    onclick={() => submitUrl(svc.key, row)}>Save</button>
                  <button class="po-sub-cancel"
                    onclick={() => lists[svc.key] = lists[svc.key].map(r =>
                      r.id === row.id ? { ...r, urlExpanded: false, urlDraft: '' } : r
                    )}>✕</button>
                </div>
              {/if}

              <!-- ── API key input row ───────────────────────────────── -->
              {#if row.keyExpanded && row.keyName}
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div class="po-sub-row" onclick={(e) => e.stopPropagation()}>
                  <input class="po-key-input" type="password"
                    placeholder="Paste API key…"
                    bind:value={row.keyDraft}
                    onkeydown={(e) => { if (e.key === 'Enter') submitKey(svc.key, row); }}
                  />
                  <button class="po-sub-save" disabled={!row.keyDraft.trim()}
                    onclick={() => submitKey(svc.key, row)}>Save</button>
                  <button class="po-sub-cancel"
                    onclick={() => lists[svc.key] = lists[svc.key].map(r =>
                      r.id === row.id ? { ...r, keyExpanded: false, keyDraft: '' } : r
                    )}>✕</button>
                </div>
              {/if}

              <!-- ── Info panel ─────────────────────────────────────── -->
              {#if row.infoExpanded && row.infoText}
                <div class="po-info-row">
                  {#if row.id === 'claude_cli'}
                    <!-- Claude CLI: probe status + install button -->
                    <div class="po-cli-status-row">
                      {#if cliStatus === 'unknown'}
                        <span class="po-cli-checking">Checking…</span>
                      {:else if cliStatus === 'ok'}
                        <span class="po-cli-ok">✓ {cliVersion}</span>
                        <button class="po-cli-recheck" onclick={probeCli}>Re-check</button>
                      {:else if cliStatus === 'fail'}
                        <span class="po-cli-fail">{cliVersion}</span>
                        <button class="po-cli-install-btn" onclick={installCli}>Install</button>
                      {:else if cliStatus === 'installing'}
                        <span class="po-cli-installing">Installing…</span>
                      {/if}
                    </div>
                    {#if cliInstallLog.length > 0}
                      <div class="po-cli-log">
                        {#each cliInstallLog.slice(-6) as line}
                          <div class="po-cli-log-line">{line}</div>
                        {/each}
                      </div>
                    {/if}
                    <div class="po-info-line" style="margin-top: 0.35rem; color: #475569;">After install: <code class="po-cli-code">claude login</code></div>
                  {:else}
                    {#each row.infoText.split('\n') as line}
                      <div class="po-info-line">{line}</div>
                    {/each}
                  {/if}
                </div>
              {/if}

              <!-- ── Model row ───────────────────────────────────────── -->
              {#if row.modelKey}
                {@const urlKey = row.urlKey ?? ''}
                {@const models = urlKey ? (availableModels[urlKey] ?? []) : []}
                {@const notFound = urlKey && modelNotFound(urlKey, row.modelDraft)}
                {@const isPulling = row.modelDraft && pullProgress[row.modelDraft] && !pullProgress[row.modelDraft].done}
                {@const pullDone  = row.modelDraft && pullProgress[row.modelDraft]?.done}
                {@const listId = `models-${row.id}`}
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div class="po-model-row" onclick={(e) => e.stopPropagation()}>
                  <span class="po-model-label">model</span>
                  <input class="po-model-input" type="text" list={models.length ? listId : undefined}
                    placeholder={row.modelKey === 'openrouter'
                      ? 'auto (free tier) or e.g. google/gemini-2.0-flash-exp:free'
                      : 'e.g. llama3.2:latest'}
                    bind:value={row.modelDraft}
                    onkeydown={(e) => { if (e.key === 'Enter') submitModel(row); }}
                    onblur={() => submitModel(row)}
                  />
                  {#if models.length}
                    <datalist id={listId}>
                      {#each models as m}<option value={m}></option>{/each}
                    </datalist>
                  {/if}

                  <!-- Local Ollama: pull button when model not found -->
                  {#if row.connectionType === 'local' && notFound && !isPulling && !pullDone}
                    <button class="po-pull-btn"
                      title="Pull this model from Ollama registry"
                      onclick={() => pullModel(row.modelDraft)}>Pull</button>
                  {/if}
                  <!-- LAN Ollama: "not on remote" hint -->
                  {#if row.connectionType === 'lan' && notFound}
                    <span class="po-not-remote" title="Pull this model on the remote machine first">not on remote</span>
                  {/if}
                  <!-- Pull success -->
                  {#if pullDone && !pullProgress[row.modelDraft]?.error}
                    <span class="po-pull-ok">✓ pulled</span>
                  {/if}
                </div>

                <!-- Pull progress bar -->
                {#if isPulling}
                  {@const prog = pullProgress[row.modelDraft]!}
                  <div class="po-pull-progress">
                    <div class="po-pull-track">
                      <div class="po-pull-fill" style="width: {prog.pct}%"></div>
                    </div>
                    <span class="po-pull-status">{prog.status}{prog.pct > 0 ? ` ${prog.pct}%` : ''}</span>
                  </div>
                {/if}
                <!-- Pull error -->
                {#if pullDone && pullProgress[row.modelDraft]?.error}
                  <div class="po-pull-error">{pullProgress[row.modelDraft].status}</div>
                {/if}
              {/if}

            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/each}
</div>

<style>
  .po-panel { display: flex; flex-direction: column; gap: 0.5rem; }

  .po-header { display: flex; align-items: center; justify-content: space-between; gap: 0.5rem; }
  .po-title  { font-size: var(--fs-sm); font-weight: 600; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em; }
  .po-actions { display: flex; align-items: center; gap: 0.5rem; }
  .po-saved  { font-size: var(--fs-sm); color: #4ade80; }
  .po-saving { font-size: var(--fs-sm); color: #64748b; }
  .po-reset  { font-size: var(--fs-sm); padding: 0.15rem 0.5rem; background: transparent; border: 1px solid #334155; border-radius: 0.3rem; color: #64748b; cursor: pointer; }
  .po-reset:hover { color: #94a3b8; border-color: #475569; }
  .po-hint   { font-size: var(--fs-sm); color: #475569; margin: 0; font-style: italic; }

  .po-section { display: flex; flex-direction: column; gap: 0.25rem; }
  .po-section-toggle { display: flex; align-items: center; justify-content: space-between; width: 100%; background: none; border: none; cursor: pointer; padding: 0.25rem 0; }
  .po-section-label  { font-size: var(--fs-sm); font-weight: 600; color: #64748b; text-transform: uppercase; letter-spacing: 0.05em; }
  .po-section-toggle:hover .po-section-label { color: #94a3b8; }
  .po-section-chevron { font-size: var(--fs-xs); color: #334155; }

  .po-list { display: flex; flex-direction: column; gap: 0.2rem; outline: none; }

  .po-item {
    background: #0f1e33;
    border: 1px solid #1e3a5f;
    border-radius: 0.4rem;
    overflow: hidden;
    transition: opacity 0.15s;
  }
  .po-item-disabled { opacity: 0.4; }

  .po-row {
    display: flex; align-items: center; gap: 0.35rem;
    padding: 0.3rem 0.5rem;
    cursor: grab; user-select: none;
  }
  .po-row:active { cursor: grabbing; }

  .po-handle { color: #334155; font-size: 1rem; line-height: 1; flex-shrink: 0; }
  .po-handle-disabled { opacity: 0.4; }

  .po-status {
    width: 7px; height: 7px; border-radius: 50%; flex-shrink: 0;
    background: #334155; border: none; padding: 0;
    cursor: pointer; transition: background 0.15s;
  }
  .po-status-ok   { background: #94a3b8; cursor: default; }
  .po-status-nokey { cursor: default; opacity: 0.4; }
  .po-status-info { background: #475569; cursor: pointer; }
  .po-status-info:hover { background: #64748b; }
  .po-status:not(.po-status-ok):not(.po-status-info):hover { background: #475569; }

  .po-badge {
    font-size: 9px; font-weight: 600; letter-spacing: 0.04em; text-transform: uppercase;
    padding: 0.05rem 0.28rem; border-radius: 0.2rem; flex-shrink: 0; line-height: 1.5;
  }
  .po-badge-local { background: transparent; color: #94a3b8; border: 1px solid #334155; }
  .po-badge-lan   { background: transparent; color: #94a3b8; border: 1px solid #334155; }
  .po-badge-api   { background: transparent; color: #94a3b8; border: 1px solid #334155; }

  .po-label { flex: 1; font-size: var(--fs-sm); font-weight: 500; color: #94a3b8; min-width: 0; }

  .po-toggle {
    position: relative; width: 28px; height: 16px; border-radius: 9999px;
    background: #1e293b; border: 1px solid #334155; cursor: pointer; padding: 0;
    transition: background 0.2s, border-color 0.2s; flex-shrink: 0;
  }
  .po-toggle-on { background: #374151; border-color: #6b7280; }
  .po-toggle-thumb {
    position: absolute; top: 2px; left: 2px; width: 10px; height: 10px; border-radius: 50%;
    background: #475569; transition: transform 0.2s, background 0.2s;
  }
  .po-toggle-on .po-toggle-thumb { transform: translateX(12px); background: #f1f5f9; }

  /* Shared sub-row (URL + key) */
  .po-sub-row {
    display: flex; align-items: center; gap: 0.3rem;
    padding: 0.3rem 0.5rem 0.35rem;
    border-top: 1px solid #1e293b; background: #0a1628;
  }
  .po-url-input, .po-key-input {
    flex: 1; background: #0f1e33; border: 1px solid #334155; border-radius: 0.3rem;
    color: #94a3b8; font-size: var(--fs-sm); padding: 0.18rem 0.4rem; outline: none; min-width: 0;
  }
  .po-key-input { font-family: monospace; }
  .po-url-input:focus, .po-key-input:focus { border-color: #3b82f6; }

  .po-probe-btn {
    font-size: var(--fs-sm); padding: 0.18rem 0.45rem;
    background: #12243d; border: 1px solid #334155; border-radius: 0.3rem;
    color: #64748b; cursor: pointer; white-space: nowrap; flex-shrink: 0;
    transition: color 0.15s, border-color 0.15s;
  }
  .po-probe-btn:hover { border-color: #475569; color: #94a3b8; }
  .po-probe-ok    { border-color: #166534 !important; color: #4ade80 !important; background: #0d2318 !important; }
  .po-probe-fail  { border-color: #7f1d1d !important; color: #f87171 !important; background: #1f0a0a !important; }
  .po-probe-pending { color: #475569 !important; }

  .po-sub-save {
    font-size: var(--fs-sm); padding: 0.18rem 0.45rem;
    background: #1e3a5f; border: 1px solid #3b82f6; border-radius: 0.3rem;
    color: #60a5fa; cursor: pointer; white-space: nowrap; flex-shrink: 0;
  }
  .po-sub-save:disabled { opacity: 0.4; cursor: not-allowed; }
  .po-sub-cancel {
    font-size: var(--fs-sm); padding: 0.18rem 0.3rem;
    background: transparent; border: 1px solid #334155; border-radius: 0.3rem;
    color: #475569; cursor: pointer; flex-shrink: 0;
  }
  .po-sub-cancel:hover { color: #64748b; }

  /* Model row */
  .po-model-row {
    display: flex; align-items: center; gap: 0.35rem;
    padding: 0.2rem 0.5rem 0.25rem;
    border-top: 1px solid #0f1e33; background: #0a1628;
  }
  .po-model-label {
    font-size: 9px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em;
    color: #334155; flex-shrink: 0;
  }
  .po-model-input {
    flex: 1; background: transparent; border: none;
    border-bottom: 1px solid #1e293b; color: #64748b;
    font-size: var(--fs-sm); padding: 0.05rem 0.1rem; outline: none; min-width: 0;
  }
  .po-model-input:focus { border-bottom-color: #3b82f6; color: #94a3b8; }
  .po-model-input::placeholder { color: #293a50; font-style: italic; font-size: 10px; }

  .po-pull-btn {
    font-size: 9px; font-weight: 600; padding: 0.1rem 0.35rem; flex-shrink: 0;
    background: #1e2d47; border: 1px solid #3b82f6; border-radius: 0.25rem;
    color: #60a5fa; cursor: pointer; text-transform: uppercase; letter-spacing: 0.04em;
  }
  .po-pull-btn:hover { background: #1e3a5f; }

  .po-not-remote {
    font-size: 9px; color: #f59e0b; flex-shrink: 0;
    font-style: italic; letter-spacing: 0.02em;
  }
  .po-pull-ok {
    font-size: 9px; color: #4ade80; flex-shrink: 0; font-weight: 600;
  }

  /* Pull progress */
  .po-pull-progress {
    display: flex; align-items: center; gap: 0.4rem;
    padding: 0.25rem 0.5rem; border-top: 1px solid #0f1e33; background: #080f1e;
  }
  .po-pull-track {
    flex: 1; height: 3px; background: #1e293b; border-radius: 9999px; overflow: hidden;
  }
  .po-pull-fill {
    height: 100%; background: #3b82f6; border-radius: 9999px; transition: width 0.3s;
  }
  .po-pull-status { font-size: 9px; color: #475569; flex-shrink: 0; }

  /* Info panel */
  .po-info-row {
    padding: 0.35rem 0.5rem;
    border-top: 1px solid #1e293b;
    background: #0a1628;
    display: flex; flex-direction: column; gap: 0.15rem;
  }
  .po-info-line {
    font-size: var(--fs-sm); color: #64748b; font-family: monospace;
    white-space: pre-wrap;
  }
  .po-info-line:first-child { color: #94a3b8; font-family: inherit; font-family: var(--font-ui, inherit); }

  .po-pull-error {
    padding: 0.2rem 0.5rem; font-size: 9px; color: #f87171;
    border-top: 1px solid #0f1e33; background: #080f1e;
  }

  /* Claude CLI status dot overrides */
  .po-status-fail { background: #7f1d1d; }
  .po-status-fail:hover { background: #991b1b; }

  /* Claude CLI info panel */
  .po-cli-status-row { display: flex; align-items: center; gap: 0.4rem; flex-wrap: wrap; }
  .po-cli-ok        { font-size: var(--fs-sm); color: #4ade80; }
  .po-cli-fail      { font-size: var(--fs-sm); color: #f87171; flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .po-cli-checking  { font-size: var(--fs-sm); color: #475569; }
  .po-cli-installing { font-size: var(--fs-sm); color: #60a5fa; }
  .po-cli-install-btn {
    font-size: var(--fs-sm); padding: 0.15rem 0.5rem; flex-shrink: 0;
    background: #1e3a5f; border: 1px solid #3b82f6; border-radius: 0.3rem;
    color: #60a5fa; cursor: pointer;
  }
  .po-cli-install-btn:hover { background: #1e4d7f; }
  .po-cli-recheck {
    font-size: var(--fs-sm); padding: 0.1rem 0.35rem; flex-shrink: 0;
    background: transparent; border: 1px solid #334155; border-radius: 0.3rem;
    color: #475569; cursor: pointer;
  }
  .po-cli-recheck:hover { color: #64748b; }
  .po-cli-log {
    background: #060e1a; border: 1px solid #1e293b; border-radius: 0.25rem;
    padding: 0.25rem 0.4rem; margin-top: 0.2rem; max-height: 6rem; overflow-y: auto;
  }
  .po-cli-log-line { font-size: 10px; font-family: monospace; color: #475569; line-height: 1.4; }
  .po-cli-code { font-size: 10px; background: #0f1e33; padding: 0.05rem 0.3rem; border-radius: 0.2rem; color: #94a3b8; }
</style>
