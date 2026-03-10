<script lang="ts">
  import type { Snippet } from 'svelte';

  const { id, title, children, isDragging, isDragOver, onDragStart, onDragOver, onDrop, onDragEnd } = $props<{
    id: string;
    title: string;
    children: Snippet;
    isDragging: boolean;
    isDragOver: boolean;
    onDragStart: (id: string) => void;
    onDragOver: (id: string) => void;
    onDrop: (id: string) => void;
    onDragEnd: () => void;
  }>();
</script>

<div
  class="draggable-panel"
  class:is-dragging={isDragging}
  class:drag-over={isDragOver}
  ondragover={(e) => { e.preventDefault(); onDragOver(id); }}
  ondrop={(e) => { e.preventDefault(); onDrop(id); }}
  role="none"
>
  <div
    class="drag-handle"
    draggable="true"
    ondragstart={() => onDragStart(id)}
    ondragend={onDragEnd}
    title="Drag to reposition"
    role="none"
  >
    <span class="grip">⠿⠿</span>
    <span class="panel-title">{title}</span>
  </div>
  <div class="panel-body">
    {@render children()}
  </div>
</div>

<style>
  .draggable-panel {
    display: flex;
    flex-direction: column;
    background: #0f172a;
    border: 1px solid #1e293b;
    border-radius: 0.5rem;
    overflow: hidden;
    height: 100%;
    box-sizing: border-box;
    transition: border-color 0.15s, opacity 0.15s;
  }
  .draggable-panel.is-dragging {
    opacity: 0.4;
  }
  .draggable-panel.drag-over {
    border-color: #3b82f6;
    box-shadow: 0 0 0 2px #1d4ed830;
  }
  .drag-handle {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0.75rem;
    background: #0d1526;
    border-bottom: 1px solid #1e293b;
    cursor: grab;
    user-select: none;
    flex-shrink: 0;
  }
  .drag-handle:active {
    cursor: grabbing;
  }
  .grip {
    color: #334155;
    font-size: 0.85rem;
    letter-spacing: -2px;
    line-height: 1;
  }
  .panel-title {
    font-size: 0.7rem;
    font-weight: 600;
    color: #64748b;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .panel-body {
    flex: 1;
    overflow: hidden;
    padding: 1rem;
    display: flex;
    flex-direction: column;
  }
</style>
