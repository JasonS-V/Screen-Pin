<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import type { PinRecord } from '$lib/types';

  let pins = $state<PinRecord[]>([]);
  let searchQuery = $state('');
  let isLoading = $state(true);
  let copyFeedback = $state<string | null>(null);

  onMount(async () => {
    await loadPins();
  });

  async function loadPins() {
    isLoading = true;
    try {
      pins = await invoke<PinRecord[]>('get_memory_pins');
    } catch (e) {
      console.error('Error al cargar memoria:', e);
    } finally {
      isLoading = false;
    }
  }

  async function handleSearch() {
    try {
      if (searchQuery.trim().length === 0) {
        pins = await invoke<PinRecord[]>('get_memory_pins');
      } else {
        pins = await invoke<PinRecord[]>('search_memory_pins', { query: searchQuery });
      }
    } catch (e) {
      console.error('Error en búsqueda FTS5:', e);
    }
  }

  async function copyText(text: string, id: string) {
    await navigator.clipboard.writeText(text);
    copyFeedback = id;
    setTimeout(() => {
      copyFeedback = null;
    }, 1200);
  }

  async function repin(pinId: string) {
    try {
      await invoke('repin_from_memory', { pinId });
    } catch (e) {
      console.error('Error al re-anclar pin:', e);
    }
  }

  async function toggleRetention(pin: PinRecord) {
    try {
      const isForever = await invoke<boolean>('toggle_retention', { pinId: pin.id });
      pin.is_pinned_forever = isForever;
    } catch (e) {
      console.error('Error al alternar retención:', e);
    }
  }

  async function triggerCapture() {
    try {
      await invoke('trigger_capture');
    } catch (e) {
      console.error('Error al disparar captura:', e);
    }
  }

  function formatTime(timestamp: number): string {
    const elapsed = Math.floor(Date.now() / 1000 - timestamp);
    if (elapsed < 60) return `${elapsed}s`;
    if (elapsed < 3600) return `${Math.floor(elapsed / 60)}m`;
    if (elapsed < 86400) return `${Math.floor(elapsed / 3600)}h`;
    return `${Math.floor(elapsed / 86400)}d`;
  }

  function formatPurgeTime(expiresAt: number | null): string {
    if (!expiresAt) return '';
    const remaining = Math.max(0, Math.floor(expiresAt - Date.now() / 1000));
    const mins = Math.floor(remaining / 60);
    return mins > 0 ? `Purga en ${mins}m` : 'Purga inminente';
  }
</script>

<main class="memory-strip" aria-label="Cajón de Memoria Screen-Pin">
  <header class="memory-header">
    <div class="search-box">
      <svg class="search-icon" viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
        <circle cx="6.5" cy="6.5" r="4.5" fill="none" stroke="currentColor" stroke-width="1.5" />
        <line x1="10" y1="10" x2="14" y2="14" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
      </svg>
      <input
        type="search"
        bind:value={searchQuery}
        oninput={handleSearch}
        placeholder="Buscar texto, cuenta, clave..."
        class="search-input"
        aria-label="Buscar en recortes históricos"
      />
    </div>
  </header>

  <section class="pins-feed" aria-live="polite">
    {#if isLoading}
      <div class="state-msg">
        <span class="pulse-indicator"></span>
        <span>Buscando en recortes locales...</span>
      </div>
    {:else if pins.length === 0}
      <div class="empty-state">
        <p class="empty-title">No hay recortes guardados</p>
        <p class="empty-desc">
          Presiona <kbd>Alt</kbd> + <kbd>Shift</kbd> + <kbd>S</kbd> para fijar una zona de tu pantalla.
        </p>
        <button type="button" class="btn primary-btn" onclick={triggerCapture}>
          Probar recorte ahora
        </button>
      </div>
    {:else}
      <ul class="cards-list">
        {#each pins as pin (pin.id)}
          <li class="memory-card" class:is-sensitive={pin.is_sensitive}>
            {#if pin.entity_type === 'BankDigits' || pin.entity_type === 'WifiCredential'}
              <div class="compact-data-card">
                <div class="card-main">
                  <div class="card-meta">
                    <span class="meta-tag">{pin.entity_type === 'BankDigits' ? 'Cuenta / Tarjeta' : 'Credencial'}</span>
                    <span class="time-tag">{formatTime(pin.created_at)}</span>
                    {#if pin.is_sensitive}
                      <span class="privacy-badge">
                        {pin.is_pinned_forever ? 'Conservado' : formatPurgeTime(pin.expires_at)}
                      </span>
                    {/if}
                  </div>
                  <div class="data-value">{pin.raw_text.trim()}</div>
                  <div class="card-actions">
                    <button
                      type="button"
                      class="card-btn"
                      onclick={() => copyText(pin.raw_text.trim(), pin.id)}
                    >
                      {copyFeedback === pin.id ? '¡Copiado!' : 'Copiar'}
                    </button>
                    <button
                      type="button"
                      class="card-btn"
                      onclick={() => repin(pin.id)}
                    >
                      Re-anclar
                    </button>
                    {#if pin.is_sensitive}
                      <button
                        type="button"
                        class="card-btn retention-btn"
                        class:active={pin.is_pinned_forever}
                        onclick={() => toggleRetention(pin)}
                      >
                        {pin.is_pinned_forever ? 'Fijado' : 'Fijar'}
                      </button>
                    {/if}
                  </div>
                </div>
                <div class="thumbnail-col">
                  <img
                    src={`pinasset://localhost/${pin.id}`}
                    alt="Miniatura"
                    class="thumb-img"
                    loading="lazy"
                  />
                </div>
              </div>
            {:else}
              <div class="visual-preview-card">
                <div class="preview-header">
                  <span class="dim-info">{Math.round(pin.width)} × {Math.round(pin.height)} px</span>
                  <span class="time-tag">{formatTime(pin.created_at)}</span>
                </div>
                <div class="preview-media" onclick={() => repin(pin.id)} role="button" tabindex="0" onkeydown={(e) => e.key === 'Enter' && repin(pin.id)}>
                  <img
                    src={`pinasset://localhost/${pin.id}`}
                    alt="Recorte histórico"
                    class="preview-img"
                    loading="lazy"
                  />
                </div>
                {#if pin.raw_text.trim()}
                  <p class="extracted-snippet">{pin.raw_text.trim()}</p>
                {/if}
                <div class="card-actions">
                  <button
                    type="button"
                    class="card-btn"
                    onclick={() => repin(pin.id)}
                  >
                    Re-anclar en ({Math.round(pin.pos_x)}, {Math.round(pin.pos_y)})
                  </button>
                  {#if pin.raw_text.trim()}
                    <button
                      type="button"
                      class="card-btn"
                      onclick={() => copyText(pin.raw_text.trim(), pin.id)}
                    >
                      {copyFeedback === pin.id ? '¡Copiado!' : 'Copiar texto'}
                    </button>
                  {/if}
                </div>
              </div>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  </section>
</main>

<style>
  .memory-strip {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background-color: #141413;
    color: #F4F3EF;
    box-sizing: border-box;
    user-select: none;
    overflow: hidden;
  }

  .memory-header {
    padding: 10px 12px;
    border-bottom: 1px solid #262624;
    background: #141413;
  }

  .search-box {
    display: flex;
    align-items: center;
    background: #262624;
    border: 1px solid #4A4945;
    border-radius: 2px;
    padding: 4px 8px;
    gap: 8px;
  }

  .search-box:focus-within {
    border-color: #FF8A50;
    outline: 1px solid #FF8A50;
  }

  .search-icon {
    color: #A3A19A;
    flex-shrink: 0;
  }

  .search-input {
    background: transparent;
    border: none;
    color: #F4F3EF;
    font-family: inherit;
    font-size: 12px;
    width: 100%;
    outline: none;
  }

  .search-input::placeholder {
    color: #A3A19A;
  }

  .pins-feed {
    flex: 1;
    overflow-y: auto;
    padding: 10px 12px;
  }

  .pins-feed::-webkit-scrollbar {
    width: 4px;
  }
  .pins-feed::-webkit-scrollbar-thumb {
    background: #262624;
    border-radius: 2px;
  }

  .state-msg {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 24px 0;
    justify-content: center;
    font-size: 12px;
    color: #A3A19A;
  }

  .pulse-indicator {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #D95D1E;
    animation: pulse 1s infinite alternate;
  }

  @keyframes pulse {
    from { opacity: 0.3; }
    to { opacity: 1; }
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 40px 16px;
    text-align: center;
    gap: 12px;
  }

  .empty-title {
    font-size: 14px;
    font-weight: 600;
  }

  .empty-desc {
    font-size: 12px;
    color: #A3A19A;
    line-height: 1.5;
  }

  kbd {
    background: #262624;
    border: 1px solid #4A4945;
    border-radius: 2px;
    padding: 2px 5px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    color: #FF8A50;
  }

  .btn.primary-btn {
    background: #262624;
    color: #F4F3EF;
    border: 1px solid #D95D1E;
    border-radius: 2px;
    padding: 6px 14px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    cursor: pointer;
  }

  .btn.primary-btn:hover {
    border-color: #FF8A50;
  }

  .cards-list {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .memory-card {
    background: #262624;
    border: 1px solid #4A4945;
    border-radius: 2px;
    padding: 8px 10px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .memory-card.is-sensitive {
    border-left: 3px solid #D95D1E;
  }

  .compact-data-card {
    display: flex;
    justify-content: space-between;
    gap: 10px;
  }

  .card-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }

  .card-meta {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 10px;
  }

  .meta-tag {
    color: #D95D1E;
    font-weight: 600;
  }

  .time-tag {
    color: #A3A19A;
  }

  .privacy-badge {
    background: #141413;
    border: 1px solid #4A4945;
    color: #FF8A50;
    padding: 1px 4px;
    border-radius: 2px;
    font-family: 'JetBrains Mono', monospace;
  }

  .data-value {
    font-family: 'JetBrains Mono', monospace;
    font-size: 13px;
    color: #F4F3EF;
    word-break: break-all;
  }

  .thumbnail-col {
    width: 52px;
    height: 52px;
    flex-shrink: 0;
    border: 1px solid #4A4945;
    border-radius: 2px;
    overflow: hidden;
    background: #141413;
  }

  .thumb-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .visual-preview-card {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .preview-header {
    display: flex;
    justify-content: space-between;
    font-size: 10px;
    color: #A3A19A;
    font-family: 'JetBrains Mono', monospace;
  }

  .preview-media {
    max-height: 120px;
    border: 1px solid #4A4945;
    border-radius: 2px;
    overflow: hidden;
    cursor: pointer;
    background: #141413;
  }

  .preview-img {
    width: 100%;
    height: auto;
    max-height: 120px;
    object-fit: contain;
    display: block;
  }

  .extracted-snippet {
    font-size: 11px;
    color: #A3A19A;
    line-height: 1.3;
    max-height: 32px;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .card-actions {
    display: flex;
    gap: 6px;
    margin-top: 2px;
  }

  .card-btn {
    background: #141413;
    color: #F4F3EF;
    border: 1px solid #4A4945;
    border-radius: 2px;
    padding: 3px 8px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 10px;
    cursor: pointer;
  }

  .card-btn:hover {
    border-color: #FF8A50;
  }

  .card-btn.retention-btn.active {
    background: #D95D1E;
    border-color: #D95D1E;
    color: #F4F3EF;
  }
</style>
