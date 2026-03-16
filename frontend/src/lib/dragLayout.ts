interface SectionSlot { panel: string; id: string; }

export function applyDrop(
  layout: SectionSlot[],
  draggingId: string,
  targetId: string,
  targetPanel: string,
  pos: 'above' | 'below'
): SectionSlot[] {
  const next = [...layout];
  const fromIdx = next.findIndex(s => s.id === draggingId);
  if (fromIdx < 0) return layout;
  const [removed] = next.splice(fromIdx, 1);
  removed.panel = targetPanel;
  const toIdx = next.findIndex(s => s.id === targetId);
  if (toIdx < 0) { next.push(removed); return next; }
  next.splice(pos === 'above' ? toIdx : toIdx + 1, 0, removed);
  return next;
}

export function moveToPanel(layout: SectionSlot[], draggingId: string, panel: string): SectionSlot[] {
  const next = [...layout];
  const fromIdx = next.findIndex(s => s.id === draggingId);
  if (fromIdx < 0) return layout;
  const [removed] = next.splice(fromIdx, 1);
  removed.panel = panel;
  next.push(removed);
  return next;
}
