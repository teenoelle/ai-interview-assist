export const SK = {
  colOrder: 'col-order',
  colLeft: 'col-left',
  colHist: 'col-hist',
  colCenter: 'col-center',
  colRight: 'col-right',
  rightPanelOrder: 'right-panel-order',
  sectionLayout: 'section-layout',
  collapsedSections: 'collapsed-sections',
  collapsedPanels: 'collapsed-panels',
  collapsedCols: 'collapsed-cols',
  rightSplitPct: 'right-split-pct',
  leftSplitPct: 'left-split-pct',
  histSplitPct: 'hist-split-pct',
  centerSplitPct: 'center-split-pct',
  kwBarH: 'kw-bar-h',
  zoomLeft: 'zoom-left',
  zoomHist: 'zoom-hist',
  zoomCenter: 'zoom-center',
  zoomRightTop: 'zoom-right-top',
  zoomRightBottom: 'zoom-right-bottom',
  zoomKw: 'zoom-kw',
  zoomQuestion: 'zoom-question',
  ttsVoice: 'tts-voice',
  ttsRate: 'tts-rate',
  ttsVolume: 'tts-volume',
  fontSize: 'font-size',
  uiFontSize: 'ui-font-size',
  cropRect: 'crop-rect',
  interviewerVidH: 'interviewer-vid-h',
  selfviewW: 'selfview-w',
} as const;

// Keys that represent visual layout (snapshotted by presets). Excludes TTS prefs and font size.
export const LAYOUT_KEYS: string[] = [
  SK.colOrder, SK.colLeft, SK.colHist, SK.colCenter, SK.colRight,
  SK.rightPanelOrder, SK.sectionLayout,
  SK.collapsedSections, SK.collapsedPanels, SK.collapsedCols,
  SK.rightSplitPct, SK.leftSplitPct, SK.histSplitPct, SK.centerSplitPct,
  SK.kwBarH,
  SK.zoomLeft, SK.zoomHist, SK.zoomCenter, SK.zoomRightTop, SK.zoomRightBottom, SK.zoomKw,
  SK.cropRect, SK.interviewerVidH, SK.selfviewW,
];

const PRESET_INDEX_KEY = 'layout-presets';

export function saveLayoutPreset(name: string): void {
  const snap: Record<string, string | null> = {};
  LAYOUT_KEYS.forEach(k => { snap[k] = localStorage.getItem(k); });
  localStorage.setItem(`layout-preset:${name}`, JSON.stringify(snap));
  const index = listLayoutPresets();
  if (!index.includes(name)) {
    localStorage.setItem(PRESET_INDEX_KEY, JSON.stringify([...index, name]));
  }
}

export function loadLayoutPreset(name: string): boolean {
  const raw = localStorage.getItem(`layout-preset:${name}`);
  if (!raw) return false;
  const snap: Record<string, string | null> = JSON.parse(raw);
  Object.entries(snap).forEach(([k, v]) => {
    if (v === null) localStorage.removeItem(k); else localStorage.setItem(k, v);
  });
  return true;
}

export function deleteLayoutPreset(name: string): void {
  localStorage.removeItem(`layout-preset:${name}`);
  localStorage.setItem(PRESET_INDEX_KEY, JSON.stringify(listLayoutPresets().filter(n => n !== name)));
}

export function listLayoutPresets(): string[] {
  try { return JSON.parse(localStorage.getItem(PRESET_INDEX_KEY) ?? '[]'); }
  catch { return []; }
}

interface SectionSlot { panel: string; id: string; }

export function loadSectionLayout(defaultLayout: SectionSlot[]): SectionSlot[] {
  try {
    const stored = localStorage.getItem(SK.sectionLayout);
    const base: SectionSlot[] = stored ? JSON.parse(stored) : [...defaultLayout];
    for (const def of defaultLayout) {
      if (!base.find(s => s.id === def.id)) base.push({ ...def });
    }
    return base;
  } catch {
    return [...defaultLayout];
  }
}
