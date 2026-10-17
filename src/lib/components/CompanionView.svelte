<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { isTauri, isMac, errorMessage } from '$lib/desktop';
  import { isEnabled as isAutostartEnabled, enable as enableAutostart, disable as disableAutostart } from '@tauri-apps/plugin-autostart';
  import { isSoundEnabled, setSoundEnabled, playClickSound } from '$lib/sound';
  import { i18n } from '$lib/i18n.svelte';

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
        successNotice = i18n.t.autostartDisabledNotice;
      } else {
        await enableAutostart();
        autostart = true;
        successNotice = i18n.t.autostartEnabledNotice;
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

<svelte:head><title>{i18n.t.companionTitle}</title></svelte:head>

<main class="companion-shell">
  <header class="brand-row">
    <div class="brand"><img src="/favicon.svg" width="28" height="28" alt="" /><span>{i18n.t.appName}</span></div>
    <div class="header-actions">
      <div class="lang-switch-pill" role="group" aria-label={i18n.t.languageLabel}>
        <button
          type="button"
          class="lang-pill-btn"
          class:active={i18n.current === 'en'}
          title="Switch to English"
          onclick={() => i18n.set('en')}
        >EN</button>
        <span class="lang-divider">/</span>
        <button
          type="button"
          class="lang-pill-btn"
          class:active={i18n.current === 'es'}
          title="Cambiar a Español"
          onclick={() => i18n.set('es')}
        >ES</button>
      </div>
      <div class="segmented-control" role="tablist" aria-label={i18n.t.shortcutsTab}>
        <button
          class="segmented-btn"
          class:active={activeTab === 'shortcuts'}
          role="tab"
          aria-selected={activeTab === 'shortcuts'}
          onclick={() => activeTab = 'shortcuts'}
        >
          {i18n.t.shortcutsTab}
        </button>
        <button
          class="segmented-btn"
          class:active={activeTab === 'settings'}
          role="tab"
          aria-selected={activeTab === 'settings'}
          onclick={() => activeTab = 'settings'}
        >
          {i18n.t.settingsTab}
        </button>
      </div>
    </div>
  </header>

  <div class="content-area">
    {#if activeTab === 'shortcuts'}
      <section class="tab-content" aria-labelledby="shortcuts-title">
        <h1 id="shortcuts-title">{i18n.t.keyboardShortcutsHeading}</h1>
        <p class="section-intro">{i18n.t.shortcutsIntro}</p>
        <div class="shortcut-row">
          <div><strong>{i18n.t.newCrop}</strong>{#if shortcutWarnings.includes('capture')}<span class="shortcut-warning" role="status">{i18n.t.shortcutBusyButton}</span>{/if}</div>
          <kbd>{isMac ? '⌥ + ⇧ + S' : 'Alt + Shift + S'}</kbd>
        </div>
        <div class="shortcut-row">
          <div><strong>{i18n.t.openMemory}</strong>{#if shortcutWarnings.includes('memory')}<span class="shortcut-warning" role="status">{i18n.t.shortcutBusyTray}</span>{/if}</div>
          <kbd>{isMac ? '⌥ + ⇧ + M' : 'Alt + Shift + M'}</kbd>
        </div>
        <div class="shortcut-row">
          <div><strong>{i18n.t.restoreClicks}</strong>{#if shortcutWarnings.includes('restore')}<span class="shortcut-warning" role="status">{i18n.t.shortcutBusyRestore}</span>{/if}</div>
          <kbd>{isMac ? '⌥ + ⇧ + G' : 'Alt + Shift + G'}</kbd>
        </div>
      </section>
    {:else}
      <section class="tab-content" aria-labelledby="settings-title">
        <h1 id="settings-title">{i18n.t.settingsHeading}</h1>
        <p class="section-intro">{i18n.t.settingsIntro}</p>
        
        <div class="setting-row">
          <div class="setting-text">
            <strong>{i18n.t.languageLabel}</strong>
            <span class="setting-desc">{i18n.t.languageDesc}</span>
          </div>
          <div class="segmented-control" role="group" aria-label={i18n.t.languageLabel}>
            <button
              type="button"
              class="segmented-btn"
              class:active={i18n.current === 'en'}
              onclick={() => i18n.set('en')}
            >
              {i18n.t.langEnglish}
            </button>
            <button
              type="button"
              class="segmented-btn"
              class:active={i18n.current === 'es'}
              onclick={() => i18n.set('es')}
            >
              {i18n.t.langSpanish}
            </button>
          </div>
        </div>

        <div class="setting-row">
          <div class="setting-text">
            <strong>{i18n.t.autostartLabel}</strong>
            <span class="setting-desc">{i18n.t.autostartDesc}</span>
          </div>
          <button
            type="button"
            role="switch"
            class="switch-control"
            class:checked={autostart}
            disabled={!desktop || autostartLoading}
            aria-checked={autostart}
            aria-label={i18n.t.autostartAria}
            onclick={toggleAutostart}
          >
            <span class="switch-thumb"></span>
          </button>
        </div>

        <div class="setting-row">
          <div class="setting-text">
            <strong>{i18n.t.soundLabel}</strong>
            <span class="setting-desc">{i18n.t.soundDesc}</span>
          </div>
          <div class="setting-actions">
            {#if soundActive}
              <button
                type="button"
                class="test-sound-btn"
                title={i18n.t.soundTestTitle}
                onclick={() => playClickSound()}
              >
                {i18n.t.soundTestBtn}
              </button>
            {/if}
            <button
              type="button"
              role="switch"
              class="switch-control"
              class:checked={soundActive}
              aria-checked={soundActive}
              aria-label={i18n.t.soundAria}
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
  {#if !desktop}<p class="notice">{i18n.t.previewOnlyNotice}</p>{/if}

  <footer class="companion-footer">
    <button class="button primary" disabled={!desktop || busy} onclick={() => run('trigger_capture')}>
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" aria-hidden="true"><path d="M8 3H3V8M16 3H21V8M21 16V21H16M8 21H3V16M8 12H16M12 8V16" /></svg>
      {busy ? i18n.t.oneMoment : i18n.t.newCrop}
    </button>
    {#if desktop}
      <button class="button memory-link" disabled={busy} onclick={() => run('open_memory_drawer')}>{i18n.t.openMemory}</button>
    {:else}
      <a class="button memory-link" href="/memory">{i18n.t.viewMemory}</a>
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
  .header-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .lang-switch-pill {
    display: inline-flex;
    align-items: center;
    padding: 2px 4px;
    background: #1b1b19;
    border: 1px solid var(--border-color);
    border-radius: 6px;
    font-size: 10px;
    font-weight: 600;
  }
  .lang-pill-btn {
    background: transparent;
    border: none;
    padding: 2px 5px;
    font-size: 10px;
    font-weight: 600;
    font-family: inherit;
    color: var(--text-secondary);
    cursor: pointer;
    border-radius: 3px;
    transition: all 120ms ease;
  }
  .lang-pill-btn:hover {
    color: var(--chassis-light);
  }
  .lang-pill-btn.active {
    background: var(--surface-secondary);
    color: var(--accent-focus, #FF8A50);
  }
  .lang-divider {
    color: #444440;
    font-size: 10px;
    padding: 0 1px;
    user-select: none;
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
