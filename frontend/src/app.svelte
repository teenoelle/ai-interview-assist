<script lang="ts">
  import SetupForm from './components/SetupForm.svelte';
  import CaptureButton from './components/CaptureButton.svelte';
  import TranscriptPanel from './components/TranscriptPanel.svelte';
  import SentimentBar from './components/SentimentBar.svelte';
  import BodyLanguagePanel from './components/BodyLanguagePanel.svelte';
  import SuggestionPanel from './components/SuggestionPanel.svelte';
  import TestQuestionBar from './components/TestQuestionBar.svelte';
  import RateLimitPanel from './components/RateLimitPanel.svelte';
  import DebriefModal from './components/DebriefModal.svelte';
  import ReviewUpload from './components/ReviewUpload.svelte';
  import ReviewPanel from './components/ReviewPanel.svelte';
  import type { ReviewReport } from './components/ReviewPanel.svelte';
  import PracticePanel from './components/PracticePanel.svelte';
  import InterviewHistoryPanel from './components/InterviewHistoryPanel.svelte';
  import WhisperOverlay from './components/WhisperOverlay.svelte';
  import QuestionsHistoryPanel from './components/QuestionsHistoryPanel.svelte';
  import CompanyBriefPanel from './components/CompanyBriefPanel.svelte';
  import InterviewerProfilePanel from './components/InterviewerProfilePanel.svelte';
  import KeywordTrackerPanel from './components/KeywordTrackerPanel.svelte';
  import SalaryCoachPanel from './components/SalaryCoachPanel.svelte';
  import NextQuestionPanel from './components/NextQuestionPanel.svelte';
  import EnergyCoachPanel from './components/EnergyCoachPanel.svelte';
  import { analyzeAudioTone, type AudioFeatures } from './lib/audioTone';
  import { splitMultiQuestions, fmtTime, fmtAgo } from './lib/utils';
  import { computeConfidence } from './lib/confidence';
  import { SK, loadSectionLayout } from './lib/storageKeys';
  import { applyDrop, moveToPanel } from './lib/dragLayout';
  import { EventWebSocket } from './lib/websocket';
  import { countFillers, totalFillers } from './lib/filler';
  import { saveInterview } from './lib/interviewHistory';
  import { tagQuestion } from './lib/questionTagger';
  import { detectRedFlag } from './lib/redFlagDetector';
  import { derivePersonality } from './lib/personalityTracker';
  import { loadKeywords, checkMentioned } from './lib/keywordTracker';
  import { isSalaryQuestion } from './lib/salaryDetector';
  import { analyzePace, detectEnergySignals } from './lib/paceCoach';
  import type { TranscriptEntry, SuggestionEntry, WsEvent } from './lib/types';
  import type { FillerCount } from './lib/filler';
  import type { MediaCapture } from './lib/capture';
  import type { CompanyBrief, InterviewerSummary } from './lib/api';
  import { fetchUsage, authFetch } from './lib/api';
  import { parseSuggestion, parseCues } from './lib/parseSuggestion';
  import { EMOTION_COLORS, EMOTION_CONFIG, emotionColor, POSITIVE_EMOTIONS, NEGATIVE_EMOTIONS } from './lib/emotions';
  import { tooltip } from './lib/tooltip';
  import '@fontsource/inter/400.css';
  import '@fontsource/inter/600.css';
  import '@fontsource/inter/700.css';
  import '@fontsource/dm-sans/400.css';
  import '@fontsource/dm-sans/600.css';
  import '@fontsource/dm-sans/700.css';
  import '@fontsource/plus-jakarta-sans/400.css';
  import '@fontsource/plus-jakarta-sans/600.css';
  import '@fontsource/plus-jakarta-sans/700.css';
  import '@fontsource/ibm-plex-sans/400.css';
  import '@fontsource/ibm-plex-sans/600.css';
  import '@fontsource/ibm-plex-sans/700.css';
  import '@fontsource/outfit/400.css';
  import '@fontsource/outfit/600.css';
  import '@fontsource/outfit/700.css';

  type Phase = 'setup' | 'practice' | 'interview';

  let phase = $state<Phase>('setup');
  let capturing = $state(false);
  let transcript = $state<TranscriptEntry[]>([]);
  let emotion = $state('');
  let emotionReason = $state('');
  let coaching = $state('');
  let coachingWhy = $state('');
  let coachingLog = $state<{ text: string; why?: string; reason?: string; emotion: string; time: number; type?: 'personality'; color?: string }[]>([]);
  let pendingCoachingEntry = $state<typeof coachingLog[0] | null>(null);
  let youLog = $state<{ text: string; time: number }[]>([]);
  let expandedYouEntries = $state(new Set<number>());
  let youDeliveryExpanded = $state(true);
  let suggestions = $state<SuggestionEntry[]>([]);
  let statusMessages = $state<string[]>([]);
  let errorMessages = $state<string[]>([]);
  let predictedQuestions = $state<string[]>([]);
  let showDebrief = $state(false);
  let focusMode = $state(false);
  let showHistory = $state(false);
  let showReviewUpload = $state(false);
  let showReviewPanel = $state(false);
  let reviewReport = $state<ReviewReport | null>(null);
  let savingLiveReport = $state(false);
  let reviewList = $state<ReviewReport[]>([]);
  let reviewSearch = $state('');
  let showReviewList = $state(false);

  async function loadReviewList() {
    try {
      const resp = await authFetch('/api/reviews');
      if (resp.ok) reviewList = await resp.json();
    } catch { /* ignore */ }
  }

  async function deleteReview(id: string) {
    try {
      await authFetch(`/api/review/${id}`, { method: 'DELETE' });
      reviewList = reviewList.filter(r => r.id !== id);
    } catch { /* ignore */ }
  }

  const filteredReviews = $derived(
    reviewSearch.trim()
      ? reviewList.filter(r =>
          (r.source_filename ?? '').toLowerCase().includes(reviewSearch.toLowerCase())
        )
      : reviewList
  );
  let showWhisper = $state(false);
  let emotionHistory = $state<string[]>([]);
  let vocalWhyExpanded = $state(false);
  let answerWhyExpanded = $state(false);
  let expandedCoachingEntries = $state(new Set<number>());
  $effect(() => {
    if (expandedCoachingEntries.size === 0 && pendingCoachingEntry) {
      coachingLog = [...coachingLog.slice(-4), pendingCoachingEntry];
      pendingCoachingEntry = null;
    }
  });


  // Setup-time data (persisted via localStorage so they survive back-navigation and practice→interview flow)
  function loadSetup<T>(key: string, fallback: T): T {
    try { const v = localStorage.getItem(key); return v ? JSON.parse(v) : fallback; } catch { return fallback; }
  }
  let companyBrief = $state<CompanyBrief | null>(loadSetup('setup-company-brief', null));
  let interviewerSummaries = $state<InterviewerSummary[]>(loadSetup('setup-interviewer-summaries', []));

  // Keyword tracker
  let jdKeywords = $state<string[]>(loadKeywords());
  let mentionedKeywords = $state<Set<string>>(new Set());
  let interviewerRaisedKeywords = $state<Set<string>>(new Set());
  let flashingKeywords = $state<Set<string>>(new Set());
  let flashTimer: ReturnType<typeof setTimeout> | null = null;

  // Salary coach
  let salaryTactics = $state<{ deflect: string; anchor: string; counter_range: string; never_say: string } | null>(null);
  let loadingSalary = $state(false);

  // Next question predictor
  let nextQuestions = $state<string[]>([]);
  let loadingNextQ = $state(false);

  // Pace / energy coach
  let recentYouTexts = $state<{ text: string; time: number }[]>([]);
  const PACE_WINDOW_SEC = 30;
  const paceReading = $derived(
    analyzePace(
      recentYouTexts.filter(t => Date.now() - t.time < PACE_WINDOW_SEC * 1000).map(t => t.text),
      PACE_WINDOW_SEC
    )
  );
  const energySignal = $derived(
    detectEnergySignals(recentYouTexts.slice(-6).map(t => t.text))
  );

  // Capture instance (for triggerFrameCapture)
  let captureInst = $state<MediaCapture | null>(null);
  let lastSentimentTrigger = 0;
  // Sync crop rect to capture instance whenever either changes
  $effect(() => { captureInst?.setCropRect(cropRect); });

  // Rolling exponential moving average of system audio energy (RMS)
  let sysEnergyEma = $state(0);
  function updateSysEnergy(sys: number) {
    sysEnergyEma = sysEnergyEma * 0.85 + sys * 0.15;
  }

  // Spectral audio features from AnalyserNode (item 8)
  let latestAudioFeatures = $state<AudioFeatures>({});

  // Client-side face emotion from MediaPipe (item 5)
  let clientFaceEmotion = $state('');
  let lastBackendSentimentTime = $state(0);

  // Wire up captureInst callbacks for live emotion + audio features
  $effect(() => {
    if (!captureInst) return;
    captureInst.onAudioFeatures(f => { latestAudioFeatures = f; });
    captureInst.onLiveEmotion(emo => {
      // Fill in between backend reads; backend takes precedence for 10s after it updates
      if (Date.now() - lastBackendSentimentTime > 10000) {
        clientFaceEmotion = emo;
      }
    });
  });

  // Webcam self-view
  let webcamStream = $state<MediaStream | null>(null);
  let webcamEl: HTMLVideoElement | undefined = $state();
  $effect(() => {
    if (webcamEl) { webcamEl.srcObject = webcamStream ?? null; if (webcamStream) webcamEl.play().catch(() => {}); }
  });

  // Screen share preview (shows interviewer's video in Zoom/Teams)
  let screenStream = $state<MediaStream | null>(null);
  let screenEl: HTMLVideoElement | undefined = $state();
  $effect(() => {
    if (screenEl) { screenEl.srcObject = screenStream ?? null; if (screenStream) screenEl.play().catch(() => {}); }
  });

  // Interviewer face crop — always starts null (full screen) regardless of saved value
  let cropRect = $state<{x:number;y:number;w:number;h:number}|null>(null);
  let videoNaturalAR = $state(16/9);
  let showCropPicker = $state(false);
  let pickerDrag = $state<{sx:number;sy:number;cx:number;cy:number}|null>(null);
  let pickerPendingRect = $state<{x:number;y:number;w:number;h:number}|null>(null);
  let pickerVideoEl: HTMLVideoElement | undefined = $state();
  let focusVideoEl: HTMLVideoElement | undefined = $state();
  let vidZoomShellEl = $state<HTMLElement | undefined>();

  // Video overlay resize (persisted)
  let interviewerVidH = $state(Number(localStorage.getItem(SK.interviewerVidH) || '0') || 0);
  let selfviewH = $state(Number(localStorage.getItem(SK.selfviewW) || '120') || 120);
  let iVidResizing = false, iVidResizeStartY = 0, iVidResizeStartH = 0;
  let selfviewResizing = false, selfviewResizeStartY = 0, selfviewResizeStartH = 0;

  function iVidResizeDown(e: PointerEvent) {
    iVidResizing = true;
    iVidResizeStartY = e.clientY;
    iVidResizeStartH = interviewerVidH || vidZoomShellEl?.getBoundingClientRect().height || 200;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    e.preventDefault(); e.stopPropagation();
  }
  function iVidResizeMove(e: PointerEvent) {
    if (!iVidResizing) return;
    interviewerVidH = Math.max(60, Math.min(600, iVidResizeStartH + (e.clientY - iVidResizeStartY)));
  }
  function iVidResizeUp() {
    if (!iVidResizing) return;
    iVidResizing = false;
    localStorage.setItem(SK.interviewerVidH, String(interviewerVidH));
  }
  function selfviewResizeDown(e: PointerEvent) {
    selfviewResizing = true;
    selfviewResizeStartY = e.clientY;
    selfviewResizeStartH = selfviewH;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    e.preventDefault(); e.stopPropagation();
  }
  function selfviewResizeMove(e: PointerEvent) {
    if (!selfviewResizing) return;
    selfviewH = Math.max(60, selfviewResizeStartH + (e.clientY - selfviewResizeStartY));
  }
  function selfviewResizeUp() {
    if (!selfviewResizing) return;
    selfviewResizing = false;
    localStorage.setItem(SK.selfviewW, String(selfviewH));
  }

  $effect(() => {
    const el = screenEl;
    if (!el) return;
    const onMeta = () => { if (el.videoWidth) videoNaturalAR = el.videoWidth / el.videoHeight; };
    if (el.readyState >= 1 && el.videoWidth) { onMeta(); return; }
    el.addEventListener('loadedmetadata', onMeta, { once: true });
  });
  $effect(() => { if (pickerVideoEl) { pickerVideoEl.srcObject = screenStream ?? null; if (screenStream) pickerVideoEl.play().catch(() => {}); } });
  $effect(() => { if (focusVideoEl) { focusVideoEl.srcObject = screenStream ?? null; if (screenStream) focusVideoEl.play().catch(() => {}); } });

  // Column widths (resizable, persisted)
  let leftW = $state(Number(localStorage.getItem(SK.colLeft) ?? 240));
  let histW = $state(Number(localStorage.getItem(SK.colHist) ?? 180));
  let centerW = $state(Number(localStorage.getItem(SK.colCenter) ?? 320));
  let rightW = $state(Number(localStorage.getItem(SK.colRight) ?? 220));

  // Per-panel zoom (persisted)
  let leftZoom = $state(Number(localStorage.getItem(SK.zoomLeft) ?? 100));
  let histZoom = $state(Number(localStorage.getItem(SK.zoomHist) ?? 100));
  let centerZoom = $state(Number(localStorage.getItem(SK.zoomCenter) ?? 100));
  let rightTopZoom = $state(Number(localStorage.getItem(SK.zoomRightTop) ?? 100));
  let rightBottomZoom = $state(Number(localStorage.getItem(SK.zoomRightBottom) ?? 100));
  let kwZoom = $state(Number(localStorage.getItem(SK.zoomKw) ?? 100));
  let rightSplitPct = $state(Number(localStorage.getItem(SK.rightSplitPct) ?? 42));
  let leftSplitPct = $state(Number(localStorage.getItem(SK.leftSplitPct) ?? 55));
  let rightColBodyEl = $state<HTMLElement | undefined>();
  let leftColBodyEl = $state<HTMLElement | undefined>();
  let kwBarH = $state(Number(localStorage.getItem(SK.kwBarH) ?? 60));

  // Collapse state (persisted)
  let modelUsageExpanded = $state(localStorage.getItem('model-usage-expanded') !== 'false');
  let collapsedSections = $state<Set<string>>(new Set(JSON.parse(localStorage.getItem(SK.collapsedSections) ?? '[]')));
  let collapsedPanels = $state<Set<string>>(new Set(JSON.parse(localStorage.getItem(SK.collapsedPanels) ?? '[]')));
  let collapsedCols = $state<Set<string>>(new Set(JSON.parse(localStorage.getItem(SK.collapsedCols) ?? '[]')));

  function toggleColCollapse(col: string) {
    const s = new Set(collapsedCols);
    s.has(col) ? s.delete(col) : s.add(col);
    collapsedCols = s;
    localStorage.setItem(SK.collapsedCols, JSON.stringify([...s]));
  }
  function togglePanelCollapse(panel: string) {
    const s = new Set(collapsedPanels);
    s.has(panel) ? s.delete(panel) : s.add(panel);
    collapsedPanels = s;
    localStorage.setItem(SK.collapsedPanels, JSON.stringify([...s]));
  }
  function toggleSectionCollapse(id: string) {
    const s = new Set(collapsedSections);
    s.has(id) ? s.delete(id) : s.add(id);
    collapsedSections = s;
    localStorage.setItem(SK.collapsedSections, JSON.stringify([...s]));
  }

  let histSplitPct = $state(Number(localStorage.getItem(SK.histSplitPct) ?? 60));
  let centerSplitPct = $state(Number(localStorage.getItem(SK.centerSplitPct) ?? 60));
  let histColBodyEl = $state<HTMLElement | undefined>();
  let centerColBodyEl = $state<HTMLElement | undefined>();

  function startHistResize(e: MouseEvent) {
    e.preventDefault();
    const onMove = (ev: MouseEvent) => {
      if (!histColBodyEl) return;
      const rect = histColBodyEl.getBoundingClientRect();
      const pct = Math.max(15, Math.min(85, ((ev.clientY - rect.top) / rect.height) * 100));
      histSplitPct = pct;
      localStorage.setItem(SK.histSplitPct, String(pct));
    };
    const onUp = () => { window.removeEventListener('mousemove', onMove); window.removeEventListener('mouseup', onUp); };
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
  }

  function startCenterResize(e: MouseEvent) {
    e.preventDefault();
    const onMove = (ev: MouseEvent) => {
      if (!centerColBodyEl) return;
      const rect = centerColBodyEl.getBoundingClientRect();
      const pct = Math.max(15, Math.min(85, ((ev.clientY - rect.top) / rect.height) * 100));
      centerSplitPct = pct;
      localStorage.setItem(SK.centerSplitPct, String(pct));
    };
    const onUp = () => { window.removeEventListener('mousemove', onMove); window.removeEventListener('mouseup', onUp); };
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
  }

  function startLeftResize(e: MouseEvent) {
    e.preventDefault();
    const onMove = (ev: MouseEvent) => {
      if (!leftColBodyEl) return;
      const rect = leftColBodyEl.getBoundingClientRect();
      const pct = Math.max(15, Math.min(85, ((ev.clientY - rect.top) / rect.height) * 100));
      leftSplitPct = pct;
      localStorage.setItem(SK.leftSplitPct, String(pct));
    };
    const onUp = () => {
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
    };
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
  }

  function startKwResize(e: MouseEvent) {
    e.preventDefault();
    const startY = e.clientY;
    const startH = kwBarH;
    const onMove = (ev: MouseEvent) => {
      kwBarH = Math.max(36, Math.min(300, startH - (ev.clientY - startY)));
      localStorage.setItem(SK.kwBarH, String(kwBarH));
    };
    const onUp = () => {
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
    };
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
  }

  function startRightResize(e: MouseEvent) {
    e.preventDefault();
    const onMove = (ev: MouseEvent) => {
      if (!rightColBodyEl) return;
      const rect = rightColBodyEl.getBoundingClientRect();
      const pct = Math.max(15, Math.min(80, ((ev.clientY - rect.top) / rect.height) * 100));
      rightSplitPct = pct;
      localStorage.setItem(SK.rightSplitPct, String(pct));
    };
    const onUp = () => {
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
    };
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
  }

  // Jump signal for SuggestionPanel cross-navigation from QuestionsHistoryPanel
  let jumpSignal = $state<{ idx: number; key: number } | null>(null);

  // New question notification
  let suggestionPinned = $state(true);
  let unseenCount = $state(0);
  let scrollToLatestKey = $state(0);
  let prevSuggestionsLen = $state(0);
  // Tracks which question index is currently viewed in SuggestionPanel (-1 = latest)
  let histViewIdx = $state(-1);

  $effect(() => {
    if (suggestions.length > prevSuggestionsLen && prevSuggestionsLen > 0) {
      if (suggestionPinned) scrollToLatestKey++;
      else unseenCount++;
    }
    prevSuggestionsLen = suggestions.length;
  });

  $effect(() => {
    if (suggestionPinned) { unseenCount = 0; histViewIdx = -1; }
  });

  function adjustZoom(col: 'left' | 'hist' | 'center' | 'rightTop' | 'rightBottom' | 'kw' | 'all', delta: number) {
    if (col === 'left' || col === 'all') {
      leftZoom = Math.max(20, leftZoom + delta);
      localStorage.setItem(SK.zoomLeft, String(leftZoom));
    }
    if (col === 'hist' || col === 'all') {
      histZoom = Math.max(20, histZoom + delta);
      localStorage.setItem(SK.zoomHist, String(histZoom));
    }
    if (col === 'center' || col === 'all') {
      centerZoom = Math.max(20, centerZoom + delta);
      localStorage.setItem(SK.zoomCenter, String(centerZoom));
    }
    if (col === 'rightTop' || col === 'all') {
      rightTopZoom = Math.max(20, rightTopZoom + delta);
      localStorage.setItem(SK.zoomRightTop, String(rightTopZoom));
    }
    if (col === 'rightBottom' || col === 'all') {
      rightBottomZoom = Math.max(20, rightBottomZoom + delta);
      localStorage.setItem(SK.zoomRightBottom, String(rightBottomZoom));
    }
    if (col === 'kw' || col === 'all') {
      kwZoom = Math.max(20, kwZoom + delta);
      localStorage.setItem(SK.zoomKw, String(kwZoom));
    }
  }

  function startResize(col: 'left' | 'hist' | 'center' | 'right', e: MouseEvent) {
    e.preventDefault();
    const startX = e.clientX;
    const startW = col === 'left' ? leftW : col === 'hist' ? histW : col === 'center' ? centerW : rightW;
    const vw = window.innerWidth;
    // Reserve space for right panels so they never get clipped
    const rightReserved = (collapsedCols.has('right') ? 28 : rightW)
                        + (collapsedCols.has('right2') ? 28 : 120)
                        + 25; // handle widths
    const colW = (c: string) => collapsedCols.has(c) ? 28 : (c === 'left' ? leftW : c === 'hist' ? histW : centerW);
    const otherLeftCols: Record<string, number> = {
      left: colW('hist') + colW('center'),
      hist: colW('left') + colW('center'),
      center: colW('left') + colW('hist'),
      right: colW('left') + colW('hist') + colW('center'),
    };
    const dynamicMax = Math.max(120, vw - rightReserved - otherLeftCols[col] - 10);
    const [min, max] = col === 'left' ? [130, Math.min(Math.floor(vw * 0.4), dynamicMax)] : col === 'hist' ? [120, Math.min(Math.floor(vw * 0.35), dynamicMax)] : col === 'center' ? [180, Math.min(Math.floor(vw * 0.6), dynamicMax)] : [120, Math.min(Math.floor(vw * 0.5), dynamicMax)];
    function onMove(ev: MouseEvent) {
      const w = Math.max(min, Math.min(max, startW + ev.clientX - startX));
      if (col === 'left') leftW = w;
      else if (col === 'hist') histW = w;
      else if (col === 'center') centerW = w;
      else rightW = w;
    }
    function onUp() {
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
      localStorage.setItem(SK.colLeft, String(leftW));
      localStorage.setItem(SK.colHist, String(histW));
      localStorage.setItem(SK.colCenter, String(centerW));
      localStorage.setItem(SK.colRight, String(rightW));
    }
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
  }

  // Column drag-to-reorder
  let colOrder = $state<string[]>(JSON.parse(localStorage.getItem(SK.colOrder) ?? '["left","hist","center","right","right2"]'));
  let rightPanelOrder = $state<string[]>(JSON.parse(localStorage.getItem(SK.rightPanelOrder) ?? '["sentiment","coaching"]'));
  let draggingCol = $state<string | null>(null);
  let draggingPanel = $state<string | null>(null);

  function onColDragStart(col: string, e: DragEvent) {
    draggingCol = col;
    if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
  }
  function onColDragOver(e: DragEvent) { e.preventDefault(); }
  function onColDrop(targetCol: string, e: DragEvent) {
    e.preventDefault();
    if (!draggingCol || draggingCol === targetCol) { draggingCol = null; return; }
    const order = [...colOrder];
    const fi = order.indexOf(draggingCol), ti = order.indexOf(targetCol);
    if (fi < 0 || ti < 0) { draggingCol = null; return; }
    order.splice(fi, 1); order.splice(ti, 0, draggingCol);
    colOrder = order;
    localStorage.setItem(SK.colOrder, JSON.stringify(order));
    draggingCol = null;
  }
  function onPanelDragStart(panel: string, e: DragEvent) {
    draggingPanel = panel;
    if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
  }
  function onPanelDragOver(e: DragEvent) { e.preventDefault(); }
  function onPanelDrop(targetPanel: string, e: DragEvent) {
    e.preventDefault();
    if (!draggingPanel || draggingPanel === targetPanel) { draggingPanel = null; return; }
    rightPanelOrder = [...rightPanelOrder].reverse();
    localStorage.setItem(SK.rightPanelOrder, JSON.stringify(rightPanelOrder));
    draggingPanel = null;
  }

  // Section drag-to-reorder (within and between right sub-panels)
  type SectionId = 'screen-preview' | 'personality' | 'sentiment-bar' | 'body-language' |
    'energy-coach' | 'fillers' | 'salary-coach' | 'next-question' | 'keywords' |
    'company-brief' | 'interviewer-profiles' | 'stats' | 'rate-limits' | 'answer-score';
  interface SectionSlot { panel: string; id: SectionId; }
  const DEFAULT_SECTION_LAYOUT: SectionSlot[] = [
    { panel: 'sentiment', id: 'screen-preview' },
    { panel: 'sentiment', id: 'sentiment-bar' },
    { panel: 'sentiment', id: 'body-language' },
    { panel: 'sentiment', id: 'stats' },
    { panel: 'sentiment', id: 'answer-score' },
    { panel: 'coaching', id: 'salary-coach' },
    { panel: 'coaching', id: 'next-question' },
    { panel: 'coaching', id: 'company-brief' },
    { panel: 'coaching', id: 'interviewer-profiles' },
    { panel: 'coaching', id: 'rate-limits' },
  ];
  const SECTION_LABELS: Record<string, string> = {
    'screen-preview': 'Interviewer Video', 'personality': 'Interviewer Personality', 'sentiment-bar': 'Interviewer Mood',
    'body-language': 'Your Body Language', 'energy-coach': 'Pace', 'fillers': 'Fillers',
    'salary-coach': 'Salary', 'next-question': 'Next Q', 'keywords': 'Keywords',
    'company-brief': 'Company', 'interviewer-profiles': 'Interviewers',
    'stats': 'Your Stats', 'rate-limits': 'Model Usage', 'answer-score': 'Your Answer Score',
  };
  const SECTION_ROLE: Record<string, 'interviewer' | 'you' | 'coaching'> = {
    'screen-preview': 'interviewer', 'personality': 'interviewer', 'sentiment-bar': 'interviewer',
    'body-language': 'you', 'stats': 'you', 'answer-score': 'you', 'energy-coach': 'you', 'fillers': 'you',
    'salary-coach': 'coaching', 'next-question': 'coaching', 'keywords': 'coaching',
    'company-brief': 'coaching', 'interviewer-profiles': 'coaching', 'rate-limits': 'coaching',
  };
  let sectionLayout = $state<SectionSlot[]>(loadSectionLayout(DEFAULT_SECTION_LAYOUT));
  let draggingSection = $state<SectionId | null>(null);
  let sectionDropTarget = $state<{ id: SectionId; pos: 'above' | 'below' } | null>(null);

  function onSectionDragStart(id: SectionId, e: DragEvent) {
    draggingSection = id;
    if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
  }
  function onSectionDragOver(id: SectionId, e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const pos: 'above' | 'below' = e.clientY < rect.top + rect.height / 2 ? 'above' : 'below';
    sectionDropTarget = { id, pos };
  }
  function onSectionDragLeave(e: DragEvent) {
    // Only clear if leaving to outside the wrapper (not into a child)
    if (!(e.currentTarget as HTMLElement).contains(e.relatedTarget as Node)) {
      sectionDropTarget = null;
    }
  }
  function onSectionDrop(targetId: SectionId, targetPanel: string, e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();
    if (!draggingSection) { sectionDropTarget = null; return; }
    if (draggingSection === targetId) { draggingSection = null; sectionDropTarget = null; return; }
    const pos = sectionDropTarget?.pos ?? 'below';
    const layout = applyDrop(sectionLayout, draggingSection, targetId, targetPanel, pos);
    sectionLayout = layout;
    localStorage.setItem(SK.sectionLayout, JSON.stringify(layout));
    draggingSection = null;
    sectionDropTarget = null;
  }
  function onPanelEmptyDrop(panel: string, e: DragEvent) {
    e.preventDefault();
    if (!draggingSection) return;
    const layout = moveToPanel(sectionLayout, draggingSection, panel);
    sectionLayout = layout;
    localStorage.setItem(SK.sectionLayout, JSON.stringify(layout));
    draggingSection = null;
    sectionDropTarget = null;
  }
  function onSectionDragEnd() {
    draggingSection = null;
    sectionDropTarget = null;
  }

  // TTS voice hints
  import * as ttsClient from './lib/ttsClient';
  import type { CombinedVoice } from './lib/ttsClient';

  const FONTS = [
    { id: 'Inter',              label: 'Inter' },
    { id: 'DM Sans',            label: 'DM Sans' },
    { id: 'Plus Jakarta Sans',  label: 'Plus Jakarta Sans' },
    { id: 'IBM Plex Sans',      label: 'IBM Plex Sans' },
    { id: 'Outfit',             label: 'Outfit' },
    { id: 'system-ui',          label: 'System Default' },
  ] as const;
  let appFont = $state(localStorage.getItem('app-font') ?? 'Inter');
  $effect(() => {
    const stack = appFont === 'system-ui'
      ? 'system-ui, -apple-system, sans-serif'
      : `'${appFont}', system-ui, sans-serif`;
    document.documentElement.style.setProperty('--ff-base', stack);
    localStorage.setItem('app-font', appFont);
  });

  let ttsEnabled = $state(false);
  let ttsVoices = $state<CombinedVoice[]>([]);
  let ttsVoiceId = $state(localStorage.getItem('tts-voice-id') ?? localStorage.getItem(SK.ttsVoice) ?? '');
  let ttsRate = $state(Number(localStorage.getItem(SK.ttsRate) ?? 1.5));
  let ttsVolume = $state(Math.max(0.1, Number(localStorage.getItem(SK.ttsVolume) ?? 1.0)));
  let showVoiceMenu = $state(false);
  // Silence gating: track last time anyone spoke
  let lastSpeechAt = 0; // ms timestamp
  const TTS_SILENCE_GAP_MS = 2500;

  $effect(() => {
    ttsClient.loadAllVoices().then(all => {
      ttsVoices = all;
      if (!ttsVoiceId || !all.find(v => v.id === ttsVoiceId)) ttsVoiceId = all[0]?.id ?? '';
    });
  });
  $effect(() => { localStorage.setItem(SK.ttsRate, String(ttsRate)); });
  $effect(() => { localStorage.setItem(SK.ttsVolume, String(ttsVolume)); });
  $effect(() => { if (ttsVoiceId) localStorage.setItem('tts-voice-id', ttsVoiceId); });

  function speakText(text: string) {
    if (!ttsEnabled || !text) return;
    if (Date.now() - lastSpeechAt < TTS_SILENCE_GAP_MS) return;
    if (answerStartTime !== null) return;
    const parsed = parseSuggestion(text);
    if (!parsed.acknowledge) return;
    const toSpeak = [parsed.acknowledge, parsed.solve, parsed.bridge, parsed.tell, parsed.close].filter(Boolean).join(" ");
    ttsClient.speak(toSpeak, ttsVoiceId, ttsRate, ttsVolume);
    lastSpeechAt = Date.now();
  }

  // Audio sentiment (client-side, free — based on interviewer text)
  let audioEmotion = $state('');
  let audioReason = $state('');

  // Body language panel state
  let consecutiveEmotionCount = $state(0);
  let lastTrackedEmotion = $state('');
  let transitionNote = $state<{ from: string; to: string } | null>(null);
  let transitionTimer: ReturnType<typeof setTimeout> | null = null;
  const positiveStreak = $derived(consecutiveEmotionCount >= 2 && POSITIVE_EMOTIONS.has(emotion));
  let blTriggerCounts = $state<Record<string, number>>(loadSetup('bl-trigger-counts', {}));
  let answerNudgeVisible = $state(false);
  let answerNudgeTimer: ReturnType<typeof setTimeout> | null = null;
  let presenceIssues = $state<string[]>([]);
  let presencePositive = $state<string | null>(null);
  let prevPresenceHadIssues = false;
  let lastPosturePositiveTime = 0;

  // Speaker mode for body-language hints
  const speakerMode = $derived<'listening' | 'answering' | 'idle'>(
    answerMs > 0 ? 'answering' :
    (transcript.length > 0 && transcript[transcript.length - 1]?.speaker === 'Interviewer') ? 'listening' :
    'idle'
  );

  // Webcam presence check (item 9)
  async function checkPresence() {
    const el = webcamEl;
    if (!el || el.videoWidth === 0) return;
    const canvas = document.createElement('canvas');
    canvas.width = 320; canvas.height = 240;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    ctx.drawImage(el, 0, 0, canvas.width, canvas.height);
    const blob = await new Promise<Blob | null>(r => canvas.toBlob(r, 'image/jpeg', 0.7));
    if (!blob) return;
    const fd = new FormData();
    fd.append('image', blob, 'webcam.jpg');
    try {
      const resp = await authFetch('/api/presence-check', { method: 'POST', body: fd });
      if (resp.ok) {
        const data = await resp.json();
        const newIssues: string[] = data.issues ?? [];
        const positive: string | null = data.positive ?? null;
        // Note positive body language: on transition from issues, or proactively if looking good (throttled 5 min)
        const now = Date.now();
        const isPositiveCheck = newIssues.length === 0 && positive !== null;
        const throttleOk = now - lastPosturePositiveTime > 5 * 60 * 1000;
        if (isPositiveCheck && (prevPresenceHadIssues || throttleOk)) {
          const fixText = positive || 'Body language looks good';
          youLog = [...youLog.slice(-4), { text: fixText, time: Date.now() }];
          lastPosturePositiveTime = now;
        }
        prevPresenceHadIssues = newIssues.length > 0;
        presenceIssues = newIssues;
        presencePositive = positive;
      }
    } catch { /* silent — webcam analysis is best-effort */ }
  }

  // Run presence check periodically while capturing with webcam
  $effect(() => {
    if (!capturing || !webcamStream) return;
    const timeout = setTimeout(checkPresence, 8000);
    const interval = setInterval(checkPresence, 30000);
    return () => { clearTimeout(timeout); clearInterval(interval); };
  });

  // Font size
  let fontSize = $state(Number(localStorage.getItem(SK.fontSize) ?? 14));
  $effect(() => {
    document.documentElement.style.setProperty('--font-size', `${fontSize}px`);
    localStorage.setItem(SK.fontSize, String(fontSize));
  });

  // Stats
  let answerStartTime = $state<number | null>(null);
  let answerMs = $state(0);
  let youSegments = $state(0);
  let interviewerSegments = $state(0);
  let allFillerCounts = $state<FillerCount[]>([]);

  // Answer feedback tracking
  let currentQuestionIdx = $state(-1);
  let youSegmentsSinceQuestion = $state<string[]>([]);

  let answerInterval: ReturnType<typeof setInterval> | null = null;

  function startAnswerTimer() {
    if (answerStartTime !== null) return;
    answerStartTime = Date.now();
    answerInterval = setInterval(() => {
      if (answerStartTime !== null) answerMs = Date.now() - answerStartTime;
    }, 500);
  }

  function resetAnswerTimer() {
    answerStartTime = null;
    answerMs = 0;
    if (answerInterval) { clearInterval(answerInterval); answerInterval = null; }
  }

  const youPct = $derived(
    youSegments + interviewerSegments === 0 ? 0 :
    Math.round((youSegments / (youSegments + interviewerSegments)) * 100)
  );
  const interviewerPct = $derived(youPct > 0 ? 100 - youPct : 0);
  const fillerTotal = $derived(totalFillers(allFillerCounts));
  const timerColor = $derived(
    answerMs === 0 ? '#475569' :
    answerMs < 15000 ? '#22c55e' :
    answerMs < 30000 ? '#f59e0b' : '#ef4444'
  );
  const ratioColor = $derived(
    youPct === 0 ? '#475569' : youPct < 65 ? '#22c55e' : '#f59e0b'
  );
  function extractTell(suggestion: string): string {
    for (const line of suggestion.split('\n')) {
      const m = line.match(/^(?:Say|Tell):\s*(.+)/i);
      if (m) return m[1].trim();
    }
    return suggestion.split('\n')[0]?.trim() ?? '';
  }
  const latestMissed = $derived(
    suggestions.slice().reverse().find(s => s.confidenceScore != null && s.confidenceScore < 40 && s.missedKeywords?.length)
  );
  const latestSuggestion = $derived(
    suggestions.length > 0 ? suggestions[suggestions.length - 1] : null
  );
  const whisperTell = $derived(
    latestSuggestion?.suggestion ? extractTell(latestSuggestion.suggestion) : ''
  );
  const personality = $derived(derivePersonality(emotionHistory));
  let lastPersonalityLabel = '';
  $effect(() => {
    const p = personality;
    if (p && p.label !== lastPersonalityLabel) {
      lastPersonalityLabel = p.label;
      coachingLog = [...coachingLog.slice(-4), { text: p.tip, why: p.description, emotion: p.label, time: Date.now(), type: 'personality', color: p.color }];
    }
  });
  const nonNeutralTones = $derived(audioEmotionHistory.filter(t => t !== 'neutral'));

  function flipSpeaker(idx: number) {
    transcript = transcript.map((e, i) => i === idx ? { ...e, speaker: e.speaker === 'You' ? 'Interviewer' : 'You' } : e);
  }

  // Rate limits
  interface RateLimitEntry { remaining: number; limit: number; history: Array<{ r: number; t: number }>; }
  let rateLimits = $state<Record<string, RateLimitEntry>>({});
  let callCounts = $state<Record<string, number>>({});
  // Last successful provider per service (transcription / suggestions / sentiment)
  let providerStatus = $state<Record<string, { name: string; local: boolean }>>({});

  $effect(() => {
    const id = setInterval(async () => {
      callCounts = await fetchUsage();
    }, 5000);
    fetchUsage().then(d => callCounts = d);
    return () => clearInterval(id);
  });

  // WS status
  let wsStatus = $state('disconnected');
  let wsAttempt = $state(0);

  // Audio tone history (last 5 readings)
  let audioEmotionHistory = $state<string[]>([]);
  // Keyword → question it was raised in
  let keywordQuestionMap = $state<Record<string, string>>({});
  // Next steps (action items from transcript)
  let nextSteps = $state<string[]>([]);
  let loadingNextSteps = $state(false);
  let showNextSteps = $state(false);
  // Cue expand signal from keyboard shortcut
  let cueExpandSignal = $state<{ cueIdx: number; key: number } | null>(null);

  // Transcript auto-save
  const sessionKey = `transcript-${new Date().toISOString().slice(0, 16).replace('T', '_').replace(':', '-')}`;
  let showTranscripts = $state(false);

  let eventWs: EventWebSocket | null = null;

  async function fetchSalaryCoach() {
    if (salaryTactics || loadingSalary) return;
    loadingSalary = true;
    try {
      const sp = companyBrief ? `${companyBrief.name}: ${companyBrief.what_they_do}` : 'unknown role';
      const resp = await authFetch('/api/salary-coach', {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ role_context: sp }),
      });
      if (resp.ok) salaryTactics = await resp.json();
    } catch { /* ignore */ }
    loadingSalary = false;
  }

  async function predictNextQuestions() {
    if (loadingNextQ) return;
    loadingNextQ = true;
    try {
      const resp = await authFetch('/api/next-question', {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ transcript: transcript.map(e => ({ speaker: e.speaker, text: e.text })) }),
      });
      if (resp.ok) { const d = await resp.json(); nextQuestions = d.questions ?? []; }
    } catch { /* ignore */ }
    loadingNextQ = false;
  }

  // Auto-predict whenever a new question comes in
  $effect(() => {
    const count = suggestions.length;
    if (count > 0) predictNextQuestions();
  });

  async function fetchNextSteps() {
    if (loadingNextSteps || nextSteps.length > 0) return;
    loadingNextSteps = true;
    try {
      const resp = await authFetch('/api/next-steps', {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ transcript: transcript.map(e => ({ speaker: e.speaker, text: e.text })) }),
      });
      if (resp.ok) { const d = await resp.json(); nextSteps = d.steps ?? []; showNextSteps = nextSteps.length > 0; }
    } catch { /* ignore */ }
    loadingNextSteps = false;
  }

  const OPENING_SUGGESTION_STATIC =
`Acknowledge: Hi! Really great to meet you — thanks so much for having me today.
Answer: I'm doing well, thank you! Really looking forward to our conversation.
---
General: [their day] Ask how their day is going — keeps the tone warm and shows you're interested in them as a person
General: [the commute / setup] A quick comment about working from home or their office is a natural low-stakes opener
General: [something you noticed] If you spot something in their background or office, a brief friendly comment shows genuine curiosity
Ask: day | How's your day going so far?
Ask: week | Has it been a busy week for you?
Ask: team | How long have you been with the team?`;

  async function fetchOpeningSuggestion() {
    const names = interviewerSummaries.map(i => i.name).filter(Boolean).join(', ');
    const tips = interviewerSummaries.flatMap(i => i.rapport_tips ?? []).filter(Boolean).join('; ');
    const context = [names && `Interviewer(s): ${names}`, tips && `Rapport tips: ${tips}`].filter(Boolean).join('. ');
    const question = `Meeting opening greeting and small talk${context ? ` — ${context}` : ''} — give me warm, natural opening lines and 2-3 casual conversation topics to exchange greetings and build rapport at the very start of the meeting, before any interview questions begin. Keep it generic and friendly, not job-focused.`;

    currentQuestionIdx = 0;
    // Show static fallback immediately so there's always something visible
    suggestions = [{ question: '🤝 Opening', suggestion: OPENING_SUGGESTION_STATIC, streaming: false }];

    try {
      const resp = await authFetch('/api/practice-question', {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ question }),
      });
      const data = resp.ok ? await resp.json() : null;
      if (data?.suggestion) {
        // Add personalized version as a second entry so both are in question history
        suggestions = [...suggestions, { question: '🤝 Opening (personalized)', suggestion: data.suggestion, streaming: false }];
        currentQuestionIdx = 1;
      }
    } catch { /* keep static fallback */ }
  }

  function handleSetupComplete(data?: { companyBrief?: any; interviewerSummaries?: any[]; jdKeywords?: string[] }) {
    if (data?.companyBrief) companyBrief = data.companyBrief;
    if (data?.interviewerSummaries) interviewerSummaries = data.interviewerSummaries;
    if (data?.jdKeywords) { jdKeywords = data.jdKeywords; mentionedKeywords = new Set(); interviewerRaisedKeywords = new Set(); }
    phase = 'interview';
    connectWs();
    void fetchOpeningSuggestion();
  }
  function handlePractice(questions: string[]) { predictedQuestions = questions; phase = 'practice'; connectWs(); }

  function connectWs() {
    eventWs = new EventWebSocket();
    eventWs.onEvent(handleWsEvent);
    eventWs.onStatus((status, attempt) => { wsStatus = status; wsAttempt = attempt; });
    eventWs.connect();
  }

  function handleWsEvent(event: WsEvent) {
    switch (event.type) {
      case 'transcript': {
        const entry = { text: event.text, timestamp_ms: event.timestamp_ms, speaker: event.speaker };
        transcript = [...transcript, entry];
        if (event.speaker === 'You') {
          lastSpeechAt = Date.now();
          youSegments++;
          if (capturing) startAnswerTimer();
          youSegmentsSinceQuestion = [...youSegmentsSinceQuestion, event.text];
          recentYouTexts = [...recentYouTexts.slice(-19), { text: event.text, time: Date.now() }];
          // Track JD keywords mentioned
          const newlyMentioned = checkMentioned(event.text, jdKeywords);
          if (newlyMentioned.length > 0) {
            const updated = new Set(mentionedKeywords);
            newlyMentioned.forEach(k => updated.add(k));
            mentionedKeywords = updated;
            // Flash newly-detected keywords for 2 seconds
            flashingKeywords = new Set(newlyMentioned);
            if (flashTimer) clearTimeout(flashTimer);
            flashTimer = setTimeout(() => { flashingKeywords = new Set(); }, 2000);
          }
          const newCounts = countFillers(event.text);
          const merged: Record<string, number> = {};
          for (const f of allFillerCounts) merged[f.word] = f.count;
          for (const f of newCounts) merged[f.word] = (merged[f.word] ?? 0) + f.count;
          allFillerCounts = Object.entries(merged).map(([word, count]) => ({ word, count }));
        } else if (event.speaker === 'Interviewer') {
          interviewerSegments++;
          // Vocal delivery feedback — capture before timer reset
          if (answerMs > 3000 && currentQuestionIdx >= 0 && youSegmentsSinceQuestion.length > 0) {
            const capturedMs = answerMs;
            const capturedAnswer = youSegmentsSinceQuestion.join(' ');
            const capturedFillerCount = fillerTotal;
            const capturedFillerDetail = allFillerCounts.map(f => `${f.word} ×${f.count}`).join(', ');
            const capturedQIdx = currentQuestionIdx;
            const capturedQuestion = suggestions[capturedQIdx]?.question ?? '';
            const wordCount = capturedAnswer.split(/\s+/).filter(Boolean).length;
            // Confidence score — client-side, immediate
            const capturedSuggestion = suggestions[capturedQIdx]?.suggestion ?? '';
            if (capturedSuggestion) {
              const conf = computeConfidence(capturedAnswer, capturedSuggestion);
              suggestions = suggestions.map((s, i) => i === capturedQIdx ? { ...s, confidenceScore: conf.score, matchedKeywords: conf.matched, missedKeywords: conf.missed } : s);
            }
            authFetch('/api/vocal-sentiment', {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({
                question: capturedQuestion,
                transcript: capturedAnswer,
                duration_seconds: capturedMs / 1000,
                word_count: wordCount,
                filler_count: capturedFillerCount,
                filler_detail: capturedFillerDetail,
              }),
            })
              .then(r => r.ok ? r.json() : null)
              .then(vf => {
                if (vf) { suggestions = suggestions.map((s, i) => i === capturedQIdx ? { ...s, vocalFeedback: vf } : s); vocalWhyExpanded = false; }
              })
              .catch(() => {});
          }
          resetAnswerTimer();
          // Surface keywords the interviewer raises (not marked done, just highlighted)
          const interviewerRaised = checkMentioned(event.text, jdKeywords);
          if (interviewerRaised.length > 0) {
            const updated = new Set(interviewerRaisedKeywords);
            const currentQ = suggestions[currentQuestionIdx]?.question ?? '';
            const updatedMap = { ...keywordQuestionMap };
            interviewerRaised.forEach(k => { updated.add(k); if (currentQ && !updatedMap[k]) updatedMap[k] = currentQ; });
            interviewerRaisedKeywords = updated;
            keywordQuestionMap = updatedMap;
          }
          const tone = analyzeAudioTone(event.text, sysEnergyEma, latestAudioFeatures);
          audioEmotion = tone.emotion;
          audioReason = tone.reason;
          audioEmotionHistory = [...audioEmotionHistory.slice(-4), tone.emotion];
          if (isSalaryQuestion(event.text)) fetchSalaryCoach();
          const now = Date.now();
          if (captureInst && now - lastSentimentTrigger > 10000) {
            lastSentimentTrigger = now;
            captureInst.triggerFrameCapture();
          }
        }
        break;
      }
      case 'sentiment':
        lastBackendSentimentTime = Date.now();
        clientFaceEmotion = ''; // backend reading takes precedence now
        emotion = event.emotion;
        if (event.reason) emotionReason = event.reason;
        if (event.coaching) {
          coaching = event.coaching;
          const last = coachingLog[coachingLog.length - 1];
          if (!last || last.text !== event.coaching) {
            const entry = { text: event.coaching, why: event.coaching_why, reason: event.reason, emotion: event.emotion, time: Date.now() };
            if (expandedCoachingEntries.size > 0) {
              pendingCoachingEntry = entry; // hold until user closes the expanded entry
            } else {
              coachingLog = [...coachingLog.slice(-4), entry];
            }
          }
        }
        if (event.coaching_why) coachingWhy = event.coaching_why;
        emotionHistory = [...emotionHistory, event.emotion];
        // Detect negative → positive transition before updating lastTrackedEmotion
        if (NEGATIVE_EMOTIONS.has(lastTrackedEmotion) && POSITIVE_EMOTIONS.has(event.emotion)) {
          if (transitionTimer) clearTimeout(transitionTimer);
          transitionNote = { from: lastTrackedEmotion, to: event.emotion };
          transitionTimer = setTimeout(() => { transitionNote = null; }, 10000);
        } else if (transitionNote && !POSITIVE_EMOTIONS.has(event.emotion)) {
          if (transitionTimer) clearTimeout(transitionTimer);
          transitionNote = null;
        }
        // Track consecutive reads of same emotion
        if (event.emotion === lastTrackedEmotion) {
          consecutiveEmotionCount++;
        } else {
          consecutiveEmotionCount = 1;
          lastTrackedEmotion = event.emotion;
        }
        // Track which body-language items get triggered (item 10)
        {
          const emotionTriggers: Record<string, string[]> = {
            eye: ['skeptical', 'bored', 'confused'],
            posture: ['bored', 'neutral', 'skeptical'],
            nod: ['neutral', 'bored'],
            hands: ['skeptical', 'confused'],
            expression: ['bored', 'neutral', 'skeptical'],
            pace: ['curious', 'engaged', 'skeptical'],
          };
          const updated = { ...blTriggerCounts };
          for (const [id, triggers] of Object.entries(emotionTriggers)) {
            if (triggers.includes(event.emotion)) {
              updated[id] = (updated[id] ?? 0) + 1;
            }
          }
          blTriggerCounts = updated;
          localStorage.setItem('bl-trigger-counts', JSON.stringify(updated));
        }
        break;
      case 'question_detected': {
        // Mark previous question as answered/unanswered
        if (currentQuestionIdx >= 0) {
          const wasAnswered = youSegmentsSinceQuestion.length > 0;
          suggestions = suggestions.map((s, i) =>
            i === currentQuestionIdx ? { ...s, answered: wasAnswered } : s
          );
          // Run answer feedback for previous question if it was answered
          if (wasAnswered) {
            const prevSuggestion = suggestions[currentQuestionIdx];
            if (prevSuggestion && prevSuggestion.suggestion) {
              const prevIdx = currentQuestionIdx;
              const answerText = youSegmentsSinceQuestion.join(' ');
              authFetch('/api/answer-feedback', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                  question: prevSuggestion.question,
                  answer: answerText,
                  suggestion: prevSuggestion.suggestion,
                }),
              })
                .then(r => r.ok ? r.json() : null)
                .then(fb => {
                  if (fb) {
                    suggestions = suggestions.map((s, i) =>
                      i === prevIdx ? { ...s, answerFeedback: fb } : s
                    );
                    answerWhyExpanded = false;
                  }
                })
                .catch(() => {});
            }
          }
        }

        const subQuestions = splitMultiQuestions(event.question);
        const secondaryTag = event.secondary_tag as import('./lib/types').QuestionTag | undefined;
        youSegmentsSinceQuestion = [];

        for (let qi = 0; qi < subQuestions.length; qi++) {
          const q = subQuestions[qi];
          const isFirst = qi === 0;
          const newIdx = suggestions.length;
          if (isFirst) { currentQuestionIdx = newIdx; jumpSignal = { idx: newIdx, key: Date.now() }; }
          const isCompound = isFirst && !!secondaryTag;
          suggestions = [...suggestions, {
            question: q,
            suggestion: isFirst ? '' : '(Additional question — will generate suggestion when you navigate here)',
            streaming: isFirst && !isCompound,    // primary starts streaming only for single-type
            tag: tagQuestion(q),
            secondaryTag: isFirst ? secondaryTag : undefined,
            compoundSuggestion: isCompound ? '' : undefined,
            compoundStreaming: isCompound,          // compound streams first for compound questions
            secondarySuggestion: isCompound ? '' : undefined,
            secondaryStreaming: false,
            redFlag: detectRedFlag(q) ?? undefined,
          }];
        }
        resetAnswerTimer();
        // Answer moment nudge (item 5)
        if (answerNudgeTimer) clearTimeout(answerNudgeTimer);
        answerNudgeVisible = true;
        answerNudgeTimer = setTimeout(() => { answerNudgeVisible = false; }, 4000);
        break;
      }
      case 'suggestion_token':
        if (event.mode === 'compound') {
          suggestions = suggestions.map(s =>
            s.compoundStreaming ? { ...s, compoundSuggestion: (s.compoundSuggestion ?? '') + event.token } : s
          );
        } else if (event.mode === 'secondary') {
          suggestions = suggestions.map(s =>
            s.secondaryStreaming ? { ...s, secondarySuggestion: (s.secondarySuggestion ?? '') + event.token } : s
          );
        } else {
          suggestions = suggestions.map(s =>
            s.streaming ? { ...s, suggestion: s.suggestion + event.token } : s
          );
        }
        break;
      case 'suggestion_complete': {
        if (event.mode === 'compound') {
          suggestions = suggestions.map(s =>
            s.compoundStreaming
              ? { ...s, compoundSuggestion: event.full_text, compoundStreaming: false, streaming: true }
              : s
          );
          speakText(event.full_text);
        } else if (event.mode === 'secondary') {
          suggestions = suggestions.map(s =>
            s.secondaryStreaming ? { ...s, secondarySuggestion: event.full_text, secondaryStreaming: false } : s
          );
        } else {
          suggestions = suggestions.map(s => {
            if (!s.streaming) return s;
            // If compound question: after primary completes, prepare secondary slot
            const next = s.secondaryTag ? { ...s, suggestion: event.full_text, streaming: false, secondaryStreaming: true } : { ...s, suggestion: event.full_text, streaming: false };
            return next;
          });
          speakText(event.full_text);
        }
        break;
      }
      case 'status':
        statusMessages = [...statusMessages.slice(-4), event.message];
        break;
      case 'error':
        if (!errorMessages.includes(event.message)) errorMessages = [...errorMessages, event.message];
        break;
      case 'rate_limit': {
        const prev = rateLimits[event.provider];
        const point = { r: event.requests_remaining, t: Date.now() };
        const history = prev ? [...prev.history.slice(-14), point] : [point];
        rateLimits = { ...rateLimits, [event.provider]: { remaining: event.requests_remaining, limit: event.requests_limit, history } };
        break;
      }
      case 'provider_used': {
        providerStatus = { ...providerStatus, [event.service]: { name: event.provider, local: event.local } };
        if (event.service === 'suggestions') {
          suggestions = suggestions.map(s => s.streaming ? { ...s, provider: event.provider, providerLocal: event.local } : s);
        }
        break;
      }
    }
  }

  function renderBold(text: string): string {
    return text.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
  }

  // Video zoom / pan
  interface VidZoom { zoom: number; panX: number; panY: number; }
  interface PanTrack { on: boolean; x: number; y: number; px: number; py: number; w: number; h: number; }
  let iVid = $state<VidZoom>({ zoom: 1, panX: 0, panY: 0 }); // interviewer strip
  let sVid = $state<VidZoom>({ zoom: 1, panX: 0, panY: 0 }); // selfview
  const iPan: PanTrack = { on: false, x: 0, y: 0, px: 0, py: 0, w: 0, h: 0 };
  const sPan: PanTrack = { on: false, x: 0, y: 0, px: 0, py: 0, w: 0, h: 0 };

  function vidWheel(v: VidZoom, e: WheelEvent) {
    e.preventDefault();
    v.zoom = Math.max(1, Math.min(10, v.zoom * (e.deltaY < 0 ? 1.2 : 1 / 1.2)));
    if (v.zoom < 1.02) { v.zoom = 1; v.panX = 0; v.panY = 0; }
  }
  function vidDown(v: VidZoom, t: PanTrack, e: PointerEvent) {
    if (v.zoom <= 1) return;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    t.on = true; t.x = e.clientX; t.y = e.clientY; t.px = v.panX; t.py = v.panY; t.w = r.width; t.h = r.height;
  }
  function vidMove(v: VidZoom, t: PanTrack, e: PointerEvent) {
    if (!t.on) return;
    const mx = t.w * (v.zoom - 1) / 2;
    const my = t.h * (v.zoom - 1) / 2;
    v.panX = Math.max(-mx, Math.min(mx, t.px + (e.clientX - t.x)));
    v.panY = Math.max(-my, Math.min(my, t.py + (e.clientY - t.y)));
  }
  function vidUp(t: PanTrack) { t.on = false; }
  function vidReset(v: VidZoom) { v.zoom = 1; v.panX = 0; v.panY = 0; }

  let pickerContainerRect = { left: 0, top: 0, width: 1, height: 1 }; // cached at mousedown

  function pickerDown(e: MouseEvent) {
    pickerContainerRect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const x = (e.clientX - pickerContainerRect.left) / pickerContainerRect.width;
    const y = (e.clientY - pickerContainerRect.top) / pickerContainerRect.height;
    pickerDrag = { sx: x, sy: y, cx: x, cy: y };
    pickerPendingRect = null;
  }
  function pickerMove(e: MouseEvent) {
    if (!pickerDrag) return;
    pickerDrag = { ...pickerDrag,
      cx: Math.max(0, Math.min(1, (e.clientX - pickerContainerRect.left) / pickerContainerRect.width)),
      cy: Math.max(0, Math.min(1, (e.clientY - pickerContainerRect.top) / pickerContainerRect.height)),
    };
  }
  function pickerUp() {
    if (!pickerDrag) return;
    const x = Math.min(pickerDrag.sx, pickerDrag.cx);
    const y = Math.min(pickerDrag.sy, pickerDrag.cy);
    const w = Math.abs(pickerDrag.cx - pickerDrag.sx);
    const h = Math.abs(pickerDrag.cy - pickerDrag.sy);
    if (w > 0.03 && h > 0.03) pickerPendingRect = { x, y, w, h };
    pickerDrag = null;
  }
  function applyFaceCrop() {
    if (pickerPendingRect) {
      cropRect = pickerPendingRect;
      localStorage.setItem(SK.cropRect, JSON.stringify(cropRect));
    }
    showCropPicker = false;
    pickerDrag = null;
    pickerPendingRect = null;
  }
  function clearFaceCrop() {
    cropRect = null;
    localStorage.removeItem(SK.cropRect);
    showCropPicker = false;
  }

  $effect(() => {
    function onKey(e: KeyboardEvent) {
      if (phase !== 'interview') return;
      const tag = (e.target as HTMLElement).tagName;
      if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return;
      switch (e.key) {
        case 'f': case 'F': focusMode = !focusMode; break;
        case 'Escape':
          if (focusMode) { focusMode = false; break; }
          suggestions = suggestions.map(s => s.streaming ? s : { ...s, suggestion: '' });
          break;
        case '+': case '=': adjustZoom('all', +10); break;
        case '-': case '_': adjustZoom('all', -10); break;
        case 't': case 'T': if (!showVoiceMenu) ttsEnabled = !ttsEnabled; break;
        case 'w': case 'W': showWhisper = !showWhisper; break;
        case '1': cueExpandSignal = { cueIdx: 0, key: Date.now() }; break;
        case '2': cueExpandSignal = { cueIdx: 1, key: Date.now() }; break;
        case '3': cueExpandSignal = { cueIdx: 2, key: Date.now() }; break;
      }
    }
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });

  $effect(() => {
    if (audioEmotion === 'wrapping up' && nextSteps.length === 0 && !loadingNextSteps && transcript.length > 4) {
      fetchNextSteps();
    }
  });
  $effect(() => {
    if (transcript.length > 0) {
      const name = companyBrief?.name ?? '';
      const role = (interviewerSummaries[0] as any)?.role ?? '';
      const label = [name, role].filter(Boolean).join(' — ');
      const key = label ? `transcript_${label}_${sessionKey}` : `transcript_${sessionKey}`;
      try {
        localStorage.setItem(key, JSON.stringify(transcript));
      } catch { /* storage full */ }
    }
  });
</script>

<main style="font-size: var(--font-size, 14px)">
  {#if phase === 'setup'}
    <div class="setup-container">
      <header class="setup-header">
        <h1>AI Interview Assistant</h1>
        <p>Real-time AI coaching during your job interview</p>
        <div class="setup-font-row">
          <label class="setup-font-label">Font</label>
          <select class="font-select" bind:value={appFont} title="App font">
            {#each FONTS as f}
              <option value={f.id}>{f.label}</option>
            {/each}
          </select>
        </div>
      </header>
      <SetupForm onSetupComplete={handleSetupComplete} onPractice={handlePractice} />
      <div class="setup-review-row">
        <button class="setup-review-btn" onclick={() => showReviewUpload = true}>
          ⬆ Review a Recording
        </button>
        <button class="setup-review-btn" onclick={() => { showReviewList = !showReviewList; if (!showReviewList) return; loadReviewList(); }}>
          {showReviewList ? '▴' : '▾'} Past Reports
        </button>
        {#if reviewReport}
          <button class="setup-review-view-btn" onclick={() => { showReviewPanel = true; }}>
            View last report: {reviewReport.source_filename}
          </button>
        {/if}
      </div>
      {#if showReviewList}
        <div class="review-list-panel">
          <input class="review-search" type="text" placeholder="Search reports…" bind:value={reviewSearch} />
          {#if filteredReviews.length === 0}
            <p class="review-list-empty">{reviewSearch ? 'No matching reports.' : 'No reports yet. Upload a recording to get started.'}</p>
          {:else}
            <div class="review-list">
              {#each filteredReviews as r}
                <div class="review-list-item">
                  <div class="review-list-meta">
                    <span class="review-list-name">{r.source_filename ?? 'Untitled'}</span>
                    <span class="review-list-date">{r.created_at ? new Date(r.created_at).toLocaleDateString() : ''}</span>
                  </div>
                  <p class="review-list-summary">{r.qa_pairs.length} Q&A · {r.vocal_summary.avg_wpm} wpm · {Math.round(r.speaker_summary.you_pct)}% you</p>
                  <div class="review-list-actions">
                    <button class="review-list-open" onclick={() => { reviewReport = r; showReviewPanel = true; }}>Open</button>
                    <button class="review-list-delete" onclick={() => deleteReview(r.id)}>Delete</button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
      {#if Object.keys(callCounts).length > 0 || Object.keys(rateLimits).length > 0}
        <div class="setup-usage">
          <RateLimitPanel {rateLimits} {callCounts} />
        </div>
      {/if}
    </div>

  {:else if phase === 'practice'}
    <PracticePanel
      questions={predictedQuestions}
      systemPrompt=""
      onStartInterview={() => { phase = 'interview'; }}
      onBackToSetup={() => { phase = 'setup'; }}
    />

  {:else}
    <div class="interview-layout">
      <header class="interview-header">
        <div class="header-title-row">
          <button class="header-back-btn" onclick={() => { phase = 'setup'; }} title="Back to overview">← Overview</button>
          <h1>AI Interview Assistant</h1>
          <span class="ws-header-dot"
            class:ws-connected={wsStatus === 'connected'}
            class:ws-reconnecting={wsStatus === 'reconnecting'}
            title={wsStatus === 'connected' ? 'Server connected — AI events (transcripts, suggestions, sentiment) are live' : wsStatus === 'reconnecting' ? `Reconnecting to server (attempt ${wsAttempt}) — AI responses paused` : 'Disconnected from server — AI responses unavailable'}
          >{wsStatus === 'connected' ? '●' : wsStatus === 'reconnecting' ? `↻ ${wsAttempt}` : '○'}</span>
        </div>
        <div class="header-right">
          <div class="shortcuts-hint">F: focus · W: whisper · T: voice · 1/2/3: expand cue · Esc: clear · +/−: font</div>

          <!-- TTS controls -->
          <div class="tts-controls">
            <button
              class="tts-btn"
              class:tts-on={ttsEnabled}
              onclick={() => ttsEnabled = !ttsEnabled}
              title="Toggle voice hints (T)"
            >{ttsEnabled ? '🔊' : '🔇'} Voice</button>

            {#if ttsEnabled}
              <label class="rate-label" title="Speech speed">
                <span class="rate-val">{ttsRate.toFixed(1)}×</span>
                <input type="range" min="0.7" max="4.0" step="0.1" bind:value={ttsRate} class="rate-slider" />
              </label>
              <label class="rate-label" title="Voice volume">
                <span class="rate-val">{Math.round(ttsVolume * 100)}%</span>
                <input type="range" min="0.1" max="4" step="0.05" bind:value={ttsVolume} class="rate-slider vol-slider"
                  oninput={(e) => { const v = Number((e.target as HTMLInputElement).value); if (Math.abs(v - 1) < 0.08) ttsVolume = 1; }} />
              </label>
              <button class="voice-pick-btn" onclick={() => showVoiceMenu = !showVoiceMenu} title="Choose voice">▾</button>
              <button
                class="voice-test-inline-btn"
                title="Test current voice"
                onclick={() => ttsClient.speak("Hi, I'm excited to be here today.", ttsVoiceId, ttsRate, ttsVolume)}
              >▶ Test</button>
            {/if}
            <select class="font-select" bind:value={appFont} title="App font">
              {#each FONTS as f}
                <option value={f.id}>{f.label}</option>
              {/each}
            </select>

            {#if showVoiceMenu}
              <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
              <div class="voice-menu" role="menu" onmouseleave={() => showVoiceMenu = false}>
                {#each ['piper', 'os', 'browser'] as src}
                  {@const group = ttsVoices.filter(v => v.source === src)}
                  {#if group.length > 0}
                    <div class="voice-group-label">{src === 'piper' ? 'Piper (Neural)' : src === 'os' ? 'Windows (SAPI)' : 'Browser'}</div>
                    {#each group as v}
                      <div class="voice-row" class:selected={v.id === ttsVoiceId}>
                        <button
                          class="voice-option"
                          class:selected={v.id === ttsVoiceId}
                          onclick={() => { ttsVoiceId = v.id; showVoiceMenu = false; }}
                        >{v.name}</button>
                        <button
                          class="voice-test-btn"
                          title="Preview this voice"
                          onclick={(e) => { e.stopPropagation(); ttsClient.speak("Hi, I'm excited to be here today.", v.id, ttsRate, ttsVolume); }}
                        >▶</button>
                      </div>
                    {/each}
                  {/if}
                {/each}
              </div>
            {/if}
          </div>

          <button class="history-btn" onclick={() => showHistory = true}>History</button>
          <button class="history-btn" title="Reset all panel positions and zoom" onclick={() => {
            Object.values(SK).forEach(k => localStorage.removeItem(k));
            location.reload();
          }}>Reset Layout</button>
          <div class="transcripts-wrapper">
            <button class="header-btn" onclick={() => showTranscripts = !showTranscripts} title="View saved transcripts">📄 Transcripts</button>
            {#if showTranscripts}
              <div class="transcripts-dropdown">
                {#each Object.keys(localStorage).filter(k => k.startsWith('transcript_')) as key}
                  <button class="transcript-item" onclick={() => {
                    const data = localStorage.getItem(key);
                    if (data) {
                      const entries = JSON.parse(data);
                      const lines = entries.map((e: any) => `[${new Date(e.timestamp_ms).toLocaleTimeString()}] ${e.speaker}: ${e.text}`);
                      const blob = new Blob([lines.join('\n')], { type: 'text/plain' });
                      const url = URL.createObjectURL(blob);
                      const a = document.createElement('a');
                      a.href = url;
                      a.download = key + '.txt';
                      a.click();
                      URL.revokeObjectURL(url);
                    }
                    showTranscripts = false;
                  }}>{key.replace('transcript_', '')}</button>
                {:else}
                  <div class="transcript-empty">No saved transcripts</div>
                {/each}
              </div>
            {/if}
          </div>
          <button class="debrief-btn save-report-btn"
            class:saving={savingLiveReport}
            disabled={savingLiveReport}
            onclick={async () => {
              savingLiveReport = true;
              try {
                const resp = await authFetch('/api/review/from-live', { method: 'POST' });
                if (resp.ok) {
                  reviewReport = await resp.json();
                  showReviewPanel = true;
                }
              } finally { savingLiveReport = false; }
            }}
          >{savingLiveReport ? 'Saving…' : 'Save Report'}</button>
          <button class="debrief-btn" onclick={() => showDebrief = true}>End Interview</button>
          <CaptureButton
            onCapture={(v) => { capturing = v; if (v) ttsEnabled = true; if (!v) { webcamStream = null; screenStream = null; captureInst = null; resetAnswerTimer(); } }}
            onStreams={(screen, webcam) => { screenStream = screen; webcamStream = webcam; }}
            onReady={(cap) => { captureInst = cap; }}
            onLevel={(_mic, sys) => updateSysEnergy(sys)}
          />
        </div>
      </header>

      {#if errorMessages.length > 0}
        <div class="error-banner">
          <div class="error-list">{#each errorMessages as msg}<div>{msg}</div>{/each}</div>
          <div class="error-actions">
            <button class="error-btn" onclick={() => navigator.clipboard.writeText(errorMessages.join('\n'))}>Copy</button>
            <button class="error-btn" onclick={() => (errorMessages = [])}>✕</button>
          </div>
        </div>
      {/if}

      {#if statusMessages.length > 0}
        <div class="status-banner">{statusMessages[statusMessages.length - 1]}</div>
      {/if}

      <!-- Resizable 4-column layout (drag col-header to reorder) -->
      <div class="three-col">
        {#each colOrder as col, ci}
          {#if ci > 0}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div class="resize-handle" onmousedown={(e) => {
              const prevCol = colOrder[ci - 1];
              if (prevCol === 'left' || prevCol === 'hist' || prevCol === 'center' || prevCol === 'right') startResize(prevCol as 'left' | 'hist' | 'center' | 'right', e);
            }} title="Drag to resize"></div>
          {/if}

          {#if col === 'left'}
            <!-- Left: Transcript + Interviewers (split) -->
            <div class="col col-left" style="width: {collapsedCols.has('left') ? '28px' : `${leftW}px`}"
              ondragover={onColDragOver} ondrop={(e) => onColDrop('left', e)}>
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div class="col-header col-drag-handle" draggable={true} ondragstart={(e) => onColDragStart('left', e)}>
                <span class="col-label">{collapsedCols.has('left') ? '…' : 'Transcript'}</span>
                {#if !collapsedCols.has('left')}
                  {#if providerStatus.transcription}
                    <span class="provider-chip" class:provider-chip-local={providerStatus.transcription.local} title={providerStatus.transcription.local ? 'Local' : 'API'}>{providerStatus.transcription.name}</span>
                  {/if}
                  <div class="zoom-btns">
                    <button class="zoom-btn" onclick={() => adjustZoom('left', -10)} title="Decrease font size">A−</button>
                    <button class="zoom-btn" onclick={() => adjustZoom('left', +10)} title="Increase font size">A+</button>
                    <button class="zoom-btn collapse-btn" onclick={() => toggleColCollapse('left')} title="Collapse">▾</button>
                  </div>
                {:else}
                  <button class="zoom-btn collapse-btn" onclick={() => toggleColCollapse('left')} title="Expand">▸</button>
                {/if}
              </div>
              {#if !collapsedCols.has('left')}
                <div class="col-body">
                  <div class="col-body-scroll" style="zoom: {leftZoom/100}">
                    <TranscriptPanel entries={transcript} onFlipSpeaker={flipSpeaker} />
                  </div>
                </div>
              {/if}
            </div>

          {:else if col === 'hist'}
            <!-- Questions history column -->
            <div class="col col-hist" style="width: {collapsedCols.has('hist') ? '28px' : `${histW}px`}"
              ondragover={onColDragOver} ondrop={(e) => onColDrop('hist', e)}>
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div class="col-header col-drag-handle" draggable={true} ondragstart={(e) => onColDragStart('hist', e)}>
                <span class="col-label">{collapsedCols.has('hist') ? '…' : 'Questions'}{#if unseenCount > 0}<span class="new-q-badge">{unseenCount}</span>{/if}</span>
                {#if !collapsedCols.has('hist')}
                  <div class="zoom-btns">
                    <button class="zoom-btn" onclick={() => adjustZoom('hist', -10)} title="Decrease font size">A−</button>
                    <button class="zoom-btn" onclick={() => adjustZoom('hist', +10)} title="Increase font size">A+</button>
                    <button class="zoom-btn collapse-btn" onclick={() => toggleColCollapse('hist')} title="Collapse">▾</button>
                  </div>
                {:else}
                  <button class="zoom-btn collapse-btn" onclick={() => toggleColCollapse('hist')} title="Expand">▸</button>
                {/if}
              </div>
              {#if !collapsedCols.has('hist')}
                <div class="col-body" bind:this={histColBodyEl}>
                  <div class="col-body-scroll" style="zoom: {histZoom/100}; padding: 0.3rem 0.4rem 0.4rem; display: flex; flex-direction: column; gap: 0.4rem;">
                    <TestQuestionBar {capturing} />
                    <QuestionsHistoryPanel
                      {suggestions}
                      currentIndex={histViewIdx === -1 ? currentQuestionIdx : histViewIdx}
                      onJump={(i) => { histViewIdx = i; jumpSignal = { idx: i, key: Date.now() }; }}
                      {scrollToLatestKey}
                    />
                  </div>
                </div>
              {/if}
            </div>

          {:else if col === 'center'}
            <!-- Center: AI Suggestions -->
            <div class="col col-center" style="width: {collapsedCols.has('center') ? '28px' : `${centerW}px`}"
              ondragover={onColDragOver} ondrop={(e) => onColDrop('center', e)}>
              {#if screenStream && !collapsedCols.has('center')}
                <div class="interviewer-strip">
                  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
                  <div class="vid-zoom-shell"
                    bind:this={vidZoomShellEl}
                    onwheel={(e) => vidWheel(iVid, e)}
                    onpointerdown={(e) => vidDown(iVid, iPan, e)}
                    onpointermove={(e) => vidMove(iVid, iPan, e)}
                    onpointerup={() => vidUp(iPan)}
                    ondblclick={() => vidReset(iVid)}
                    style="cursor:{iVid.zoom > 1 ? 'grab' : 'default'}{interviewerVidH ? `;height:${interviewerVidH}px` : ';max-height:40vh'}"
                    title="Scroll to zoom · drag to pan · double-click to reset"
                  >
                    <div style="width:100%;height:100%;display:flex;align-items:center;justify-content:center;transform:translate({iVid.panX}px,{iVid.panY}px) scale({iVid.zoom});transform-origin:center;pointer-events:none;">
                      {#if cropRect}
                        <div class="face-crop-wrap" style="aspect-ratio:{cropRect.w * videoNaturalAR / cropRect.h};{interviewerVidH ? 'height:100%;width:auto;max-width:100%' : 'width:100%'};overflow:hidden;position:relative;">
                          <!-- svelte-ignore a11y_media_has_caption -->
                          <video bind:this={screenEl} autoplay muted playsinline
                            style="position:absolute;width:{100/cropRect.w}%;height:auto;transform:translate({-cropRect.x*100}%,{-cropRect.y*100}%);"
                          ></video>
                        </div>
                      {:else}
                        <!-- svelte-ignore a11y_media_has_caption -->
                        <video bind:this={screenEl} autoplay muted playsinline
                          style="{interviewerVidH ? 'width:100%;height:100%;object-fit:contain;' : 'width:100%;aspect-ratio:16/9;object-fit:cover;'} display:block;background:#0a1525;"
                        ></video>
                      {/if}
                    </div>
                  </div>
                  <div class="face-pick-row">
                    <button class="face-pick-btn" onclick={() => { showCropPicker = true; pickerPendingRect = cropRect ? { ...cropRect } : null; }}>
                      {cropRect ? '✎ Adjust' : '⊞ Pick face'}
                    </button>
                    {#if cropRect}
                      <button class="face-pick-btn" onclick={clearFaceCrop}>Full screen</button>
                    {/if}
                  </div>
                  <!-- svelte-ignore a11y_no_static_element_interactions -->
                  <div class="vid-resize-handle"
                    onpointerdown={iVidResizeDown}
                    onpointermove={iVidResizeMove}
                    onpointerup={iVidResizeUp}
                    onpointercancel={iVidResizeUp}
                  ></div>
                </div>
              {/if}
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div class="col-header col-drag-handle" draggable={true} ondragstart={(e) => onColDragStart('center', e)} style="padding-top: 0.35rem;">
                <span class="col-label">{collapsedCols.has('center') ? '…' : 'AI Suggestions'}</span>
                {#if !collapsedCols.has('center')}
                  {#if providerStatus.suggestions}
                    <span class="provider-chip" class:provider-chip-local={providerStatus.suggestions.local} title={providerStatus.suggestions.local ? 'Local' : 'API'}>{providerStatus.suggestions.name}</span>
                  {/if}
                  <div class="zoom-btns">
                    <button class="zoom-btn" onclick={() => adjustZoom('center', -10)} title="Decrease font size">A−</button>
                    <button class="zoom-btn" onclick={() => adjustZoom('center', +10)} title="Increase font size">A+</button>
                    <button class="zoom-btn collapse-btn" onclick={() => toggleColCollapse('center')} title="Collapse">▾</button>
                  </div>
                {:else}
                  <button class="zoom-btn collapse-btn" onclick={() => toggleColCollapse('center')} title="Expand">▸</button>
                {/if}
              </div>
              {#if !collapsedCols.has('center')}
                <div class="col-body col-split-body" bind:this={centerColBodyEl}>
                  <div class="col-body-scroll" style="zoom: {centerZoom/100}; padding: 0.25rem 0.5rem 0.5rem;">
                    <SuggestionPanel {suggestions} onClear={() => (suggestions = [])} teleprompter={true} {jumpSignal} {cueExpandSignal} onPinnedChange={(p) => (suggestionPinned = p)} />
                  </div>
                </div>
              {/if}
            </div>

          {:else if col === 'right'}
            <!-- Right: Interviewer Sentiment -->
            <div class="col col-right"
              style={collapsedCols.has('right') ? 'flex: 0 0 28px; min-width: 0; width: 28px;' : `flex-shrink: 0; width: ${rightW}px`}
              ondragover={onColDragOver} ondrop={(e) => onColDrop('right', e)}>
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              {#if collapsedCols.has('right')}
                <div class="col-header col-drag-handle" draggable={true} ondragstart={(e) => onColDragStart('right', e)}>
                  <span class="col-label">…</span>
                  <button class="zoom-btn collapse-btn" onclick={() => toggleColCollapse('right')} title="Expand">▸</button>
                </div>
              {/if}
              {#if !collapsedCols.has('right')}
                {#if webcamStream}
                  <div class="selfview-strip">
                    <!-- svelte-ignore a11y_no_noninteractive_element_interactions a11y_media_has_caption -->
                    <div class="selfview-zoom-shell"
                      class:selfview-zoomed={sVid.zoom > 1}
                      onwheel={(e) => vidWheel(sVid, e)}
                      onpointerdown={(e) => vidDown(sVid, sPan, e)}
                      onpointermove={(e) => vidMove(sVid, sPan, e)}
                      onpointerup={() => vidUp(sPan)}
                      ondblclick={() => vidReset(sVid)}
                      style="height:{selfviewH}px;cursor:{sVid.zoom > 1 ? 'grab' : 'default'}"
                      title="Scroll to zoom · drag to pan · double-click to reset"
                    >
                      <video bind:this={webcamEl} class="selfview" autoplay muted playsinline
                        style="transform: translate({sVid.panX}px, {sVid.panY}px) scale({sVid.zoom}) scaleX(-1); transform-origin: center;"
                      ></video>
                    </div>
                    <div class="selfview-label">You</div>
                    <!-- svelte-ignore a11y_no_static_element_interactions -->
                    <div class="vid-resize-handle selfview-resize-handle"
                      onpointerdown={selfviewResizeDown}
                      onpointermove={selfviewResizeMove}
                      onpointerup={selfviewResizeUp}
                      onpointercancel={selfviewResizeUp}
                    ></div>
                  </div>
                {/if}
                <div class="col-header col-drag-handle" draggable={true} ondragstart={(e) => onColDragStart('right', e)}>
                  <span class="col-label">Interviewer</span>
                  {#if providerStatus.sentiment}
                    <span class="provider-chip" class:provider-chip-local={providerStatus.sentiment.local} title={providerStatus.sentiment.local ? 'Local' : 'API'}>{providerStatus.sentiment.name}</span>
                  {/if}
                  <div class="zoom-btns">
                    <button class="zoom-btn" onclick={() => adjustZoom('rightTop', -10)} title="Decrease font size">A−</button>
                    <button class="zoom-btn" onclick={() => adjustZoom('rightTop', +10)} title="Increase font size">A+</button>
                    <button class="zoom-btn collapse-btn" onclick={() => toggleColCollapse('right')} title="Collapse">▾</button>
                  </div>
                </div>
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div class="right-panel-scroll" style="zoom: {rightTopZoom/100}" ondragover={(e) => { e.preventDefault(); }} ondrop={(e) => onPanelEmptyDrop('sentiment', e)}>
                  {#if answerNudgeVisible}
                    <div class="answer-nudge">
                      <span class="answer-nudge-icon">🧘</span>
                      <div class="answer-nudge-text">
                        <strong>Breathe.</strong> Pause 1–2 seconds before answering.
                      </div>
                    </div>
                  {/if}
                  {#if transitionNote}
                    <div class="transition-note">
                      Turned it around — <span class="transition-from">{transitionNote.from}</span> → <span style="color: {emotionColor(transitionNote.to)}">{transitionNote.to}</span>
                    </div>
                  {:else if positiveStreak}
                    <div class="positive-streak">
                      ✓ Staying <span style="color: {emotionColor(emotion)}">{emotion}</span> — keep going
                    </div>
                  {/if}
                  {@render sectionList('sentiment')}
                  {#if nonNeutralTones.length > 1}
                    <div class="tone-history tone-history-bottom">
                      {#each nonNeutralTones as t, i}
                        <span class="tone-pip"
                          class:tone-positive={t === 'enthusiastic' || t === 'curious' || t === 'pleased'}
                          class:tone-negative={t === 'skeptical' || t === 'wrapping up'}
                          class:tone-latest={i === nonNeutralTones.length - 1}
                          >{t.slice(0, 3)}</span>
                      {/each}
                    </div>
                  {/if}
                </div>
              {/if}
            </div>
          {:else if col === 'right2'}
            <!-- Right2: Coaching Tips + Self-view -->
            <div class="col col-right"
              style={collapsedCols.has('right2') ? 'flex: 0 0 28px; min-width: 0; width: 28px;' : 'flex: 1; min-width: 120px;'}
              ondragover={onColDragOver} ondrop={(e) => onColDrop('right2', e)}>
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div class="col-header col-drag-handle" draggable={true} ondragstart={(e) => onColDragStart('right2', e)}>
                <span class="col-label">{collapsedCols.has('right2') ? '…' : 'Resources'}</span>
                {#if !collapsedCols.has('right2')}
                  <div class="zoom-btns">
                    <button class="zoom-btn" onclick={() => adjustZoom('rightBottom', -10)} title="Decrease font size">A−</button>
                    <button class="zoom-btn" onclick={() => adjustZoom('rightBottom', +10)} title="Increase font size">A+</button>
                    <button class="zoom-btn collapse-btn" onclick={() => toggleColCollapse('right2')} title="Collapse">▾</button>
                  </div>
                {:else}
                  <button class="zoom-btn collapse-btn" onclick={() => toggleColCollapse('right2')} title="Expand">▸</button>
                {/if}
              </div>
              {#if !collapsedCols.has('right2')}
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div class="right-panel-scroll" style="zoom: {rightBottomZoom/100}" ondragover={(e) => { e.preventDefault(); }} ondrop={(e) => onPanelEmptyDrop('coaching', e)}>
                  {@render sectionList('coaching')}
                </div>
              {/if}
            </div>
          {/if}
        {/each}
      </div>

      {#snippet sectionList(panelId: string)}
        {@const panelSections = sectionLayout.filter(s => s.panel === panelId && s.id !== 'keywords')}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="section-drop-zone" ondragover={(e) => { e.preventDefault(); }} ondrop={(e) => onPanelEmptyDrop(panelId, e)}>
          {#each panelSections as slot, i (slot.id)}
            {@const sid = slot.id}
            {@const myRole = SECTION_ROLE[sid] ?? 'coaching'}
            {@const prevRole = i > 0 ? (SECTION_ROLE[panelSections[i-1].id] ?? 'coaching') : null}
            {#if prevRole !== myRole && !(prevRole === null && myRole === 'coaching')}
              <div class="group-divider group-divider-{myRole}">
                {myRole === 'interviewer' ? '👤 Interviewer' : myRole === 'you' ? '✅ Your Performance' : 'Resources'}
              </div>
            {/if}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div class="section-wrapper"
              class:drop-above={sectionDropTarget?.id === sid && sectionDropTarget.pos === 'above'}
              class:drop-below={sectionDropTarget?.id === sid && sectionDropTarget.pos === 'below'}
              class:section-dragging={draggingSection === sid}
              class:section-interviewer={myRole === 'interviewer'}
              class:section-you={myRole === 'you'}
              ondragover={(e) => onSectionDragOver(sid, e)}
              ondragleave={onSectionDragLeave}
              ondrop={(e) => onSectionDrop(sid, panelId, e)}>

              {#if collapsedSections.has(sid)}
                <div class="section-collapsed-bar">
                  <!-- svelte-ignore a11y_no_static_element_interactions -->
                  <div class="section-drag-handle-inline" draggable={true} ondragstart={(e) => onSectionDragStart(sid, e)} ondragend={onSectionDragEnd}>⠿</div>
                  <span class="section-name-sm">{SECTION_LABELS[sid] ?? sid}</span>
                  <button class="section-expand-btn" onclick={() => toggleSectionCollapse(sid)}>▸</button>
                </div>
              {:else}
                <div class="section-header-row">
                  <!-- svelte-ignore a11y_no_static_element_interactions -->
                  <div class="section-drag-handle" draggable={true} ondragstart={(e) => onSectionDragStart(sid, e)} ondragend={onSectionDragEnd}>⠿</div>
                  <button class="section-collapse-btn" onclick={() => toggleSectionCollapse(sid)}>▾</button>
                </div>
                {#if sid === 'screen-preview'}
                {:else if sid === 'personality'}
                  <!-- merged into sentiment-bar -->
                {:else if sid === 'sentiment-bar'}
                  {#if presenceIssues.length > 0}<BodyLanguagePanel {presenceIssues} />{/if}
                  {#if emotionHistory.length > 1}
                    <div class="emotion-sparkline">
                      {#each emotionHistory.slice(-15) as emo}
                        <span class="emotion-dot" style="background: {emotionColor(emo)}" title={emo}></span>
                      {/each}
                    </div>
                  {/if}
                  {#if coachingLog.length > 0}
                    <div class="coaching-log coaching-log-sentiment">
                      {#each coachingLog.slice().reverse() as entry, i}
                        <!-- svelte-ignore a11y_no_static_element_interactions -->
                        <div class="coaching-log-entry" class:coaching-log-latest={i === 0}
                          class:coaching-log-latest-positive={i === 0 && POSITIVE_EMOTIONS.has(entry.emotion)}
                          onclick={() => {
                            if (!entry.why && !entry.reason) return;
                            const s = new Set(expandedCoachingEntries);
                            s.has(entry.time) ? s.delete(entry.time) : s.add(entry.time);
                            expandedCoachingEntries = s;
                          }}>
                          {#if i === 0 && entry.type === 'personality'}
                            <!-- Latest: personality read -->
                            <div class="coaching-log-meta">
                              <span class="coaching-log-emotion">
                                <span class="coaching-log-who">Interviewer</span>
                                <span class="coaching-log-icon">🧠</span>
                                <span style="color: {entry.color ?? '#a78bfa'}">{entry.emotion}</span>
                              </span>
                              {#if entry.why}<span class="coaching-log-expand-hint">{expandedCoachingEntries.has(entry.time) ? '▾' : '▸'} Profile</span>{/if}
                            </div>
                            <span class="coaching-log-text coaching-log-text-latest"
                              style="color: {entry.color ?? '#a78bfa'}">
                              <span class="tip-label tip-label-personality" style="color: {entry.color ?? '#a78bfa'}; border-color: {entry.color ?? '#a78bfa'}">Tip</span> {entry.text}
                            </span>
                          {:else if i === 0}
                            <!-- Latest: interviewer label + icon + emotion, no timestamp -->
                            <div class="coaching-log-meta">
                              <span class="coaching-log-emotion">
                                <span class="coaching-log-who">Interviewer</span>
                                <span class="coaching-log-icon">{EMOTION_CONFIG[entry.emotion]?.icon ?? ''}</span>
                                <span style="color: {emotionColor(entry.emotion)}">{entry.emotion}</span>
                              </span>
                            </div>
                            <span class="coaching-log-text coaching-log-text-latest"
                              class:coaching-log-positive={POSITIVE_EMOTIONS.has(entry.emotion)}
                              class:coaching-log-clickable={!!(entry.why || entry.reason)}>
                              <span class="tip-label" class:tip-label-positive={POSITIVE_EMOTIONS.has(entry.emotion)}>
                                {POSITIVE_EMOTIONS.has(entry.emotion) ? '✓' : 'Tip'}
                              </span> {entry.text}
                            </span>
                          {:else if entry.type === 'personality'}
                            <!-- History: personality read -->
                            <div class="coaching-log-meta">
                              <span style="color: {entry.color ?? '#a78bfa'}" class="coaching-log-emotion-hist">🧠 {entry.emotion}</span>
                              <span class="coaching-log-ago">{fmtAgo(entry.time)}</span>
                            </div>
                            <span class="coaching-log-text-hist" class:coaching-log-clickable={!!(entry.why || entry.reason)}>{entry.text}</span>
                          {:else}
                            <!-- History: no icon, no Tip label, timestamp, dimmer -->
                            <div class="coaching-log-meta">
                              <span style="color: {emotionColor(entry.emotion)}" class="coaching-log-emotion-hist">{entry.emotion}</span>
                              <span class="coaching-log-ago">{fmtAgo(entry.time)}</span>
                            </div>
                            <span class="coaching-log-text-hist" class:coaching-log-clickable={!!(entry.why || entry.reason)}>{entry.text}</span>
                          {/if}
                          {#if (entry.why || entry.reason) && expandedCoachingEntries.has(entry.time)}
                            {#if entry.type === 'personality'}
                              <div class="coaching-log-profile-section">
                                <span class="coaching-log-profile-label">Profile</span>
                                {#if entry.reason}<span class="coaching-log-profile-text">{entry.reason}</span>{/if}
                                {#if entry.why}<span class="coaching-log-profile-text">{entry.why}</span>{/if}
                              </div>
                            {:else}
                              {#if entry.reason}<span class="coaching-log-reason">{entry.reason}</span>{/if}
                              {#if entry.why}<span class="coaching-log-why">{entry.why}</span>{/if}
                            {/if}
                          {/if}
                        </div>
                      {/each}
                    </div>
                  {/if}
                {:else if sid === 'body-language'}
                  <!-- merged into sentiment-bar -->
                {:else if sid === 'energy-coach'}
                  <!-- merged into stats section -->
                {:else if sid === 'fillers'}
                  <!-- merged into stats section -->
                {:else if sid === 'salary-coach'}
                  {#if salaryTactics}
                    <SalaryCoachPanel tactics={salaryTactics} onClose={() => salaryTactics = null} />
                  {/if}
                {:else if sid === 'next-question'}
                  <!-- prediction runs in background; panel hidden from display -->
                {:else if sid === 'keywords'}
                  {#if jdKeywords.length > 0}
                    <div class="side-section">
                      <div class="side-section-label">Keywords</div>
                      <KeywordTrackerPanel keywords={jdKeywords} mentionedSet={mentionedKeywords} flashSet={flashingKeywords} interviewerRaisedSet={interviewerRaisedKeywords} />
                    </div>
                  {/if}
                {:else if sid === 'company-brief'}
                  {#if companyBrief}
                    <CompanyBriefPanel brief={companyBrief} />
                  {/if}
                {:else if sid === 'interviewer-profiles'}
                  {#if interviewerSummaries.length > 0}
                    <InterviewerProfilePanel interviewers={interviewerSummaries} />
                  {/if}
                {:else if sid === 'stats'}
                  <div class="side-stats">
                    <div class="side-stat" title="Time since you started answering. Green = under 30s (concise), amber = 30–60s (detailed), red = over 60s (wrap up). Aim for 30–90 seconds per answer.">
                      <span class="side-label">Answer</span>
                      <span class="side-value" style="color: {timerColor}">{answerMs > 0 ? fmtTime(answerMs) : '—'}</span>
                    </div>
                    <div class="side-stat" title="Your share of speaking time vs the interviewer. Aim for 40–60% — enough to show depth, while leaving room for the interviewer to lead.">
                      <span class="side-label">You / Them</span>
                      <span class="side-value" style="color: {ratioColor}">{youPct > 0 ? `${youPct}% / ${interviewerPct}%` : '—'}</span>
                    </div>
                    <div class="side-stat side-stat-pace" title="Your speaking pace in words per minute">
                      <EnergyCoachPanel
                        wpm={paceReading.wordsPerMinute}
                        status={paceReading.status}
                        tip={paceReading.tip}
                      />
                    </div>
                    <div class="side-stat" title="Filler word count — words to avoid">
                      <span class="side-label">Fillers</span>
                      <span class="side-value" class:filler-active={fillerTotal > 0} style="color: {fillerTotal > 0 ? '#f59e0b' : '#475569'}">
                        {fillerTotal > 0 ? fillerTotal : '—'}
                      </span>
                    </div>
                    {#if allFillerCounts.length > 0}
                      <div class="filler-list">
                        {#each allFillerCounts.sort((a, b) => b.count - a.count) as f}
                          <span class="filler-tag">"{f.word}" <strong>×{f.count}</strong></span>
                        {/each}
                      </div>
                    {/if}
                  </div>
                  {#if energySignal || youLog.length > 0 || suggestions.some(s => s.answerFeedback || s.vocalFeedback)}
                    {@const latestFeedback = suggestions.slice().reverse().find(s => s.answerFeedback || s.vocalFeedback)}
                    <div class="you-log">
                      <button class="you-log-header-btn" onclick={() => youDeliveryExpanded = !youDeliveryExpanded} title="Click to show/hide Your Delivery tips">
                        <span class="you-log-header">Your Delivery</span>
                        <span class="you-log-header-tip">tip</span>
                        <span class="you-log-header-chevron">{youDeliveryExpanded ? '▾' : '▸'}</span>
                      </button>
                      {#if youDeliveryExpanded}
                        {#if latestFeedback}
                          {#if latestFeedback.vocalFeedback}
                            {@const vf = latestFeedback.vocalFeedback}
                            {@const scoreColor = vf.confidence_score >= 70 ? '#4ade80' : vf.confidence_score >= 45 ? '#f59e0b' : '#f87171'}
                            <div class="you-delivery-row">
                              <button class="side-stat ascore-stat-btn" title="Click to expand coaching details" onclick={() => vocalWhyExpanded = !vocalWhyExpanded}>
                                <span class="side-label">Voice Read</span>
                                <span class="ascore-vocal-score" style="color: {scoreColor}">{vf.confidence_score}%</span>
                                <span class="ascore-vocal-tone">{vf.tone}</span>
                                {#if vf.fillers_noted}<span class="ascore-vocal-fillers">{vf.fillers_noted}</span>{/if}
                                <span class="ascore-vocal-chevron">{vocalWhyExpanded ? '▾' : '▸'}</span>
                              </button>
                              {#if vocalWhyExpanded && vf.coaching}
                                <p class="ascore-coaching you-log-coaching">{vf.coaching}</p>
                              {/if}
                            </div>
                          {/if}
                          {#if latestFeedback.answerFeedback}
                            {@const af = latestFeedback.answerFeedback}
                            <div class="you-delivery-row">
                              <button class="side-stat ascore-stat-btn" title="Click to expand answer coaching" onclick={() => answerWhyExpanded = !answerWhyExpanded}>
                                <span class="side-label">Answer</span>
                                {#if af.missed_followup}<span class="ascore-badge ascore-warn">No follow-up</span>{/if}
                                {#if af.missed_metric}<span class="ascore-badge ascore-warn">Add metric</span>{/if}
                                {#if !af.missed_followup && !af.missed_metric}<span class="side-value" style="color: #4ade80">✓</span>{/if}
                                <span class="ascore-vocal-chevron">{answerWhyExpanded ? '▾' : '▸'}</span>
                              </button>
                              {#if answerWhyExpanded && af.coaching}
                                <p class="ascore-coaching you-log-coaching">{af.coaching}</p>
                              {/if}
                            </div>
                          {/if}
                        {/if}
                        {#if energySignal}
                          <div class="you-log-entry you-log-cue" title="Detected from your recent transcript">
                            <span class="you-log-cue-text">{energySignal}</span>
                          </div>
                        {/if}
                        {#each youLog.slice().reverse() as entry, i}
                          <div class="you-log-entry" class:you-log-latest={i === 0 && !energySignal && !latestFeedback}>
                            <div class="you-log-meta">
                              <span class="you-log-who">You</span>
                              <span class="you-log-icon">✓</span>
                              {#if i > 0}<span class="you-log-ago">{fmtAgo(entry.time)}</span>{/if}
                            </div>
                            <span class="you-log-text">{entry.text}</span>
                          </div>
                        {/each}
                      {/if}
                    </div>
                  {/if}
                {:else if sid === 'answer-score'}
                  <!-- merged into Your Delivery (stats section) -->
                {:else if sid === 'rate-limits'}
                  <div class="side-section">
                    <button class="side-section-toggle" onclick={() => { modelUsageExpanded = !modelUsageExpanded; localStorage.setItem('model-usage-expanded', String(modelUsageExpanded)); }}>
                      <span class="side-section-label">Model Usage</span>
                      <span class="side-section-chevron">{modelUsageExpanded ? '▴' : '▾'}</span>
                    </button>
                    {#if modelUsageExpanded}
                      <RateLimitPanel {rateLimits} {callCounts} {providerStatus} />
                    {/if}
                  </div>
                {/if}
              {/if}
            </div>
          {/each}
          {#if panelSections.length === 0}
            <div class="section-empty-hint">Drop sections here</div>
          {/if}
        </div>
      {/snippet}

      {#if showNextSteps && nextSteps.length > 0}
        <div class="next-steps-strip">
          <div class="next-steps-header">
            <span class="next-steps-label">Next Steps</span>
            <button class="next-steps-close" onclick={() => showNextSteps = false}>✕</button>
          </div>
          <div class="next-steps-list">
            {#each nextSteps as step}
              <div class="next-step-item">· {step}</div>
            {/each}
          </div>
        </div>
      {:else if loadingNextSteps}
        <div class="next-steps-strip next-steps-loading">Detecting next steps…</div>
      {/if}
      {#if !showNextSteps && nextSteps.length > 0}
        <button class="next-steps-pill" onclick={() => showNextSteps = true}>
          {nextSteps.length} next step{nextSteps.length > 1 ? 's' : ''} →
        </button>
      {/if}

      {#if jdKeywords.length > 0}
        <div class="keywords-bar" style="height: {kwBarH}px">
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div class="kw-resize-handle" onmousedown={startKwResize}></div>
          <div class="keywords-bar-meta">
            <span class="keywords-bar-label">Keywords</span>
            <div class="zoom-btns kw-zoom-btns">
              <button class="zoom-btn" onclick={() => adjustZoom('kw', -10)} title="Decrease font size">A−</button>
              <button class="zoom-btn" onclick={() => adjustZoom('kw', +10)} title="Increase font size">A+</button>
            </div>
          </div>
          <div class="keywords-bar-content" style="zoom: {kwZoom/100}">
            <KeywordTrackerPanel keywords={jdKeywords} mentionedSet={mentionedKeywords} flashSet={flashingKeywords} interviewerRaisedSet={interviewerRaisedKeywords} {keywordQuestionMap} horizontal={true} />
          </div>
        </div>
      {/if}
    </div>

    {#if focusMode}
      <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
      <div class="focus-overlay" onclick={() => focusMode = false}>
        {#if screenStream}
          <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
          <div class="focus-video-wrap" onclick={(e) => e.stopPropagation()}>
            {#if cropRect}
              <div class="focus-crop-shell" style="aspect-ratio:{cropRect.w * videoNaturalAR / cropRect.h};overflow:hidden;position:relative;">
                <!-- svelte-ignore a11y_media_has_caption -->
                <video bind:this={focusVideoEl} autoplay muted playsinline
                  style="position:absolute;width:{100/cropRect.w}%;height:auto;transform:translate({-cropRect.x*100}%,{-cropRect.y*100}%);"></video>
              </div>
            {:else}
              <!-- svelte-ignore a11y_media_has_caption -->
              <video bind:this={focusVideoEl} class="focus-video" autoplay muted playsinline></video>
            {/if}
          </div>
        {/if}
        <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
        <div class="focus-card" onclick={(e) => e.stopPropagation()}>
          {#if latestSuggestion}
            {@const fp = parseSuggestion(latestSuggestion.suggestion ?? '')}
            {@const fcues = parseCues(fp.body)}
            <div class="focus-question">"{latestSuggestion.question}"</div>
            {#if latestSuggestion.suggestion}
              {#if fp.acknowledge}
                <div class="focus-section">
                  <span class="focus-badge focus-badge-ack">Acknowledge</span>
                  <span class="focus-ack">{fp.acknowledge}</span>
                </div>
              {/if}
              {#if fp.tell}
                <div class="focus-section">
                  <span class="focus-badge">Answer</span>
                  <span class="focus-tell">{@html renderBold(fp.tell)}</span>
                  {#if latestSuggestion.streaming && !fp.body}<span class="focus-cursor">|</span>{/if}
                </div>
              {/if}
              {#if fcues.length > 0}
                <div class="focus-cues">
                  {#each fcues as cue}
                    <div class="focus-cue">
                      <span class="focus-cue-label">{cue.typeTag || (cue.label === 'General' ? 'Point' : cue.label)}</span>
                      <span class="focus-cue-text">{@html renderBold(cue.text)}</span>
                    </div>
                  {/each}
                  {#if latestSuggestion.streaming}<span class="focus-cursor">|</span>{/if}
                </div>
              {/if}
              {#if fp.asks.length > 0}
                <div class="focus-section">
                  <span class="focus-badge focus-badge-ask">Ask</span>
                  <div class="focus-asks">
                    {#each fp.asks as ask}
                      <span class="focus-ask-item">{ask.question}</span>
                    {/each}
                  </div>
                </div>
              {/if}
            {:else if latestSuggestion.streaming}
              <span class="focus-loading">Generating...</span>
            {/if}
          {:else}
            <div class="focus-empty">Waiting for a question...</div>
          {/if}
        </div>
        <div class="focus-hint">glance at bold keywords · F or click outside to exit</div>
      </div>
    {/if}

    {#if showCropPicker && screenStream}
      <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
      <div class="crop-picker-bg" onclick={(e) => { if (e.target === e.currentTarget) { showCropPicker = false; pickerDrag = null; pickerPendingRect = null; } }}>
        <div class="crop-picker-dialog">
          <div class="crop-picker-header">
            <span>Drag to select the interviewer's video tile</span>
            <button class="crop-picker-close" onclick={() => { showCropPicker = false; pickerDrag = null; pickerPendingRect = null; }}>✕</button>
          </div>
          <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
          <div class="crop-picker-vwrap" role="presentation"
            style="aspect-ratio:{videoNaturalAR}"
            onmousedown={pickerDown}
            onmousemove={pickerMove}
            onmouseup={pickerUp}
          >
            <!-- svelte-ignore a11y_media_has_caption -->
            <video bind:this={pickerVideoEl} class="crop-picker-vid" autoplay muted playsinline></video>
            {#if pickerDrag ?? pickerPendingRect}
              {@const d = pickerDrag ?? { sx: pickerPendingRect!.x, sy: pickerPendingRect!.y, cx: pickerPendingRect!.x + pickerPendingRect!.w, cy: pickerPendingRect!.y + pickerPendingRect!.h }}
              {@const rx = Math.min(d.sx, d.cx) * 100}
              {@const ry = Math.min(d.sy, d.cy) * 100}
              {@const rw = Math.abs(d.cx - d.sx) * 100}
              {@const rh = Math.abs(d.cy - d.sy) * 100}
              <div class="crop-picker-sel" style="left:{rx}%;top:{ry}%;width:{rw}%;height:{rh}%"></div>
            {/if}
          </div>
          <div class="crop-picker-footer">
            <span class="crop-picker-hint">Click and drag over any interviewer face tile · selection shown in blue</span>
            <button class="crop-apply-btn" onclick={applyFaceCrop} disabled={!pickerPendingRect}>Apply</button>
            <button class="crop-cancel-btn" onclick={() => { showCropPicker = false; pickerDrag = null; pickerPendingRect = null; }}>Cancel</button>
          </div>
        </div>
      </div>
    {/if}

    {#if showDebrief}
      <DebriefModal
        {transcript}
        {suggestions}
        onClose={() => showDebrief = false}
        onSave={(r) => saveInterview({
          summary: r.summary,
          strong_points: r.strong_points,
          improvement_areas: r.improvement_areas,
          rehearsal_questions: r.improvement_areas.map(a => `Practice answering: ${a}`),
        })}
      />
    {/if}
    {#if showWhisper && whisperTell}
      <WhisperOverlay tell={whisperTell} onClose={() => showWhisper = false} />
    {/if}
  {/if}
{#if showHistory}
  <InterviewHistoryPanel
    onClose={() => showHistory = false}
    onRehearsal={(questions) => { predictedQuestions = questions; phase = 'practice'; connectWs(); showHistory = false; }}
  />
{/if}
{#if showReviewUpload}
  <ReviewUpload
    onReport={(r) => { reviewReport = r; showReviewUpload = false; showReviewPanel = true; }}
    onCancel={() => showReviewUpload = false}
  />
{/if}
{#if showReviewPanel && reviewReport}
  <ReviewPanel
    report={reviewReport}
    onClose={() => showReviewPanel = false}
    onDelete={(id) => { if (reviewReport?.id === id) reviewReport = null; showReviewPanel = false; }}
  />
{/if}
</main>

<style>
  :root {
    /* Typography */
    --ff-base: 'Inter', system-ui, sans-serif;
    --ff-mono: 'JetBrains Mono', 'Fira Code', monospace;
    --fs-xs:   0.62rem;
    --fs-sm:   0.74rem;
    --fs-base: 0.85rem;
    --fs-lg:   1.0rem;
    --fs-mono: 0.72rem;

    /* Text colors */
    --clr-text-primary:   #f1f5f9;
    --clr-text-secondary: #94a3b8;
    --clr-text-muted:     #475569;
    --clr-text-dim:       #334155;

    /* Speaker / brand colors */
    --clr-speaker-you:  #4ade80;
    --clr-speaker-them: #f87171;
    --clr-green:        #22c55e;
    --clr-blue:         #60a5fa;
    --clr-purple:       #a78bfa;
    --clr-amber:        #f59e0b;

    /* Surface backgrounds */
    --bg-surface:    #0f172a;
    --bg-card:       #1e293b;
    --bg-entry-you:  #0a1f14;
    --bg-entry-them: #1a0a0a;

    /* Section block colors (Acknowledge / Say / Ask) */
    --bg-ack:     #110823;
    --border-ack: #6d28d9;
    --bg-say:     #060e0a;
    --border-say: #166534;
    --bg-ask:     #0e0700;
    --border-ask: #92400e;

    /* Borders */
    --border-subtle: #1e293b;
    --border-muted:  #334155;

    /* Radius */
    --radius-sm:   0.25rem;
    --radius-md:   0.4rem;
    --radius-lg:   0.5rem;
    --radius-pill: 9999px;
  }

  main {
    min-height: 100vh;
    font-family: var(--ff-base);
  }

  .app { font-family: var(--ff-base); }

  /* Ensure form elements inherit the chosen font */
  :global(input), :global(select), :global(button), :global(textarea) {
    font-family: inherit;
  }

  /* === Global utility classes === */
  :global(.label-uppercase) {
    font-size: var(--fs-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.07em;
  }
  :global(.panel-empty) {
    color: var(--clr-text-muted);
    font-style: italic;
    font-size: var(--fs-base);
    text-align: center;
    padding: 2rem 1rem;
  }
  :global(.text-secondary) { color: var(--clr-text-secondary); }
  :global(.text-muted)     { color: var(--clr-text-muted); }
  :global(.text-dim)       { color: var(--clr-text-dim); }

  .setup-container { max-width: 800px; margin: 0 auto; }
  .setup-usage { max-width: 800px; margin: 0 auto 2rem; padding: 0 2rem; }
  .setup-review-row { max-width: 800px; margin: 0.75rem auto 0; padding: 0 2rem; display: flex; align-items: center; gap: 0.75rem; flex-wrap: wrap; }
  .setup-review-btn {
    padding: 0.4rem 1rem; background: transparent;
    border: 1px solid #334155; border-radius: 0.4rem;
    color: #64748b; font-size: 0.82rem; cursor: pointer;
    transition: all 0.15s;
  }
  .setup-review-btn:hover { border-color: #60a5fa; color: #60a5fa; }
  /* Review list panel */
  .review-list-panel {
    max-width: 800px; margin: 0.5rem auto 0; padding: 0 2rem;
    display: flex; flex-direction: column; gap: 0.5rem;
  }
  .review-search {
    width: 100%; padding: 0.4rem 0.75rem; background: #0a1525;
    border: 1px solid #1e3a5f; border-radius: 0.4rem;
    color: #cbd5e1; font-size: var(--fs-sm); outline: none;
  }
  .review-search:focus { border-color: #3b82f6; }
  .review-list-empty { color: #475569; font-size: var(--fs-sm); font-style: italic; margin: 0; text-align: center; padding: 0.75rem 0; }
  .review-list { display: flex; flex-direction: column; gap: 0.4rem; max-height: 300px; overflow-y: auto; }
  .review-list-item {
    display: flex; flex-direction: column; gap: 0.25rem;
    padding: 0.5rem 0.75rem; background: #060e1a;
    border: 1px solid #1a2d4a; border-radius: 0.4rem;
  }
  .review-list-meta { display: flex; align-items: baseline; justify-content: space-between; gap: 0.5rem; }
  .review-list-name { font-size: var(--fs-sm); font-weight: 600; color: #94a3b8; }
  .review-list-date { font-size: var(--fs-xs); color: #334155; flex-shrink: 0; }
  .review-list-summary { font-size: var(--fs-xs); color: #475569; margin: 0; line-height: 1.4; }
  .review-list-actions { display: flex; gap: 0.5rem; }
  .review-list-open {
    padding: 0.2rem 0.6rem; background: rgba(59,130,246,0.1);
    border: 1px solid rgba(59,130,246,0.3); border-radius: 0.3rem;
    color: #60a5fa; font-size: var(--fs-xs); cursor: pointer; transition: all 0.15s;
  }
  .review-list-open:hover { background: rgba(59,130,246,0.2); }
  .review-list-delete {
    padding: 0.2rem 0.6rem; background: none;
    border: 1px solid #1e293b; border-radius: 0.3rem;
    color: #475569; font-size: var(--fs-xs); cursor: pointer; transition: all 0.15s;
  }
  .review-list-delete:hover { border-color: #ef4444; color: #ef4444; }

  .setup-review-view-btn {
    padding: 0.4rem 1rem; background: rgba(59,130,246,0.1);
    border: 1px solid rgba(59,130,246,0.3); border-radius: 0.4rem;
    color: #93c5fd; font-size: 0.82rem; cursor: pointer;
    transition: all 0.15s; max-width: 320px;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  }
  .setup-review-view-btn:hover { border-color: #60a5fa; }
  .save-report-btn { opacity: 0.85; }
  .save-report-btn.saving { opacity: 0.5; cursor: default; }
  .setup-header { text-align: center; padding: 3rem 2rem 1rem; }
  .setup-font-row { display: flex; align-items: center; justify-content: center; gap: 0.5rem; margin-top: 1rem; }
  .setup-font-label { font-size: var(--fs-sm); color: #64748b; }
  .setup-header h1 {
    font-size: 2.5rem; font-weight: 800;
    background: linear-gradient(135deg, #60a5fa, #a78bfa);
    -webkit-background-clip: text; -webkit-text-fill-color: transparent;
  }
  .setup-header p { color: #64748b; margin-top: 0.5rem; }

  .side-section { display: flex; flex-direction: column; gap: 0.3rem; }
  .side-section-label { font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase; letter-spacing: 0.07em; color: #334155; }
  .side-section-toggle { display: flex; align-items: center; justify-content: space-between; width: 100%; background: none; border: none; cursor: pointer; padding: 0; }
  .side-section-toggle:hover .side-section-label { color: #475569; }
  .side-section-chevron { font-size: var(--fs-xs); color: #1e293b; }


  .interview-layout { display: flex; flex-direction: column; height: 100vh; }
  .interview-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 0.4rem 1rem; background: #0f172a; border-bottom: 1px solid #1e293b; flex-shrink: 0;
  }
  .header-back-btn { background: none; border: none; color: #60a5fa; font-size: var(--fs-base); font-weight: 600; cursor: pointer; padding: 0 0.5rem 0 0; white-space: nowrap; }
  .header-back-btn:hover { color: #93c5fd; }
  .interview-header h1 {
    font-size: var(--fs-lg); font-weight: 700;
    background: linear-gradient(135deg, #60a5fa, #a78bfa);
    -webkit-background-clip: text; -webkit-text-fill-color: transparent;
  }
  .header-right { display: flex; align-items: center; gap: 0.75rem; }
  .shortcuts-hint { font-size: var(--fs-sm); color: #334155; white-space: nowrap; }
  .debrief-btn {
    padding: 0.3rem 0.8rem; background: transparent;
    border: 1px solid #334155; border-radius: 0.375rem;
    color: #64748b; font-size: var(--fs-base); cursor: pointer; white-space: nowrap;
  }
  .debrief-btn:hover { border-color: #a78bfa; color: #a78bfa; }

  .history-btn {
    padding: 0.3rem 0.8rem; background: transparent;
    border: 1px solid #334155; border-radius: 0.375rem;
    color: #64748b; font-size: var(--fs-base); cursor: pointer; white-space: nowrap;
  }
  .history-btn:hover { border-color: #60a5fa; color: #60a5fa; }

  .personality-strip {
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    padding: 0.4rem 0.75rem;
    border-left: 3px solid;
    background: #080d18;
    border-bottom: 1px solid #1e293b;
  }
  .personality-label {
    font-size: var(--fs-xs);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .personality-desc {
    font-size: var(--fs-sm);
    color: #64748b;
    line-height: 1.3;
    font-style: italic;
  }
  .personality-tip {
    font-size: var(--fs-sm);
    color: #94a3b8;
    line-height: 1.4;
    margin-top: 0.25rem;
    padding-top: 0.25rem;
    border-top: 1px solid #1e293b;
  }
  .side-pace-tip { font-size: var(--fs-xs); font-style: italic; color: inherit; margin-left: 0.25rem; }
  .side-energy-signal {
    font-size: var(--fs-sm); color: #f59e0b; font-style: italic;
    padding: 0.15rem 0.4rem; background: #1a0f00; border-left: 2px solid #92400e;
    border-radius: 0.2rem; margin: 0.1rem 0.5rem;
  }

  /* TTS */
  .tts-controls { position: relative; display: flex; align-items: center; gap: 0.25rem; }
  .tts-btn {
    padding: 0.25rem 0.5rem; background: transparent;
    border: 1px solid #334155; border-radius: 0.375rem;
    color: #64748b; font-size: var(--fs-base); cursor: pointer; white-space: nowrap;
  }
  .tts-btn.tts-on { border-color: #22c55e; color: #22c55e; }
  .rate-label {
    display: flex; align-items: center; gap: 0.25rem;
    font-size: var(--fs-base); color: #64748b;
  }
  .rate-val { min-width: 2rem; text-align: right; font-variant-numeric: tabular-nums; }
  .rate-slider { width: 56px; accent-color: #22c55e; cursor: pointer; }
  .vol-slider { accent-color: #60a5fa; }
  .voice-pick-btn {
    padding: 0.2rem 0.35rem; background: transparent;
    border: 1px solid #334155; border-radius: 0.25rem;
    color: #64748b; font-size: var(--fs-base); cursor: pointer;
  }
  .voice-menu {
    position: absolute; top: calc(100% + 4px); right: 0; z-index: 200;
    background: #1e293b; border: 1px solid #334155; border-radius: 0.375rem;
    min-width: 200px; max-height: 250px; overflow-y: auto;
    display: flex; flex-direction: column;
  }
  .font-select {
    background: #0f172a;
    color: #94a3b8;
    border: 1px solid #1e293b;
    border-radius: 0.25rem;
    font-size: var(--fs-sm);
    padding: 0.15rem 0.3rem;
    cursor: pointer;
    height: 1.6rem;
  }
  .font-select:hover { border-color: #334155; color: #cbd5e1; }

  .voice-group-label {
    padding: 0.2rem 0.75rem 0.1rem; font-size: var(--fs-xs); font-weight: 800;
    text-transform: uppercase; letter-spacing: 0.08em; color: #475569;
  }
  .voice-row {
    display: flex; align-items: center;
  }
  .voice-row:hover { background: #334155; }
  .voice-row.selected { background: #1e3a5f; }
  .voice-option {
    flex: 1; padding: 0.3rem 0.5rem 0.3rem 0.75rem; background: transparent; border: none;
    color: #94a3b8; font-size: var(--fs-sm); cursor: pointer; text-align: left;
  }
  .voice-option:hover { color: #e2e8f0; }
  .voice-option.selected { color: #60a5fa; }
  .voice-test-btn {
    flex-shrink: 0; padding: 0.25rem 0.5rem; background: transparent; border: none;
    color: #475569; font-size: var(--fs-sm); cursor: pointer; opacity: 0.7;
    transition: color 0.15s, opacity 0.15s;
  }
  .voice-test-btn:hover { color: #60a5fa; opacity: 1; }

  .voice-test-inline-btn {
    flex-shrink: 0; padding: 0.2rem 0.55rem; background: transparent;
    border: 1px solid #334155; border-radius: 0.25rem;
    color: #60a5fa; font-size: var(--fs-base); cursor: pointer; transition: all 0.15s;
  }
  .voice-test-inline-btn:hover { background: #1e3a5f; border-color: #3b82f6; }

  .error-banner {
    display: flex; align-items: flex-start; gap: 0.75rem;
    padding: 0.5rem 1rem; background: #450a0a; color: #fca5a5; font-size: var(--fs-base); flex-shrink: 0;
  }
  .error-list { flex: 1; max-height: 6rem; overflow-y: auto; }
  .error-actions { flex-shrink: 0; display: flex; flex-direction: column; gap: 0.25rem; }
  .error-btn {
    padding: 0.15rem 0.5rem; background: transparent;
    border: 1px solid #7f1d1d; border-radius: 0.25rem;
    color: #fca5a5; font-size: var(--fs-sm); cursor: pointer;
  }
  .error-btn:hover { background: #7f1d1d; }
  .status-banner { padding: 0.2rem 1rem; background: #1e3a5f; color: #93c5fd; font-size: var(--fs-base); flex-shrink: 0; }

  /* Header title + WS dot */
  .header-title-row { display: flex; align-items: center; gap: 0.4rem; }
  .ws-header-dot {
    font-size: var(--fs-xs); color: #334155; flex-shrink: 0;
    font-variant-numeric: tabular-nums; cursor: default;
    transition: color 0.3s;
  }
  .ws-header-dot.ws-connected { color: #22c55e; }
  .ws-header-dot.ws-reconnecting { color: #f59e0b; animation: ws-pulse 1s ease-in-out infinite; }
  @keyframes ws-pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.3; } }

  /* Audio tone history */
  .tone-history { display: flex; gap: 0.25rem; flex-wrap: wrap; padding: 0.2rem 0.4rem; }
  .tone-pip {
    font-size: var(--fs-xs); padding: 0.08rem 0.3rem; border-radius: 0.2rem;
    background: #0d1525; color: #334155; border: 1px solid #1e293b;
    text-transform: capitalize; cursor: default;
  }
  .tone-pip.tone-positive { color: #4ade80; background: #071a0f; border-color: #14532d; }
  .tone-pip.tone-negative { color: #f59e0b; background: #1a1000; border-color: #92400e; }
  .tone-pip.tone-latest { font-weight: 700; opacity: 1; }

  /* Next steps strip */
  .next-steps-strip {
    flex-shrink: 0; padding: 0.5rem 1rem; background: #060e1a;
    border-top: 1px solid #1e3a5f; display: flex; flex-direction: column; gap: 0.3rem;
  }
  .next-steps-loading { color: #475569; font-size: var(--fs-sm); font-style: italic; }
  .next-steps-header { display: flex; align-items: center; justify-content: space-between; }
  .next-steps-label {
    font-size: var(--fs-xs); font-weight: 800; text-transform: uppercase;
    letter-spacing: 0.09em; color: #60a5fa;
  }
  .next-steps-close {
    background: none; border: none; color: #334155; font-size: var(--fs-sm);
    cursor: pointer; padding: 0; line-height: 1;
  }
  .next-steps-close:hover { color: #64748b; }
  .next-steps-list { display: flex; flex-wrap: wrap; gap: 0.25rem 1rem; }
  .next-step-item { font-size: var(--fs-sm); color: #94a3b8; }
  .next-steps-pill {
    align-self: flex-end; margin: 0 1rem 0.25rem;
    padding: 0.15rem 0.6rem; background: #0a1a2e; border: 1px solid #1e3a5f;
    border-radius: 9999px; color: #60a5fa; font-size: var(--fs-sm); cursor: pointer;
    transition: all 0.15s;
  }
  .next-steps-pill:hover { background: #0f2540; }

  .keywords-bar {
    flex-shrink: 0;
    display: flex;
    align-items: flex-start;
    gap: 0.6rem;
    padding: 0.4rem 0.75rem 0.4rem;
    background: #080d18;
    border-top: 1px solid #1e293b;
    overflow-y: auto;
    position: relative;
  }
  .kw-resize-handle {
    position: absolute;
    top: 0; left: 0; right: 0;
    height: 5px;
    cursor: row-resize;
    background: transparent;
  }
  .kw-resize-handle:hover {
    background: rgba(59, 130, 246, 0.25);
  }
  .keywords-bar-label {
    font-size: var(--fs-xs);
    font-weight: 700;
    color: #334155;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    flex-shrink: 0;
    padding-top: 0.2rem;
  }
  .keywords-bar-content {
    flex: 1;
    min-width: 0;
  }

  /* Resizable 4-column layout */
  .three-col {
    flex: 1;
    display: flex;
    overflow: hidden;
    background: #070c14;
    min-height: 0;
  }

  .col {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    flex-shrink: 0;
  }

  .col-right {
    flex-shrink: 0;
    min-width: 120px;
    background: #080d18;
  }

  /* Drag resize handles */
  .resize-handle {
    width: 5px;
    flex-shrink: 0;
    background: #0f172a;
    cursor: col-resize;
    transition: background 0.15s;
  }
  .resize-handle:hover { background: #1e293b; }

  /* Column header with zoom controls */
  .col-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.35rem 0.75rem 0;
    flex-shrink: 0;
  }
  .col-drag-handle {
    cursor: grab;
    user-select: none;
  }
  .col-drag-handle:hover {
    background: rgba(59, 130, 246, 0.08);
    border-radius: 4px;
  }
  .col-drag-handle:active {
    cursor: grabbing;
  }

  /* Section drag-and-drop */
  .section-drop-zone { min-height: 20px; }
  .section-empty-hint {
    font-size: var(--fs-xs); color: #1e293b; text-align: center;
    padding: 0.5rem; font-style: italic;
  }
  .section-wrapper {
    position: relative;
    padding-left: 14px;
  }
  .section-drag-handle {
    position: absolute;
    top: 6px;
    left: 1px;
    width: 12px;
    font-size: var(--fs-base);
    color: #334155;
    cursor: grab;
    user-select: none;
    opacity: 0;
    z-index: 5;
    line-height: 1;
  }
  .section-wrapper:hover > .section-header-row { opacity: 1; }
  .section-drag-handle:hover { opacity: 1 !important; color: #64748b; }
  .section-drag-handle:active { cursor: grabbing; }
  .section-wrapper.section-dragging { opacity: 0.35; }
  .section-wrapper.drop-above { border-top: 2px solid #3b82f6; }
  .section-wrapper.drop-below { border-bottom: 2px solid #3b82f6; }
  .section-wrapper.section-interviewer { border-left: 3px solid rgba(239, 68, 68, 0.35); }
  .section-wrapper.section-you { border-left: 3px solid rgba(245, 158, 11, 0.3); }

  .section-role-badge {
    position: absolute;
    top: 3px;
    right: 2px;
    font-size: 0.57rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    opacity: 0.45;
    pointer-events: none;
  }
  .role-interviewer { color: #ef4444; }
  .role-you { color: #f59e0b; }

  .group-divider {
    font-size: var(--fs-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.09em;
    padding: 0.5rem 0.5rem 0.15rem;
    margin-top: 0.1rem;
    border-top: 1px solid #1e293b;
  }
  .group-divider:first-child { border-top: none; margin-top: 0; padding-top: 0.15rem; }
  .group-divider-interviewer { color: rgba(239, 68, 68, 0.6); }
  .group-divider-you { color: rgba(245, 158, 11, 0.6); }
  .group-divider-coaching { color: #334155; }

  /* Collapsed section bar */
  .section-collapsed-bar {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    padding: 0.15rem 0.3rem 0.15rem 0;
    cursor: default;
  }
  .section-drag-handle-inline {
    font-size: var(--fs-base); color: #334155; cursor: grab;
    user-select: none; line-height: 1; width: 12px;
  }
  .section-drag-handle-inline:active { cursor: grabbing; }
  .section-name-sm {
    font-size: var(--fs-xs); color: #334155; text-transform: uppercase;
    letter-spacing: 0.08em; font-weight: 600; flex: 1;
  }
  .section-expand-btn {
    background: none; border: none; color: #475569;
    font-size: var(--fs-xs); cursor: pointer; padding: 0; line-height: 1;
  }
  .section-expand-btn:hover { color: #94a3b8; }

  .col-label {
    font-size: var(--fs-xs);
    font-weight: 700;
    color: #334155;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }

  .new-q-badge {
    font-size: 0.6em;
    font-weight: 800;
    color: #fff;
    background: #2563eb;
    border-radius: 999px;
    padding: 0.05em 0.45em;
    letter-spacing: 0;
    text-transform: none;
    animation: badge-pulse 1.5s ease-in-out infinite;
  }
  @keyframes badge-pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.55; }
  }

  .zoom-btns {
    display: flex;
    gap: 0.15rem;
    opacity: 0;
    transition: opacity 0.15s;
  }
  .col-header:hover .zoom-btns { opacity: 1; }

  .provider-chip {
    font-size: 0.6rem;
    font-weight: 600;
    letter-spacing: 0.03em;
    padding: 0.08rem 0.35rem;
    border-radius: 9999px;
    background: #1e3a5f;
    color: #7eb8f7;
    border: 1px solid #2a4a7f;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 90px;
    flex-shrink: 0;
    cursor: default;
    transition: opacity 0.3s;
  }
  .provider-chip-local {
    background: #14302a;
    color: #4ade80;
    border-color: #166534;
  }

  .zoom-btn {
    padding: 0.08rem 0.28rem;
    background: transparent;
    border: 1px solid #1a2540;
    border-radius: 0.2rem;
    color: #334155;
    font-size: var(--fs-xs);
    cursor: pointer;
    letter-spacing: -0.02em;
    font-family: inherit;
    line-height: 1.4;
  }
  .zoom-btn:hover { border-color: #334155; color: #64748b; }

  .col-body {
    flex: 1;
    overflow: hidden;
    padding: 0.5rem 0.75rem 0.75rem;
    display: flex;
    flex-direction: column;
  }
  .col-body-scroll { flex: 1; overflow-y: auto; min-height: 0; }

  /* Collapsed columns */
  .col-left, .col-hist, .col-center { transition: width 0.15s; overflow: hidden; }
  .col-right { transition: none; }

  /* Left column split */
  .col-split-body {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow: hidden;
    padding: 0;
    gap: 0;
  }
  .split-panel { display: flex; flex-direction: column; min-height: 0; overflow: hidden; }
  .split-panel-top { flex: 0 0 auto; border-bottom: none; }
  .split-panel-bottom { flex: 1; min-height: 0; }
  .split-panel-scroll { flex: 1; overflow-y: auto; min-height: 0; padding: 0.5rem 0.75rem 0.75rem; }
  /* Collapse buttons */
  .collapse-btn { font-size: var(--fs-xs) !important; padding: 0.05rem 0.2rem !important; color: #475569 !important; }
  .collapse-btn:hover { color: #94a3b8 !important; }

  /* Panel collapsed */
  .panel-collapsed { display: none; }

  /* Section collapse */
  .section-header-row {
    display: flex;
    align-items: center;
    position: absolute;
    top: 4px;
    left: 1px;
    gap: 2px;
    opacity: 0;
    z-index: 5;
  }
  .section-wrapper:hover > .section-header-row { opacity: 1; }
  .section-collapse-btn {
    background: none;
    border: none;
    color: #334155;
    font-size: var(--fs-xs);
    cursor: pointer;
    padding: 0;
    line-height: 1;
    width: 12px;
  }
  .section-collapse-btn:hover { color: #64748b; }

  /* Keywords bar meta */
  .keywords-bar-meta {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.15rem;
    flex-shrink: 0;
  }
  .kw-zoom-btns { flex-direction: column; gap: 0.1rem; opacity: 1 !important; }

  .col-left { background: #080d18; }
  .col-left .col-body { opacity: 0.75; }
  .col-left .col-body:hover { opacity: 1; transition: opacity 0.2s; }

  .col-hist {
    background: #060c16;
    border-right: 1px solid #0d1525;
  }

  .col-center {
    background: #07101e;
    border-right: 1px solid #0f172a;
    border-left: 1px solid #0f172a;
  }

  /* Interviewer video strip above AI Suggestions */
  .interviewer-strip {
    flex-shrink: 0;
    background: #060e1a;
    border-bottom: 1px solid #1e293b;
  }
  .interviewer-strip-vid {
    width: 100%;
    aspect-ratio: 16 / 9;
    object-fit: cover;
    display: block;
    background: #0a1525;
  }
  /* Interviewer strip zoom shell */
  .vid-zoom-shell {
    overflow: hidden;
    width: 100%;
    background: #060e1a;
  }
  /* Selfview zoom shell (overrides applied via .selfview-strip context) */
  .selfview-zoom-shell {
    overflow: hidden;
    border-radius: 0.375rem;
    border: 1px solid #1e293b;
    position: relative;
  }
  .selfview-zoom-shell.selfview-zoomed {
    overflow: hidden;
    z-index: 50;
    position: relative;
    border-color: #3b82f6;
  }
  .face-pick-row {
    display: flex; gap: 0.3rem; padding: 0.2rem 0.5rem;
    background: #060e1a;
  }
  .face-pick-btn {
    padding: 0.1rem 0.4rem; background: transparent;
    border: 1px solid #1e293b; border-radius: 0.2rem;
    color: #475569; font-size: var(--fs-xs); cursor: pointer;
  }
  .face-pick-btn:hover { border-color: #60a5fa; color: #60a5fa; }

  /* Interviewer strip bottom resize handle */
  .vid-resize-handle {
    height: 5px; cursor: ns-resize; background: transparent;
    border-top: 1px solid #1e293b; flex-shrink: 0; touch-action: none;
  }
  .vid-resize-handle:hover, .vid-resize-handle:active {
    border-top-color: #3b82f6; background: rgba(59,130,246,0.08);
  }

  /* Crop picker modal */
  .crop-picker-bg {
    position: fixed; inset: 0; background: rgba(0,0,0,0.88);
    z-index: 2000; display: flex; align-items: center; justify-content: center;
    padding: 1rem; box-sizing: border-box;
  }
  .crop-picker-dialog {
    width: 82vw; max-width: 1200px; background: #0a1525;
    border: 1px solid #1e3a5f; border-radius: 0.75rem;
    display: flex; flex-direction: column; gap: 0.6rem; padding: 0.9rem;
    max-height: calc(100vh - 2rem); overflow: hidden;
  }
  .crop-picker-header {
    display: flex; align-items: center; justify-content: space-between;
    font-size: var(--fs-sm); color: #94a3b8; flex-shrink: 0;
  }
  .crop-picker-close {
    background: transparent; border: none; color: #475569;
    font-size: var(--fs-base); cursor: pointer; padding: 0.3rem 0.5rem; line-height: 1;
  }
  .crop-picker-close:hover { color: #f87171; }
  .crop-picker-vwrap {
    position: relative; width: 100%; user-select: none; cursor: crosshair;
    overflow: hidden; background: #060e1a;
    flex: 1 1 auto; min-height: 0;
  }
  .crop-picker-vid { width: 100%; height: 100%; object-fit: fill; display: block; }
  .crop-picker-sel {
    position: absolute; border: 2px solid #60a5fa;
    background: rgba(96,165,250,0.15); pointer-events: none;
  }
  .crop-picker-footer {
    display: flex; align-items: center; gap: 0.5rem; flex-shrink: 0;
  }
  .crop-picker-hint { flex: 1; font-size: var(--fs-xs); color: #334155; font-style: italic; }
  .crop-apply-btn {
    padding: 0.3rem 0.9rem; background: #1e40af;
    border: none; border-radius: 0.375rem; color: #e2e8f0;
    font-size: var(--fs-sm); cursor: pointer;
  }
  .crop-apply-btn:disabled { opacity: 0.4; cursor: default; }
  .crop-apply-btn:not(:disabled):hover { background: #2563eb; }
  .crop-cancel-btn {
    padding: 0.3rem 0.9rem; background: transparent;
    border: 1px solid #334155; border-radius: 0.375rem; color: #64748b;
    font-size: var(--fs-sm); cursor: pointer;
  }
  .crop-cancel-btn:hover { border-color: #64748b; color: #94a3b8; }

  /* Webcam self-view */
  .selfview-strip {
    display: flex; flex-direction: column;
    border-bottom: 1px solid #1e293b;
    background: #060e1a; flex-shrink: 0;
  }
  .selfview-strip .selfview-zoom-shell {
    width: 100%; overflow: hidden;
    border-radius: 0; border: none;
  }
  .selfview-strip .selfview-zoom-shell.selfview-zoomed {
    overflow: hidden; border: none;
  }
  .selfview {
    width: 100%; height: 100%; object-fit: contain;
    background: #0a1525; display: block; transform-origin: center;
  }
  .sv-zoom-btn { display: none; }
  .selfview-label { font-size: var(--fs-xs); color: #334155; text-transform: uppercase; letter-spacing: 0.08em; padding: 0.15rem 0.75rem; }
  .selfview-resize-handle { touch-action: none; }

  /* Interviewer screen preview */
  .interviewer-preview {
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    background: #060e1a;
    border-bottom: 1px solid #1e293b;
  }
  .interviewer-video {
    width: 100%;
    aspect-ratio: 16 / 9;
    object-fit: cover;
    display: block;
    background: #0a1525;
  }
  .interviewer-label {
    font-size: var(--fs-xs);
    color: #1e3a5f;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    padding: 0.2rem 0.5rem;
    text-align: center;
  }
  /* Emotion sparkline */
  .emotion-sparkline { display: flex; flex-wrap: wrap; gap: 3px; padding: 0.2rem 0 0.1rem; }
  .emotion-dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; opacity: 0.85; transition: opacity 0.2s; }
  .emotion-dot:last-child { opacity: 1; box-shadow: 0 0 0 2px rgba(255,255,255,0.15); }

  /* Coaching log feed */
  .coaching-log {
    display: flex; flex-direction: column; gap: 0.3rem;
    padding: 0.25rem 0;
  }
  .coaching-log-sentiment {
    margin-top: 0.5rem; padding-top: 0.5rem;
    border-top: 1px solid #1e293b;
    max-height: 14rem; overflow-y: auto;
    resize: vertical; min-height: 4rem;
  }
  .coaching-log-entry {
    display: flex; flex-direction: column; gap: 0.15rem;
    padding: 0.3rem 0.55rem;
    border-radius: 0.35rem;
    border-left: 2px solid #1a1200;
  }
  .coaching-log-entry.coaching-log-latest {
    border-left-color: #92400e;
    background: #150e00;
    padding: 0.45rem 0.65rem;
  }
  .coaching-log-meta {
    display: flex; align-items: center; justify-content: space-between;
  }
  .coaching-log-icon { font-size: 0.85rem; margin-right: 0.2rem; }
  .coaching-log-who { color: #ef4444; font-weight: 800; margin-right: 0.35rem; }
  .coaching-log-who-you { color: #4ade80; font-weight: 800; margin-right: 0.35rem; }
  .you-log { display: flex; flex-direction: column; gap: 0.3rem; padding: 0.25rem 0; margin-top: 0.2rem; }
  .you-log-header { font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase; letter-spacing: 0.07em; color: #334155; }
  .you-log-header-btn { display: flex; align-items: center; gap: 0.3rem; background: none; border: none; padding: 0 0.1rem 0.1rem; cursor: pointer; width: 100%; text-align: left; }
  .you-log-header-btn:hover .you-log-header { color: #475569; }
  .you-log-header-tip { font-size: var(--fs-xs); color: #1e3a5f; background: #0a1525; border-radius: 0.2rem; padding: 0.05rem 0.3rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; }
  .you-log-header-chevron { font-size: var(--fs-xs); color: #334155; margin-left: auto; }
  .you-log-cue { background: #1a0f00 !important; border-color: #92400e !important; border-left-color: #f59e0b !important; }
  .you-log-cue-text { font-size: var(--fs-sm); color: #f59e0b; font-style: italic; line-height: 1.35; }
  .you-log-entry { display: flex; flex-direction: column; gap: 0.15rem; padding: 0.4rem 0.6rem; background: #060e0a; border: 1px solid #0f1e14; border-radius: 0.4rem; }
  .you-log-entry.you-log-latest { background: #0a1f12; border-color: #166534; border-left: 4px solid #4ade80; }
  .you-log-meta { display: flex; align-items: center; gap: 0.35rem; }
  .you-log-who { font-size: var(--fs-xs); font-weight: 800; text-transform: uppercase; letter-spacing: 0.04em; color: #4ade80; }
  .you-log-icon { font-size: var(--fs-xs); color: #4ade80; }
  .you-log-ago { font-size: var(--fs-xs); color: #334155; margin-left: auto; }
  .you-log-text { font-size: var(--fs-sm); color: #86efac; line-height: 1.35; overflow-wrap: break-word; }
  .coaching-log-emotion {
    font-size: var(--fs-xs); font-weight: 800; text-transform: uppercase;
    letter-spacing: 0.07em; display: flex; align-items: center;
    white-space: nowrap; overflow: hidden; min-width: 0;
  }
  .coaching-log-emotion-hist {
    font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase;
    letter-spacing: 0.06em; opacity: 0.6;
  }
  .coaching-log-ago {
    font-size: var(--fs-xs); color: #334155; font-family: var(--ff-mono);
  }
  /* Latest tip — prominent */
  .coaching-log-text {
    font-size: var(--fs-sm); color: #fb923c; line-height: 1.4;
  }
  .coaching-log-text-latest { font-size: var(--fs-base); }
  /* History tips — muted */
  .coaching-log-text-hist {
    font-size: var(--fs-xs); color: #64748b; line-height: 1.35;
  }
  .coaching-log-emotion-only {
    font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase;
    letter-spacing: 0.07em; color: #f59e0b; padding: 0.25rem 0.6rem;
  }
  .subject-label { color: #64748b; font-weight: 600; text-transform: uppercase; font-size: var(--fs-xs); letter-spacing: 0.06em; }
  .tip-label { color: #4ade80; font-weight: 700; font-size: var(--fs-xs); text-transform: uppercase; letter-spacing: 0.06em; }
  .coaching-log-reason { font-size: var(--fs-xs); color: #64748b; line-height: 1.35; display: block; margin-top: 0.1rem; }
  .coaching-log-why { font-size: var(--fs-xs); color: #475569; font-style: italic; line-height: 1.35; display: block; margin-top: 0.1rem; }
  .coaching-log-clickable { cursor: pointer; text-decoration-line: underline; text-decoration-style: dotted; text-decoration-color: #334155; }
  .coaching-log-expand-hint { font-size: var(--fs-xs); color: #475569; margin-left: 0.5rem; flex-shrink: 0; cursor: pointer; letter-spacing: 0.04em; font-weight: 600; text-transform: uppercase; white-space: nowrap; }
  .tip-label-personality { background: none !important; border: 1px solid; border-radius: 0.2rem; padding: 0.05rem 0.3rem; }
  .coaching-log-profile-section {
    display: flex; flex-direction: column; gap: 0.2rem;
    margin-top: 0.35rem; padding-top: 0.35rem;
    border-top: 1px solid #1e293b;
  }
  .coaching-log-profile-label {
    font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase;
    letter-spacing: 0.07em; color: #475569;
  }
  .coaching-log-profile-text { font-size: var(--fs-xs); color: #64748b; font-style: italic; line-height: 1.4; }
  .coaching-log-latest-positive { border-left-color: #166534 !important; background: #060e0a !important; }
  .coaching-log-positive { color: #4ade80 !important; }
  .tip-label-positive { color: #22c55e !important; }
  .transition-note {
    margin: 0.4rem 0.5rem 0; padding: 0.4rem 0.65rem;
    background: #071a0f; border-left: 3px solid #22c55e;
    border-radius: var(--radius-md); font-size: var(--fs-sm); color: #4ade80;
    flex-shrink: 0; animation: fadeInDown 0.3s ease-out;
  }
  .transition-from { color: #f59e0b; }
  .positive-streak {
    margin: 0.4rem 0.5rem 0; padding: 0.35rem 0.65rem;
    background: #060e0a; border-left: 3px solid #166534;
    border-radius: var(--radius-md); font-size: var(--fs-sm); color: #4ade80;
    flex-shrink: 0;
  }
  @keyframes fadeInDown {
    from { opacity: 0; transform: translateY(-4px); }
    to   { opacity: 1; transform: none; }
  }
  .sp-header {
    font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase;
    letter-spacing: 0.08em; color: #334155; padding: 0.3rem 0.75rem 0.1rem;
  }
  .sp-header-history { margin-top: 0.35rem; }

  .rapport-coaching { display: flex; flex-direction: column; gap: 0.2rem; padding: 0.35rem 0.5rem; background: #071a0f; border-left: 2px solid #14532d; border-radius: 0.25rem; margin-top: 0.35rem; }
  .rapport-coaching-label { font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase; letter-spacing: 0.07em; color: #4ade80; }
  .rapport-coaching-who { font-size: var(--fs-xs); color: #60a5fa; font-weight: 600; margin-top: 0.1rem; }
  .rapport-coaching-tip { display: flex; flex-direction: column; gap: 0.05rem; }
  .rapport-coaching-kw { font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: #4ade80; }
  .rapport-coaching-text { font-size: var(--fs-sm); color: #94a3b8; line-height: 1.4; }

  .interviewer-coaching {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    padding: 0.4rem 0.6rem;
    background: #150e00;
    border-top: 1px solid #2a1a00;
    flex-shrink: 0;
  }
  .coaching-emotion {
    font-size: var(--fs-xs);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: #f59e0b;
  }
  .coaching-note {
    font-size: var(--fs-sm);
    color: #fb923c;
    line-height: 1.4;
  }

  .col-right-body { gap: 0; overflow: hidden; padding: 0; }
  .right-panel { display: flex; flex-direction: column; min-height: 0; overflow: hidden; }
  .right-panel-top { border-bottom: none; }
  .right-panel-bottom { flex: 1; min-height: 0; }
  .right-panel-scroll { flex: 1; overflow-y: auto; min-height: 0; padding: 0.5rem 0.75rem 0.75rem; }
  .right-resize-handle {
    height: 5px; flex-shrink: 0; cursor: row-resize;
    background: #0f1e33; transition: background 0.15s;
  }
  .right-resize-handle:hover { background: #1e3a5f; }
  .right-sub-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 0.3rem 0.75rem; flex-shrink: 0;
    background: #080d18; border-bottom: 1px solid #0f1e33;
  }
  .right-sub-header:hover .zoom-btns { opacity: 1; }

  .side-stats {
    display: flex; flex-direction: column; gap: 0.1rem;
    padding: 0.5rem 0.25rem;
    border-top: 1px solid #1e293b; border-bottom: 1px solid #1e293b; flex-shrink: 0;
  }
  .side-stat {
    display: flex; align-items: center; justify-content: space-between;
    padding: 0.2rem 0.25rem; position: relative;
  }
  .side-label { font-size: var(--fs-xs); color: #475569; text-transform: uppercase; letter-spacing: 0.06em; font-weight: 600; }
  .side-value { font-size: var(--fs-sm); font-weight: 700; font-variant-numeric: tabular-nums; color: #475569; }

  .filler-block {
    display: flex; flex-direction: column; gap: 0.2rem;
    padding: 0.2rem 0.25rem;
  }
  .filler-active { font-variant-numeric: tabular-nums; }
  .filler-list {
    display: flex; flex-wrap: wrap; gap: 0.2rem 0.35rem;
    padding-left: 0.1rem;
  }
  .filler-tag {
    font-size: var(--fs-base); color: #92400e;
    background: #1c1006; border: 1px solid #78350f;
    border-radius: 0.25rem; padding: 0.1rem 0.5rem;
    white-space: nowrap;
  }
  .filler-tag strong { color: #fbbf24; font-weight: 800; font-size: 1.05em; }
  .side-ratelimits { flex: 1; overflow-y: auto; min-height: 0; }

  /* Focus overlay */
  .focus-overlay {
    position: fixed; inset: 0; background: rgba(0,0,0,0.93);
    z-index: 1000; display: flex; flex-direction: column;
    align-items: center; padding: 1.5rem 2rem 2rem; cursor: pointer;
    gap: 0.75rem; overflow-y: auto;
  }
  .focus-video-wrap {
    width: 100%; max-width: 680px; cursor: default;
    border-radius: 0.75rem; overflow: hidden;
    border: 1px solid #1a2d4a;
  }
  .focus-video { width: 100%; display: block; }
  .focus-crop-shell { width: 100%; }
  .focus-card {
    width: 100%; max-width: 680px; background: #07101e;
    border: 1px solid #1e3a5f; border-radius: 1rem; padding: 1.75rem 2rem;
    cursor: default; box-shadow: 0 0 60px rgba(59,130,246,0.08);
    display: flex; flex-direction: column; gap: 1rem;
  }
  .focus-question {
    color: #60a5fa; font-style: italic; font-size: 0.95rem; line-height: 1.5;
    padding-bottom: 1rem; border-bottom: 1px solid #1e293b;
  }
  .focus-section { display: flex; align-items: baseline; gap: 0.6rem; flex-wrap: wrap; }
  .focus-badge {
    font-size: 0.65rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.1em;
    padding: 0.15rem 0.5rem; border-radius: 0.25rem; flex-shrink: 0;
    background: rgba(100,116,139,0.2); color: #94a3b8; border: 1px solid #1e293b;
  }
  .focus-badge-ack { background: rgba(59,130,246,0.12); color: #60a5fa; border-color: rgba(59,130,246,0.25); }
  .focus-badge-ask { background: rgba(167,139,250,0.12); color: #a78bfa; border-color: rgba(167,139,250,0.25); }
  .focus-ack { color: #93c5fd; font-size: var(--fs-lg); line-height: 1.7; }
  .focus-tell { color: #e2e8f0; font-size: var(--fs-lg); line-height: 1.7; }
  :global(.focus-tell strong) { color: #fff; font-size: 1.2rem; font-weight: 800; }
  .focus-cues { display: flex; flex-direction: column; gap: 0.55rem; margin-top: 0.2rem; }
  .focus-cue { display: flex; align-items: baseline; gap: 0.55rem; }
  .focus-cue-label {
    font-size: 0.6rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.08em;
    color: #475569; flex-shrink: 0; padding-top: 0.15rem;
  }
  .focus-cue-text { color: #cbd5e1; font-size: 1rem; line-height: 1.6; }
  :global(.focus-cue-text strong) { color: #fff; font-size: 1.15rem; font-weight: 800; }
  .focus-asks { display: flex; flex-direction: column; gap: 0.3rem; }
  .focus-ask-item { color: #c4b5fd; font-size: 0.95rem; line-height: 1.5; }
  .focus-cursor { animation: blink 1s step-end infinite; color: #60a5fa; }
  @keyframes blink { 50% { opacity: 0; } }
  .focus-loading { color: #60a5fa; font-style: italic; }
  .focus-empty { color: #334155; font-style: italic; font-size: 1rem; text-align: center; padding: 3rem 0; }
  .focus-hint { margin-top: 0.75rem; font-size: var(--fs-sm); color: #1e293b; }

  /* Transcripts dropdown */
  .transcripts-wrapper { position: relative; }
  .header-btn {
    padding: 0.3rem 0.8rem; background: transparent;
    border: 1px solid #334155; border-radius: 0.375rem;
    color: #64748b; font-size: var(--fs-base); cursor: pointer; white-space: nowrap;
  }
  .header-btn:hover { border-color: #60a5fa; color: #60a5fa; }
  .transcripts-dropdown {
    position: absolute; top: 100%; right: 0; z-index: 200;
    background: #0f172a; border: 1px solid #1e293b; border-radius: 0.4rem;
    display: flex; flex-direction: column; min-width: 260px;
    max-height: 300px; overflow-y: auto; box-shadow: 0 4px 20px rgba(0,0,0,0.5);
  }
  .transcript-item {
    padding: 0.4rem 0.75rem; background: none; border: none; border-bottom: 1px solid #0f1e33;
    color: #94a3b8; font-size: var(--fs-sm); cursor: pointer; text-align: left;
  }
  .transcript-item:hover { background: #1e293b; color: #e2e8f0; }
  .transcript-empty { padding: 0.5rem 0.75rem; color: #334155; font-size: var(--fs-sm); font-style: italic; }

  /* Answer scoring in sentiment panel */
  .answer-score-panel {
    display: flex; flex-direction: column; gap: 0.3rem;
    margin-top: 0.5rem; padding: 0.5rem 0.6rem;
    background: #0d0d1a; border: 1px solid #1e1a2a;
    border-left: 3px solid #7c3aed; border-radius: 0.4rem;
  }
  .ascore-stat-btn { background: none; border: none; padding: 0.35rem 0.5rem; cursor: pointer; text-align: left; width: 100%; gap: 0.4rem; }
  .ascore-stat-btn:hover { background: #0d1525; }
  .ascore-row { display: flex; flex-wrap: wrap; gap: 0.3rem; align-items: center; background: none; border: none; padding: 0; cursor: pointer; width: 100%; text-align: left; }
  .ascore-badge {
    padding: 0.1rem 0.4rem; border-radius: 0.2rem;
    font-size: var(--fs-xs); font-weight: 800; text-transform: uppercase; letter-spacing: 0.05em;
  }
  .ascore-warn { background: #431407; color: #fb923c; }
  .ascore-coaching { margin: 0; font-size: var(--fs-sm); color: #fb923c; line-height: 1.5; }
  .you-delivery-row { display: flex; flex-direction: column; gap: 0; }
  .answer-nudge {
    display: flex; align-items: center; gap: 0.5rem;
    background: #0f1a2b; border: 1px solid #1e3a5f; border-left: 3px solid #3b82f6;
    border-radius: 0.4rem; padding: 0.45rem 0.65rem; margin-bottom: 0.4rem;
    animation: nudgefade 4s ease-out forwards;
  }
  .answer-nudge-icon { font-size: 1.1rem; flex-shrink: 0; }
  .answer-nudge-text { font-size: var(--fs-sm); color: #93c5fd; line-height: 1.3; }
  .answer-nudge-text strong { color: #e2e8f0; }
  @keyframes nudgefade {
    0%, 60% { opacity: 1; }
    100% { opacity: 0; }
  }
  .ascore-missed { display: flex; flex-wrap: wrap; align-items: center; gap: 0.25rem; }
  .ascore-missed-coaching { padding: 0.4rem 0.5rem 0; }
  .ascore-missed-label { font-size: var(--fs-xs); font-weight: 800; text-transform: uppercase; letter-spacing: 0.06em; color: #f87171; }
  .ascore-missed-kw { font-size: var(--fs-xs); padding: 0.05rem 0.3rem; border-radius: 0.2rem; background: #2a0a0a; color: #fca5a5; border: 1px solid #7f1d1d; }
  .ascore-vocal-row { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; background: none; border: none; padding: 0; cursor: pointer; width: 100%; text-align: left; }
  .ascore-vocal-row:hover .ascore-vocal-chevron { color: #64748b; }
  .ascore-vocal-score { font-size: var(--fs-sm); font-weight: 800; }
  .ascore-vocal-tone { font-size: var(--fs-sm); color: #94a3b8; }
  .ascore-vocal-fillers { font-size: var(--fs-xs); color: #f59e0b; }
  .ascore-vocal-chevron { font-size: var(--fs-xs); color: #334155; margin-left: auto; }

  /* Tone history at bottom of right panel */
  .tone-history-bottom { margin-top: 0.5rem; padding: 0.2rem 0.75rem; }

  /* Custom tooltip — rendered fixed above element, never clipped by overflow containers */
  :global(.app-tooltip) {
    position: fixed;
    z-index: 9999;
    background: #1e293b;
    color: #cbd5e1;
    font-size: 0.67rem;
    line-height: 1.45;
    padding: 0.3rem 0.55rem;
    border-radius: 0.3rem;
    border: 1px solid #334155;
    max-width: 220px;
    white-space: normal;
    word-break: break-word;
    pointer-events: none;
    opacity: 0;
    transition: opacity 0.1s;
    box-shadow: 0 4px 14px rgba(0,0,0,0.55);
  }
</style>
