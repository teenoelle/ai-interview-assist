<script lang="ts">
  const { currentQuestion, suggestion, streaming } = $props<{
    currentQuestion: string;
    suggestion: string;
    streaming: boolean;
  }>();
</script>

<div class="suggestion-panel">
  <h3>AI Suggestions</h3>
  {#if currentQuestion}
    <div class="question-detected">
      <span class="badge">Question Detected</span>
      <p class="question-text">"{currentQuestion}"</p>
    </div>
  {/if}
  <div class="suggestion-content">
    {#if suggestion}
      <div class="suggestion-text">
        {suggestion}{#if streaming}<span class="cursor">|</span>{/if}
      </div>
    {:else if currentQuestion && streaming}
      <div class="loading">Generating suggestions<span class="dots">...</span></div>
    {:else}
      <p class="empty">
        Suggestions will appear when a question is detected in the conversation...
      </p>
    {/if}
  </div>
</div>

<style>
  .suggestion-panel {
    height: 100%;
    display: flex;
    flex-direction: column;
  }
  h3 {
    font-size: 1rem;
    font-weight: 600;
    color: #94a3b8;
    margin-bottom: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .question-detected {
    margin-bottom: 1rem;
    padding: 0.75rem;
    background: #1e293b;
    border-radius: 0.5rem;
  }
  .badge {
    display: inline-block;
    padding: 0.2rem 0.6rem;
    background: #1d4ed8;
    color: white;
    border-radius: 9999px;
    font-size: 0.7rem;
    font-weight: 600;
    margin-bottom: 0.5rem;
  }
  .question-text {
    color: #93c5fd;
    font-style: italic;
    font-size: 0.9rem;
  }
  .suggestion-content {
    flex: 1;
    overflow-y: auto;
  }
  .suggestion-text {
    color: #e2e8f0;
    line-height: 1.7;
    white-space: pre-wrap;
    font-size: 0.95rem;
  }
  .cursor {
    animation: blink 1s step-end infinite;
  }
  @keyframes blink {
    50% {
      opacity: 0;
    }
  }
  .loading {
    color: #60a5fa;
    font-style: italic;
  }
  .dots {
    display: inline-block;
  }
  .empty {
    color: #475569;
    font-style: italic;
    font-size: 0.85rem;
  }
</style>
