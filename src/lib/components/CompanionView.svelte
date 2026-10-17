<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { isTauri, errorMessage } from '$lib/desktop';
  import { isEnabled as isAutostartEnabled, enable as enableAutostart, disable as disableAutostart } from '@tauri-apps/plugin-autostart';
  import { isSoundEnabled, setSoundEnabled, playClickSound } from '$lib/sound';

  const desktop = isTauri();
  let busy = $state(false);
  let error = $state('');
  let successNotice = $state('');
  let shortcutWarnings = $state<string[]>([]);
  let activeTab = $state<'shortcuts' | 'settings'>('shortcuts');

  let autostart = $state(false);
  let autostartLoading = $state(false);
  let soundActive = $state(true);

  onMount(() => {
    soundActive = isSoundEnabled();
    const handleCustomSoundChange = (e: Event) => {
      soundActive = (e as CustomEvent<boolean>).detail;
    };
    window.addEventListener('screen-pin:sound-changed', handleCustomSoundChange);

    if (desktop) {
      invoke<string[]>('get_shortcut_warnings').then(value => { shortcutWarnings = value; }).catch(cause => { error = errorMessage(cause); });
      const unlisten = listen<string>('app:error', event => { error = event.payload; });

      isAutostartEnabled().then(val => {
        autostart = val;
      }).catch(cause => {
        console.warn('Error comprobando estado de autostart:', cause);
      });

      return () => {
        void unlisten.then(fn => fn());
        window.removeEventListener('screen-pin:sound-changed', handleCustomSoundChange);
      };
    }

    return () => {
      window.removeEventListener('screen-pin:sound-changed', handleCustomSoundChange);
    };
  });

  async function toggleAutostart() {
    if (!desktop || autostartLoading) return;
    autostartLoading = true;
    error = '';
    successNotice = '';
    try {
      if (autostart) {
        await disableAutostart();
        autostart = false;
        successNotice = 'Arranque automático desactivado';
      } else {
        await enableAutostart();
        autostart = true;
        successNotice = 'Arranque con Windows activado';
      }
      setTimeout(() => { successNotice = ''; }, 3000);
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      autostartLoading = false;
    }
  }

  function toggleSound() {
    soundActive = !soundActive;
    setSoundEnabled(soundActive);
    if (soundActive) {
      playClickSound();
    }
  }

  async function run(command: string) {
    if (busy || !desktop) return;
    busy = true;
    error = '';
    try { await invoke(command); }
    catch (cause) { error = errorMessage(cause); }
    finally { busy = false; }
  }
</script>

<svelte:head><title>Screen-Pin · Atajos y configuración</title></svelte:head>

<main class="companion-shell">
  <header class="brand-row">
    <div class="brand"><img src="/favicon.svg" width="28" height="28" alt="" /><span>Screen-Pin</span></div>
    <div class="segmented-control" role="tablist" aria-label="Secciones de Screen-Pin">
      <button
        class="segmented-btn"
        class:active={activeTab === 'shortcuts'}
        role="tab"
        aria-selected={activeTab === 'shortcuts'}
        onclick={() => activeTab = 'shortcuts'}
      >
        Atajos
      </button>
      <button
        class="segmented-btn"
        class:active={activeTab === 'settings'}
        role="tab"
        aria-selected={activeTab === 'settings'}
        onclick={() => activeTab = 'settings'}
      >
        Configuración
      </button>
    </div>
  </header>

  <div class="content-area">
    {#if activeTab === 'shortcuts'}
      <section class="tab-content" aria-labelledby="shortcuts-title">
        <h1 id="shortcuts-title">Atajos de teclado</h1>
        <p class="section-intro">Úsalos desde cualquier ventana de tu equipo.</p>
        <div class="shortcut-row">
          <div><strong>Nuevo recorte</strong>{#if shortcutWarnings.includes('capture')}<span class="shortcut-warning" role="status">Atajo ocupado · usa el botón</span>{/if}</div>
          <kbd>Alt + Shift + S</kbd>
        </div>
        <div class="shortcut-row">
          <div><strong>Abrir memoria</strong>{#if shortcutWarnings.includes('memory')}<span class="shortcut-warning" role="status">Atajo ocupado · usa el botón o la bandeja</span>{/if}</div>
          <kbd>Alt + Shift + M</kbd>
        </div>
        <div class="shortcut-row">
          <div><strong>Recuperar clics</strong>{#if shortcutWarnings.includes('restore')}<span class="shortcut-warning" role="status">Atajo ocupado · abre Memoria o usa la bandeja</span>{/if}</div>
          <kbd>Alt + Shift + G</kbd>
        </div>
      </section>
    {:else}
      <section class="tab-content" aria-labelledby="settings-title">
        <h1 id="settings-title">Configuración</h1>
        <p class="section-intro">Preferencias de sistema y experiencia.</p>
        
        <div class="setting-row">
          <div class="setting-text">
            <strong>Arrancar con Windows</strong>
            <span class="setting-desc">Inicia Screen-Pin minimizado en la bandeja al encender el equipo.</span>
          </div>
          <button
            type="button"
            role="switch"
            class="switch-control"
            class:checked={autostart}
            disabled={!desktop || autostartLoading}
            aria-checked={autostart}
            aria-label="Arrancar Screen-Pin con Windows"
            onclick={toggleAutostart}
          >
            <span class="switch-thumb"></span>
          </button>
        </div>

        <div class="setting-row">
          <div class="setting-text">
            <strong>Efectos de sonido (Ficha)</strong>
            <span class="setting-desc">Reproduce un sonido de ficha táctil al pulsar opciones y acciones.</span>
          </div>
          <div class="setting-actions">
            {#if soundActive}
              <button
                type="button"
                class="test-sound-btn"
                title="Probar sonido de ficha"
                onclick={() => playClickSound()}
              >
                Probar
              </button>
            {/if}
            <button
              type="button"
              role="switch"
              class="switch-control"
              class:checked={soundActive}
              aria-checked={soundActive}
              aria-label="Efectos de sonido táctiles"
              onclick={toggleSound}
            >
              <span class="switch-thumb"></span>
            </button>
          </div>
        </div>
      </section>
    {/if}
  </div>

  {#if successNotice}<p class="notice success" role="status">{successNotice}</p>{/if}
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if !desktop}<p class="notice">Vista previa: las capturas, atajos y arranque automático requieren la aplicación de escritorio.</p>{/if}

  <footer class="companion-footer">
    <button class="button primary" disabled={!desktop || busy} onclick={() => run('trigger_capture')}>
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" aria-hidden="true"><path d="M8 3H3V8M16 3H21V8M21 16V21H16M8 21H3V16M8 12H16M12 8V16" /></svg>
      {busy ? 'Un momento…' : 'Nuevo recorte'}
    </button>
    {#if desktop}
      <button class="button memory-link" disabled={busy} onclick={() => run('open_memory_drawer')}>Abrir memoria</button>
    {:else}
      <a class="button memory-link" href="/memory">Ver memoria</a>
    {/if}
  </footer>
</main>

<style>
  .companion-shell {
    height: 100dvh;
    padding: 20px 24px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--chassis-dark);
    color: var(--chassis-light);
  }
  .brand-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-bottom: 12px;
    border-bottom: 1px solid var(--border-color);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    font-weight: 650;
    letter-spacing: -.3px;
  }
  .segmented-control {
    display: flex;
    background: #1b1b19;
    padding: 2px;
    border-radius: 6px;
    border: 1px solid var(--border-color);
    gap: 2px;
  }
  .segmented-btn {
    padding: 5px 12px;
    font-size: 11px;
    font-weight: 500;
    background: transparent;
    border: none;
    border-radius: 4px;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 120ms ease;
  }
  .segmented-btn.active {
    background: var(--surface-secondary);
    color: var(--chassis-light);
    font-weight: 600;
  }
  .content-area {
    flex: 1;
    overflow-y: auto;
    padding: 16px 0;
  }
  .tab-content {
    display: flex;
    flex-direction: column;
  }
  h1 {
    font-size: 21px;
    line-height: 1.2;
    font-weight: 600;
    letter-spacing: -.5px;
  }
  .section-intro {
    color: var(--text-secondary);
    font-size: 12px;
    margin: 4px 0 14px;
  }
  .shortcut-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    min-height: 48px;
    padding: 8px 0;
    border-top: 1px solid var(--border-color);
  }
  .shortcut-row strong {
    display: block;
    font-size: 13px;
    font-weight: 500;
  }
  .shortcut-row kbd {
    flex-shrink: 0;
    font-size: 11px;
    padding: 4px 8px;
  }
  .shortcut-warning {
    display: block;
    margin-top: 4px;
    color: var(--accent-focus);
    font-size: 11px;
  }
  .setting-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 12px 0;
    border-top: 1px solid var(--border-color);
  }
  .setting-text {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .setting-text strong {
    font-size: 13px;
    font-weight: 500;
  }
  .setting-desc {
    color: var(--text-secondary);
    font-size: 11px;
    line-height: 1.4;
  }
  .setting-actions {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-shrink: 0;
  }
  .test-sound-btn {
    background: #2a2a28;
    color: var(--text-secondary);
    border: 1px solid var(--border-strong);
    padding: 4px 8px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 600;
    cursor: pointer;
    transition: all 120ms ease;
  }
  .test-sound-btn:hover {
    color: var(--chassis-light);
    border-color: var(--accent-focus);
  }
  .switch-control {
    position: relative;
    width: 42px;
    height: 24px;
    border-radius: 12px;
    background: #2b2b29;
    border: 1px solid var(--border-strong);
    cursor: pointer;
    transition: background 150ms ease, border-color 150ms ease;
    padding: 0;
    display: inline-flex;
    align-items: center;
    flex-shrink: 0;
  }
  .switch-control.checked {
    background: var(--accent-focus);
    border-color: var(--accent-focus);
  }
  .switch-thumb {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #ffffff;
    transform: translateX(2px);
    transition: transform 150ms cubic-bezier(0.4, 0, 0.2, 1), background 150ms ease;
  }
  .switch-control.checked .switch-thumb {
    transform: translateX(20px);
    background: var(--chassis-dark);
  }
  .notice.success {
    color: #a8f5b8;
    border-color: #2b6e3f;
    background: #14281c;
    padding: 8px 12px;
    font-size: 11px;
    margin-top: 8px;
  }
  .companion-footer {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: auto;
    padding-top: 14px;
    border-top: 1px solid var(--border-color);
  }
  .companion-footer .button {
    flex: 1;
    min-height: 40px;
    text-decoration: none;
  }
  @media (max-width: 480px) {
    .companion-shell { padding: 16px; }
    .shortcut-row, .setting-row { gap: 8px; }
    .shortcut-row kbd { font-size: 10px; }
  }
</style>
