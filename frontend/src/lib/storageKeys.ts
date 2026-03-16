export const SK = {
  colOrder: 'col-order',
  colLeft: 'col-left',
  colHist: 'col-hist',
  colCenter: 'col-center',
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
  ttsVoice: 'tts-voice',
  ttsRate: 'tts-rate',
  ttsVolume: 'tts-volume',
  fontSize: 'font-size',
  cropRect: 'crop-rect',
  interviewerVidH: 'interviewer-vid-h',
  selfviewW: 'selfview-w',
} as const;

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
