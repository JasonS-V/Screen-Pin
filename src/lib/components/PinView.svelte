<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import type { OcrLine, SmartEntity } from '$lib/types';

  let { pinId }: { pinId: string } = $props();

  let lines = $state<OcrLine[]>([]);
  let entities = $state<SmartEntity[]>([]);
  let ocrStatus = $state<'loading' | 'ready' | 'empty' | 'error'>('loading');
  let isGhostMode = $state(false);
  let copyFeedback = $state<string | null>(null);

  const bankEntity = $derived(
    entities.find(
      (e): e is { kind: 'BankDigits'; value: { raw: string; clean_digits: string } } =>
        e.kind === 'BankDigits'
    )
  );

  const appWindow = getCurrentWindow();

  onMount(() => {
    const unlistenReady = listen<{ lines: OcrLine[]; entities: SmartEntity[] }>(
      `ocr:ready:${pinId}`,
      (event) => {
        lines = event.payload.lines;
        entities = event.payload.entities;
        ocrStatus = lines.length > 0 ? 'ready' : 'empty';
      }
    );

    const unlistenError = listen(`ocr:error:${pinId}`, () => {
      ocrStatus = 'error';
    });

    return () => {
      unlistenReady.then((fn) => fn());
      unlistenError.then((fn) => fn());
    };
  });

  async function archivePin() {
    try {
      await invoke('archive_pin_to_memory', { pinId });
    } catch {
      // Ignorar fallo de base de datos
    }
    await appWindow.close();
  }

  function handleBackgroundDoubleClick(event: MouseEvent) {
    const target = event.target as HTMLElement;
    if (target.classList.contains('ocr-word')) return;
    archivePin();
  }

  async function copyQuickValue(text: string, label: string) {
    await navigator.clipboard.writeText(text);
    copyFeedback = `${label} copiado`;
    setTimeout(() => {
      copyFeedback = null;
    }, 1400);
  }

  async function toggleGhostMode() {
    isGhostMode = !isGhostMode;
    try {
      await invoke('toggle_ghost_mode', { pinId, enabled: isGhostMode });
    } catch (e) {
      console.warn('Error alternando modo fantasma:', e);
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' || event.key === 'Backspace') {
      archivePin();
    } else if (event.key === 'Enter' && bankEntity) {
      copyQuickValue(bankEntity.value.clean_digits, 'Número');
    } else if (
      (event.altKey && event.key.toLowerCase() === 'g') ||
      (event.altKey && event.key.toLowerCase() === 'f')
    ) {
      toggleGhostMode();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<main
  class="pin-chassis"
  class:is-ghost={isGhostMode}
  ondblclick={handleBackgroundDoubleClick}
  aria-label="Recorte de pantalla flotante"
>
  <div
    class="calibration-corner"
    class:is-scanning={ocrStatus === 'loading'}
    data-tauri-drag-region
    title="Arrastrar recorte"
  ></div>

  <div class="image-stage" data-tauri-drag-region>
    <img
      src={`pinasset://localhost/${pinId}`}
      alt="Captura fijada en pantalla"
      draggable="false"
    />

    {#if ocrStatus === 'ready'}
      <div class="ocr-layer" aria-label="Texto extraído de la imagen">
        {#each lines as line}
          {#each line.words as word}
            <span
              class="ocr-word"
              style:left="{word.bounds.x * 100}%"
              style:top="{word.bounds.y * 100}%"
              style:width="{word.bounds.width * 100}%"
              style:height="{word.bounds.height * 100}%"
            >
              {word.text}{" "}
            </span>
          {/each}
        {/each}
      </div>
    {/if}
  </div>

  <footer class="pin-hud">
    {#if copyFeedback}
      <span class="status-msg" role="status">{copyFeedback}</span>
    {:else if ocrStatus === 'loading'}
      <span class="status-msg">Procesando texto...</span>
    {:else if ocrStatus === 'error'}
      <span class="status-msg error">Contraste bajo para lectura</span>
    {:else if ocrStatus === 'empty'}
      <span class="status-msg">Sin texto detectable</span>
    {:else if bankEntity}
      <button
        type="button"
        class="hud-btn primary-extract"
        onclick={() => copyQuickValue(bankEntity.value.clean_digits, 'Número limpio')}
      >
        Copiar {bankEntity.value.clean_digits}
      </button>
    {/if}

    <div class="hud-actions">
      <button
        type="button"
        class="hud-btn"
        class:active={isGhostMode}
        onclick={toggleGhostMode}
        title="Modo Fantasma (Alt+G)"
      >
        Fantasma
      </button>

      <button
        type="button"
        class="hud-btn close-btn"
        onclick={archivePin}
        aria-label="Archivar en memoria y cerrar"
      >
        Archivar
      </button>
    </div>
  </footer>
</main>

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
    width: 10px;
    height: 10px;
    border-top: 3px solid var(--accent-amber);
    border-left: 3px solid var(--accent-amber);
    z-index: 30;
    cursor: grab;
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
    cursor: grab;
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
    position: absolute;
    inset: 0;
    user-select: text;
  }

  .ocr-word {
    position: absolute;
    color: transparent;
    font-family: 'JetBrains Mono', monospace;
    line-height: 1;
    white-space: nowrap;
    cursor: text;
    border-bottom: 1px solid transparent;
    transition: border-color 100ms ease;
  }

  .pin-chassis:hover .ocr-word,
  .pin-chassis:focus-within .ocr-word {
    border-bottom-color: rgba(217, 93, 30, 0.55);
  }

  .ocr-word::selection {
    background-color: rgba(20, 20, 19, 0.90);
    color: var(--chassis-light);
  }

  .pin-hud {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    min-height: 28px;
    background-color: var(--chassis-dark);
    color: var(--chassis-light);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 2px 6px;
    gap: 6px;
    opacity: 0;
    pointer-events: none;
    transition: opacity 100ms ease;
    z-index: 25;
  }

  .pin-chassis:hover .pin-hud,
  .pin-chassis:focus-within .pin-hud {
    opacity: 1;
    pointer-events: auto;
  }

  .status-msg {
    font-family: 'JetBrains Mono', monospace;
    font-size: 10px;
    color: #A3A19A;
  }

  .status-msg.error {
    color: var(--accent-amber);
  }

  .hud-actions {
    display: flex;
    gap: 4px;
    align-items: center;
  }

  .hud-btn {
    background: #262624;
    color: var(--chassis-light);
    border: 1px solid #4A4945;
    border-radius: 2px;
    padding: 2px 8px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    cursor: pointer;
  }

  .hud-btn.active {
    background: var(--accent-amber);
    color: var(--chassis-light);
    border-color: var(--accent-amber);
  }

  .hud-btn.primary-extract {
    border-color: var(--accent-amber);
  }

  .hud-btn:focus-visible {
    outline: 2px solid var(--accent-focus);
    outline-offset: 1px;
  }
</style>
