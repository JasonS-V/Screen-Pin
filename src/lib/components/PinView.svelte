<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { emitTo, listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { assetUrl, errorMessage, isTauri } from '$lib/desktop';
  import type { OcrLine, OcrPayload, PinRecord, SmartEntity } from '$lib/types';
  import { playClickSound } from '$lib/sound';
  import { i18n } from '$lib/i18n.svelte';

  let { pinId, toolbar = false }: { pinId: string; toolbar?: boolean } = $props();
  let lines = $state<OcrLine[]>([]);
  let entities = $state<SmartEntity[]>([]);
  let fullText = $state('');
  let ocrStatus = $state<'loading' | 'ready' | 'empty' | 'error'>('loading');
  let isGhostMode = $state(false);
  let copyTextFeedback = $state(false);
  let copyImageFeedback = $state(false);
  let copyFeedback = $state('');
  let copySuccessToast = $state('');
  let error = $state('');
  let busy = $state(false);
  let viewHeight = $state(0);
  let feedbackTimer: ReturnType<typeof setTimeout>;
  let imageFeedbackTimer: ReturnType<typeof setTimeout>;
  let toastTimer: ReturnType<typeof setTimeout>;
  const bankEntity = $derived(entities.find(e => e.kind === 'BankDigits'));

  // Estados para visibilidad animada de la barra de opciones
  let isNear = $state(false);
  let isHovered = $state(false);
  let isVisible = $state(false);
  let hideTimer: ReturnType<typeof setTimeout>;
  let interactiveTimer: ReturnType<typeof setTimeout>;

  function updateVisibility(show: boolean) {
    clearTimeout(hideTimer);
    clearTimeout(interactiveTimer);
    if (show) {
      isVisible = true;
      if (isTauri()) {
        void invoke('set_toolbar_interactive', { pinId, interactive: true });
      }
    } else {
      hideTimer = setTimeout(() => {
        isVisible = false;
        interactiveTimer = setTimeout(() => {
          if (!isVisible && isTauri()) {
            void invoke('set_toolbar_interactive', { pinId, interactive: false });
          }
        }, 260);
      }, 280);
    }
  }

  $effect(() => {
    if (!toolbar) return;
    const shouldBeVisible = isNear || isHovered;
    updateVisibility(shouldBeVisible);
  });

  function applyPayload(payload: OcrPayload) {
    lines = payload.lines;
    entities = payload.entities;
    fullText = payload.full_text;
    ocrStatus = fullText.trim() ? 'ready' : 'empty';
  }

  onMount(() => {
    if (!isTauri()) {
      if (!toolbar) {
        ocrStatus = 'error';
        error = i18n.t.pinOpenInDesktop;
      } else {
        isVisible = true;
      }
      return;
    }
    let disposed = false;
    const listeners: UnlistenFn[] = [];
    async function initialize() {
      try {
        for (const [name, handler] of [
          [`ocr:ready:${pinId}`, (event: { payload: OcrPayload }) => applyPayload(event.payload)],
          [`ocr:error:${pinId}`, () => { ocrStatus = 'error'; }],
          ['ghost:changed', (event: { payload: boolean }) => { isGhostMode = event.payload; }],
        ] as const) {
          const unlisten = await listen(name, handler as (event: { payload: unknown }) => void, { target: `${toolbar ? 'pin-toolbar' : 'pin'}-${pinId}` });
          if (disposed) { unlisten(); return; }
          listeners.push(unlisten);
        }

        if (toolbar) {
          const unlistenProximity = await listen<boolean>('toolbar:proximity', (event) => {
            isNear = event.payload;
          });
          if (disposed) { unlistenProximity(); return; }
          listeners.push(unlistenProximity);

          // Inicialmente la barra está oculta y no intercepta clics
          void invoke('set_toolbar_interactive', { pinId, interactive: false });
        }

        const snapshot = await invoke<{ pin: PinRecord; entities: SmartEntity[] }>('get_pin', { pinId });
        if (disposed || ocrStatus !== 'loading') return;
        const pin = snapshot.pin;
        if (pin.ocr_status === 'error') { ocrStatus = 'error'; return; }
        if (pin.ocr_status === 'pending') return;
        applyPayload({ pin_id: pinId, lines: pin.ocr_json ? JSON.parse(pin.ocr_json) : [], full_text: pin.raw_text, entities: snapshot.entities, is_sensitive: pin.is_sensitive });
      } catch (cause) { if (!disposed) { ocrStatus = 'error'; error = errorMessage(cause); } }
    }
    void initialize();
    return () => {
      disposed = true;
      listeners.forEach(fn => fn());
      clearTimeout(feedbackTimer);
      clearTimeout(imageFeedbackTimer);
      clearTimeout(toastTimer);
      clearTimeout(hideTimer);
      clearTimeout(interactiveTimer);
    };
  });

  async function archivePin() {
    if (busy) return;
    busy = true;
    error = '';
    playClickSound();
    try { await invoke('archive_pin_to_memory', { pinId }); }
    catch (cause) { error = errorMessage(cause); }
    finally { busy = false; }
  }

  function handleBackgroundDoubleClick(event: MouseEvent) {
    if ((event.target as HTMLElement).closest('.ocr-word, button') || window.getSelection()?.toString()) return;
    void archivePin();
  }

  async function copyPinImageToClipboard() {
    try {
      playClickSound();
      let success = false;
      if (isTauri()) {
        try {
          await invoke('copy_pin_image', { pinId });
          success = true;
        } catch (cause) {
          console.warn('Rust copy_pin_image fallo, probando navegador:', cause);
        }
      }
      if (!success) {
        const response = await fetch(assetUrl(pinId));
        const blob = await response.blob();
        await navigator.clipboard.write([
          new ClipboardItem({ 'image/png': blob })
        ]);
      }
      copyImageFeedback = true;
      copySuccessToast = i18n.t.imageCopied;
      clearTimeout(imageFeedbackTimer);
      clearTimeout(toastTimer);
      imageFeedbackTimer = setTimeout(() => { copyImageFeedback = false; }, 1600);
      toastTimer = setTimeout(() => { copySuccessToast = ''; }, 1600);
    } catch {
      error = i18n.t.copyImageError;
    }
  }

  async function copyAllText() {
    if (!fullText.trim()) return;
    try {
      playClickSound();
      await navigator.clipboard.writeText(fullText.trim());
      copyTextFeedback = true;
      copyFeedback = i18n.t.textCopied;
      copySuccessToast = i18n.t.textCopied;
      clearTimeout(feedbackTimer);
      clearTimeout(toastTimer);
      feedbackTimer = setTimeout(() => { copyTextFeedback = false; copyFeedback = ''; }, 1600);
      toastTimer = setTimeout(() => { copySuccessToast = ''; }, 1600);
    } catch {
      error = i18n.t.copyTextError;
    }
  }

  function handleCopy(event: ClipboardEvent) {
    const text = window.getSelection()?.toString();
    if (text) {
      if (event.clipboardData) {
        event.clipboardData.setData('text/plain', text);
        event.preventDefault();
      }
      return;
    }
    // Si no hay texto seleccionado, copiar la imagen al portapapeles con Ctrl+C
    void copyPinImageToClipboard();
  }

  async function copyQuickValue(text: string, label: string) {
    try {
      playClickSound();
      await navigator.clipboard.writeText(text);
      copyFeedback = `${label} ${i18n.current === 'es' ? 'copiado' : 'copied'}`;
      copySuccessToast = copyFeedback;
      copyTextFeedback = true;
      clearTimeout(feedbackTimer);
      clearTimeout(toastTimer);
      feedbackTimer = setTimeout(() => { copyTextFeedback = false; copyFeedback = ''; }, 1600);
      toastTimer = setTimeout(() => { copySuccessToast = ''; }, 1600);
    } catch { error = i18n.t.clipboardError; }
  }

  async function toggleGhostMode() {
    playClickSound();
    try {
      await invoke('toggle_ghost_mode', { pinId, enabled: !isGhostMode });
    } catch (cause) { error = errorMessage(cause); }
  }

  function handlePinMouseEnter() {
    if (isTauri()) {
      void emitTo(`pin-toolbar-${pinId}`, 'toolbar:proximity', true);
    }
  }

  function handlePinMouseLeave() {
    // Proximidad gestionada globalmente por monitor de Rust con márgenes de tolerancia
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.repeat) return;
    if (event.key === 'Escape') { event.preventDefault(); void archivePin(); }
    else if ((event.target as HTMLElement).closest('button, input, textarea')) return;
    else if (event.altKey && event.key.toLowerCase() === 'g') { event.preventDefault(); void toggleGhostMode(); }
    else if (event.key === 'Enter' && bankEntity?.kind === 'BankDigits') {
      event.preventDefault();
      void copyQuickValue(bankEntity.value.clean_digits, i18n.current === 'es' ? 'Número' : 'Number');
    }
  }
</script>

<svelte:head><title>{i18n.t.pinTitle}</title></svelte:head>
<svelte:window onkeydown={handleKeydown} oncopy={handleCopy} bind:innerHeight={viewHeight} />

{#if toolbar}
<div
  class="pin-toolbar-viewport"
  role="presentation"
  onmouseenter={() => { isHovered = true; }}
  onmouseleave={() => { isHovered = false; }}
>
  <main
    class="pin-toolbar"
    class:is-visible={isVisible}
    class:has-error={!!error}
    title={error || undefined}
    aria-label={i18n.t.pinActionsAria}
  >
    <button
      class="toolbar-btn"
      title={i18n.t.copyImageTitle}
      onclick={copyPinImageToClipboard}
    >
      {#if copyImageFeedback}
        <svg class="btn-icon" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polyline points="20 6 9 17 4 12"/></svg>
        <span>{i18n.t.imageCopied}</span>
      {:else}
        <svg class="btn-icon" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
        <span>{i18n.t.copyImage}</span>
      {/if}
    </button>
    <button
      class="toolbar-btn"
      disabled={!fullText.trim()}
      title={ocrStatus === 'loading' ? i18n.t.ocrLoading : ocrStatus === 'error' ? i18n.t.ocrError : ocrStatus === 'empty' ? i18n.t.ocrEmpty : i18n.t.copyAllText}
      onclick={copyAllText}
    >
      {#if copyTextFeedback}
        <svg class="btn-icon" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polyline points="20 6 9 17 4 12"/></svg>
        <span>{i18n.t.textCopied}</span>
      {:else}
        <svg class="btn-icon" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/></svg>
        <span>{i18n.t.copyText}</span>
      {/if}
    </button>
    <button
      class="toolbar-btn"
      class:active={isGhostMode}
      onclick={toggleGhostMode}
      title={i18n.t.ghostTitle}
      aria-pressed={isGhostMode}
    >
      <span>{i18n.t.ghost}</span>
    </button>
    <button
      class="toolbar-btn"
      onclick={archivePin}
      disabled={busy}
      title={i18n.t.archiveTitle}
    >
      <span>{i18n.t.archive}</span>
    </button>
    {#if error}<span class="sr-only" role="alert">{error}</span>{/if}
  </main>
</div>
{:else}
<main
  class="pin-chassis"
  class:is-ghost={isGhostMode}
  ondblclick={handleBackgroundDoubleClick}
  onmouseenter={handlePinMouseEnter}
  onmouseleave={handlePinMouseLeave}
  aria-label={i18n.t.floatingCropAria}
>
  <div
    class="calibration-corner"
    class:is-scanning={ocrStatus === 'loading'}
    data-tauri-drag-region
    title={i18n.t.dragWindowMove}
  ></div>

  <div
    class="window-drag-bar"
    data-tauri-drag-region
    title={i18n.t.dragWindowMove}
  ></div>

  <div
    class="image-stage"
    data-tauri-drag-region
    role="region"
    aria-label={i18n.t.capturedImageAria}
    title={i18n.t.dragCropTitle}
  >
    <img
      src={isTauri() ? assetUrl(pinId) : undefined}
      alt={i18n.t.capturedImageAria}
      draggable="false"
      onerror={() => { error = i18n.t.pinImageUnavailable; }}
    />

    {#if ocrStatus === 'ready'}
      <div class="ocr-layer" data-tauri-drag-region="false" aria-label={i18n.t.extractedTextAria}>
        {#each lines as line, lineIndex}
          {#each line.words as word, wordIndex}
            <span
              class="ocr-word"
              data-tauri-drag-region="false"
              style:left="{word.bounds.x * 100}%"
              style:top="{word.bounds.y * 100}%"
              style:width="{word.bounds.width * 100}%"
              style:height="{word.bounds.height * 100}%"
              style:font-size="{Math.max(1, word.bounds.height * viewHeight * 0.85)}px"
            >
              {word.text}{wordIndex < line.words.length - 1 ? ' ' : lineIndex < lines.length - 1 ? '\n' : ''}
            </span>
          {/each}
        {/each}
      </div>
    {/if}
  </div>

  {#if copySuccessToast}
    <div class="pin-toast" role="status">
      <svg class="btn-icon" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polyline points="20 6 9 17 4 12"/></svg>
      <span>{copySuccessToast}</span>
    </div>
  {/if}
  {#if error}<span class="pin-error" role="alert">{error}</span>{/if}
  {#if copyFeedback}<span class="sr-only" role="status">{copyFeedback}</span>{/if}
</main>
{/if}

<style>
  :global(:root) {
    --chassis-dark: #141413;
    --chassis-light: #F4F3EF;
    --accent-amber: #D95D1E;
    --accent-focus: #FF8A50;
  }

  .pin-chassis {
    position: relative;
    width: 100vw;
    height: 100vh;
    box-sizing: border-box;
    border-radius: 2px;
    border: 1px solid var(--chassis-light);
    outline: 1px solid var(--chassis-dark);
    outline-offset: -2px;
    overflow: hidden;
    user-select: none;
    transition: opacity 80ms cubic-bezier(0.2, 0, 0, 1);
    background: transparent;
  }

  .pin-chassis:focus-within {
    border-color: var(--accent-focus);
    outline: 2px solid var(--accent-focus);
  }

  .pin-chassis.is-ghost {
    opacity: 0.22;
  }

  .calibration-corner {
    position: absolute;
    top: 0;
    left: 0;
    width: 12px;
    height: 12px;
    border-top: 3px solid var(--accent-amber);
    border-left: 3px solid var(--accent-amber);
    z-index: 30;
    cursor: move;
  }

  .window-drag-bar {
    position: absolute;
    top: 0;
    left: 14px;
    right: 0;
    height: 8px;
    z-index: 29;
    cursor: move;
  }

  .calibration-corner.is-scanning {
    animation: pulse-amber 600ms infinite alternate;
  }

  @keyframes pulse-amber {
    from { border-color: var(--accent-amber); }
    to { border-color: var(--chassis-light); }
  }

  .image-stage {
    position: relative;
    width: 100%;
    height: 100%;
    cursor: move;
  }

  .image-stage:active {
    cursor: grabbing;
  }

  .image-stage img {
    width: 100%;
    height: 100%;
    object-fit: fill;
    display: block;
    pointer-events: none;
  }

  .ocr-layer {
    pointer-events: none;
    position: absolute;
    inset: 0;
    user-select: text;
  }

  .ocr-word {
    pointer-events: auto;
    position: absolute;
    color: transparent;
    font-family: 'JetBrains Mono', monospace;
    line-height: 1;
    white-space: pre;
    cursor: text;
    border-bottom: 1px solid transparent;
    transition: border-color 100ms ease;
  }

  .pin-chassis:hover .ocr-word,
  .pin-chassis:focus-within .ocr-word {
    border-bottom-color: rgba(217, 93, 30, 0.55);
  }

  .ocr-word::selection {
    background-color: rgba(255, 138, 80, 0.3);
    color: transparent;
  }

  .pin-error { position: absolute; bottom: 4px; left: 4px; right: 4px; z-index: 25; padding: 5px 7px; color: var(--accent-focus); background: var(--chassis-dark); font-size: 11px; overflow-wrap: anywhere; }

  .pin-toast {
    position: absolute;
    top: 8px;
    left: 50%;
    transform: translateX(-50%);
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 3px 8px;
    background: rgba(20, 20, 19, 0.92);
    border: 1px solid var(--accent-focus);
    color: var(--chassis-light);
    border-radius: 4px;
    font-size: 10px;
    font-weight: 600;
    z-index: 50;
    pointer-events: none;
    white-space: nowrap;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.4);
    animation: toast-pop 160ms ease-out;
  }

  @keyframes toast-pop {
    from { opacity: 0; transform: translate(-50%, -4px); }
    to { opacity: 1; transform: translate(-50%, 0); }
  }

  .pin-toolbar-viewport {
    width: 100vw;
    height: 100vh;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding: 4px 14px 26px 14px;
    box-sizing: border-box;
    overflow: visible;
    background: transparent;
  }

  .pin-toolbar {
    display: flex;
    align-items: center;
    gap: 4px;
    width: 100%;
    max-width: 412px;
    height: 38px;
    padding: 3px 4px;
    box-sizing: border-box;
    overflow: hidden;
    color: var(--chassis-light);
    background: #191918;
    border: 1px solid #383835;
    border-radius: 6px;
    box-shadow:
      0 2px 4px rgba(0, 0, 0, 0.08),
      0 6px 16px rgba(0, 0, 0, 0.12),
      0 12px 24px rgba(0, 0, 0, 0.14);

    /* Oculto por defecto y desplazado hacia arriba */
    opacity: 0;
    transform: translateY(-10px);
    pointer-events: none;

    /* Animación de retorno */
    transition:
      transform 220ms cubic-bezier(0.4, 0, 0.2, 1),
      opacity 180ms ease-out;
    will-change: transform, opacity;
  }

  .pin-toolbar.is-visible {
    /* Desplazamiento fluido hacia abajo al aparecer */
    opacity: 1;
    transform: translateY(0);
    pointer-events: auto;
    transition:
      transform 280ms cubic-bezier(0.16, 1, 0.3, 1),
      opacity 220ms cubic-bezier(0.16, 1, 0.3, 1);
  }

  .pin-toolbar.has-error {
    border-color: var(--accent-focus);
  }

  .toolbar-btn {
    flex: 1;
    min-width: 0;
    height: 30px;
    padding: 0 6px;
    border: 0;
    border-radius: 3px;
    background: var(--surface-secondary, #262624);
    color: var(--chassis-light);
    font-size: 11px;
    font-weight: 600;
    white-space: nowrap;
    cursor: pointer;
    transition: background 120ms ease;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    overflow: hidden;
    box-sizing: border-box;
  }

  .toolbar-btn span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  .btn-icon {
    flex-shrink: 0;
  }

  .toolbar-btn:hover:not(:disabled) {
    background: #393936;
  }

  .toolbar-btn.active {
    background: var(--accent-focus);
    color: var(--chassis-dark);
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
</style>
