import { isTauri, isMac } from '$lib/desktop';
import { emit, listen } from '@tauri-apps/api/event';

export type Locale = 'en' | 'es';

export const translations = {
  en: {
    // Brand & generic
    appName: 'Screen-Pin',
    tagline: 'Floating screen crop utility',
    previewOnlyNotice: 'Preview: captures, shortcuts, and autostart require the desktop app.',
    oneMoment: 'One moment…',
    retry: 'Retry',
    cancel: 'Cancel',

    // Companion View
    companionTitle: 'Screen-Pin · Shortcuts & Settings',
    shortcutsTab: 'Shortcuts',
    settingsTab: 'Settings',
    keyboardShortcutsHeading: 'Keyboard shortcuts',
    shortcutsIntro: 'Use them from any window on your computer.',
    newCrop: 'New pin',
    openMemory: 'Open memory',
    viewMemory: 'View memory',
    restoreClicks: 'Restore clicks',
    shortcutBusyButton: 'Shortcut in use · use the button',
    shortcutBusyTray: 'Shortcut in use · use the button or tray',
    shortcutBusyRestore: 'Shortcut in use · open Memory or use tray',

    // Settings
    settingsHeading: 'Settings',
    settingsIntro: 'System preferences and experience.',
    languageLabel: 'Language',
    languageDesc: 'Choose interface language.',
    langEnglish: 'English',
    langSpanish: 'Español',
    autostartLabel: isMac ? 'Start at login' : 'Start with Windows',
    autostartDesc: isMac
      ? 'Launch Screen-Pin in the menu bar when logging in.'
      : 'Launch Screen-Pin minimized to the tray when you turn on your PC.',
    autostartAria: isMac ? 'Start Screen-Pin at login' : 'Start Screen-Pin with Windows',
    autostartEnabledNotice: isMac ? 'Start at login enabled' : 'Windows autostart enabled',
    autostartDisabledNotice: isMac ? 'Start at login disabled' : 'Windows autostart disabled',
    soundLabel: 'Sound effects (Tactile)',
    soundDesc: 'Play a tactile tile sound when clicking buttons and actions.',
    soundTestBtn: 'Test',
    soundTestTitle: 'Test tile sound',
    soundAria: 'Tactile sound effects',

    // Pin View & Toolbar
    pinTitle: 'Pin · Screen-Pin',
    pinActionsAria: 'Pin actions',
    copyImage: 'Copy image',
    imageCopied: 'Image copied',
    copyImageTitle: isMac ? 'Copy image to clipboard (⌘C)' : 'Copy image to clipboard (Ctrl+C)',
    copyText: 'Copy text',
    textCopied: 'Text copied',
    ocrLoading: 'Recognizing text…',
    ocrError: 'Reading unavailable',
    ocrEmpty: 'No text detected',
    copyAllText: 'Copy all text',
    ghost: 'Ghost',
    ghostTitle: isMac
      ? 'Pass clicks through. Restore with ⌥+⇧+G or from menu bar.'
      : 'Pass clicks through. Restore with Alt+Shift+G or from tray.',
    archive: 'Archive',
    archiveTitle: 'Save to Memory and close',
    floatingCropAria: 'Floating screen pin',
    dragWindowMove: 'Drag to move window',
    dragCropTitle: 'Drag pin',
    capturedImageAria: 'Captured screen area',
    extractedTextAria: 'Extracted text from image',
    pinImageUnavailable: 'Pin image is no longer available.',
    pinOpenInDesktop: 'Open this pin in Screen-Pin to view its image and text.',
    numberCopied: 'Number copied',
    clipboardError: 'Could not copy to clipboard.',
    copyImageError: 'Could not copy image to clipboard.',
    copyTextError: 'Could not copy text to clipboard.',

    // Memory View
    memoryTitle: 'Memory · Screen-Pin',
    memoryHeading: 'Memory',
    searchPlaceholder: 'Search pins…',
    searchAria: 'Search within pins text',
    loadingPins: 'Loading pins',
    pinCount: (count: number, hasMore: boolean) => `${count}${hasMore ? '+' : ''} pin${count === 1 ? '' : 's'}`,
    savedPinsAria: 'Saved pins',
    memoryDesktopTitle: 'Your memory lives on the desktop',
    memoryDesktopDesc: 'Open Screen-Pin to explore your local captures. Your pins are not loaded in this preview.',
    backToHome: 'Return to home',
    loadingYourPins: 'Loading your pins…',
    noMatches: 'No matches',
    tryAnotherKeyword: 'Try another keyword.',
    clearSearch: 'Clear search',
    noPinsYet: 'No pins yet',
    createPinPrompt: 'Create a pin to view it here.',
    createFirstPin: 'Create my first pin',
    repinTitle: 'Click to re-pin or drag',
    repinAria: 'Re-pin or drag this pin',
    repinBadge: 'Pin',
    bankDigitsBadge: 'Bank details',
    wifiCredentialBadge: 'Credential',
    genericPinBadge: 'Pin',
    timeNow: 'Now',
    timeMinutesAgo: (m: number) => `${m}m ago`,
    timeHoursAgo: (h: number) => `${h}h ago`,
    keptForever: 'Kept forever',
    expiresInMin: (m: number) => `Expires in ${m} min`,
    pendingCleanup: 'Pending cleanup',
    ocrTextLoadingNote: 'Recognizing text…',
    ocrTextErrorNote: 'Text reading unavailable',
    keepBtn: 'Keep',
    releaseBtn: 'Release',
    keepTitle: 'Keep permanently',
    releaseTitle: 'Automatically delete in 30 minutes',
    memoryCopyTextError: 'Could not copy text to clipboard. Try again.',

    // Overlay View
    overlayAria: 'Screen capture selection canvas',
    frozenBgAlt: 'Frozen screen for cropping',
    overlayErrorNonDesktop: 'Screen capture is available in the desktop app.',
    overlayErrorNoMonitor: 'Monitor not found. Cancel and try again.',
    overlayBgLoadError: 'Could not load screen. Cancel and try again.',
    hintDrag: 'Drag: Select area',
    hintCancel: 'Esc / Right-Click: Cancel',
    hintArrows: 'Arrows: Fine tune',
    hintSpace: 'Space: Lock anchor',
    hintEnter: 'Enter: Confirm',
    savingCrop: 'Saving pin…',
  },
  es: {
    // Brand & generic
    appName: 'Screen-Pin',
    tagline: 'Utilidad de recortes flotantes',
    previewOnlyNotice: 'Vista previa: las capturas, atajos y arranque automático requieren la aplicación de escritorio.',
    oneMoment: 'Un momento…',
    retry: 'Reintentar',
    cancel: 'Cancelar',

    // Companion View
    companionTitle: 'Screen-Pin · Atajos y configuración',
    shortcutsTab: 'Atajos',
    settingsTab: 'Configuración',
    keyboardShortcutsHeading: 'Atajos de teclado',
    shortcutsIntro: 'Úsalos desde cualquier ventana de tu equipo.',
    newCrop: 'Nuevo recorte',
    openMemory: 'Abrir memoria',
    viewMemory: 'Ver memoria',
    restoreClicks: 'Recuperar clics',
    shortcutBusyButton: 'Atajo ocupado · usa el botón',
    shortcutBusyTray: 'Atajo ocupado · usa el botón o la bandeja',
    shortcutBusyRestore: 'Atajo ocupado · abre Memoria o usa la bandeja',

    // Settings
    settingsHeading: 'Configuración',
    settingsIntro: 'Preferencias de sistema y experiencia.',
    languageLabel: 'Idioma',
    languageDesc: 'Elige el idioma de la interfaz.',
    langEnglish: 'English',
    langSpanish: 'Español',
    autostartLabel: isMac ? 'Iniciar con el sistema' : 'Arrancar con Windows',
    autostartDesc: isMac
      ? 'Inicia Screen-Pin en la barra de menús al iniciar sesión.'
      : 'Inicia Screen-Pin minimizado en la bandeja al encender el equipo.',
    autostartAria: isMac ? 'Iniciar Screen-Pin con el sistema' : 'Arrancar Screen-Pin con Windows',
    autostartEnabledNotice: isMac ? 'Inicio con el sistema activado' : 'Arranque con Windows activado',
    autostartDisabledNotice: isMac ? 'Inicio automático desactivado' : 'Arranque automático desactivado',
    soundLabel: 'Efectos de sonido (Ficha)',
    soundDesc: 'Reproduce un sonido de ficha táctil al pulsar opciones y acciones.',
    soundTestBtn: 'Probar',
    soundTestTitle: 'Probar sonido de ficha',
    soundAria: 'Efectos de sonido táctiles',

    // Pin View & Toolbar
    pinTitle: 'Recorte · Screen-Pin',
    pinActionsAria: 'Acciones del recorte',
    copyImage: 'Copiar imagen',
    imageCopied: 'Imagen copiada',
    copyImageTitle: isMac ? 'Copiar imagen al portapapeles (⌘C)' : 'Copiar imagen al portapapeles (Ctrl+C)',
    copyText: 'Copiar texto',
    textCopied: 'Texto copiado',
    ocrLoading: 'Reconociendo texto…',
    ocrError: 'Lectura no disponible',
    ocrEmpty: 'No se detectó texto',
    copyAllText: 'Copiar todo el texto',
    ghost: 'Fantasma',
    ghostTitle: isMac
      ? 'Dejar pasar los clics. Recuperar con ⌥+⇧+G o desde la barra de menús.'
      : 'Dejar pasar los clics. Recuperar con Alt+Shift+G o desde la bandeja.',
    archive: 'Archivar',
    archiveTitle: 'Guardar en Memoria y cerrar',
    floatingCropAria: 'Recorte de pantalla flotante',
    dragWindowMove: 'Arrastrar para mover ventana',
    dragCropTitle: 'Arrastrar recorte',
    capturedImageAria: 'Captura fijada en pantalla',
    extractedTextAria: 'Texto extraído de la imagen',
    pinImageUnavailable: 'La imagen del recorte ya no está disponible.',
    pinOpenInDesktop: 'Abre este recorte en Screen-Pin para ver su imagen y texto.',
    numberCopied: 'Número copiado',
    clipboardError: 'No se pudo copiar al portapapeles.',
    copyImageError: 'No se pudo copiar la imagen al portapapeles.',
    copyTextError: 'No se pudo copiar el texto al portapapeles.',

    // Memory View
    memoryTitle: 'Memoria · Screen-Pin',
    memoryHeading: 'Memoria',
    searchPlaceholder: 'Buscar recortes…',
    searchAria: 'Buscar en el texto de los recortes',
    loadingPins: 'Cargando recortes',
    pinCount: (count: number, hasMore: boolean) => `${count}${hasMore ? '+' : ''} recorte${count === 1 ? '' : 's'}`,
    savedPinsAria: 'Recortes guardados',
    memoryDesktopTitle: 'Tu memoria vive en el escritorio',
    memoryDesktopDesc: 'Abre Screen-Pin para consultar tus capturas locales. Tus recortes no se cargan en esta vista previa.',
    backToHome: 'Volver al inicio',
    loadingYourPins: 'Cargando tus recortes…',
    noMatches: 'Sin coincidencias',
    tryAnotherKeyword: 'Prueba con otra palabra.',
    clearSearch: 'Limpiar búsqueda',
    noPinsYet: 'Aún no hay recortes',
    createPinPrompt: 'Crea un recorte para verlo aquí.',
    createFirstPin: 'Crear mi primer recorte',
    repinTitle: 'Haz clic para volver a fijar o arrastrar recorte',
    repinAria: 'Volver a fijar o arrastrar este recorte',
    repinBadge: 'Fijar',
    bankDigitsBadge: 'Datos bancarios',
    wifiCredentialBadge: 'Credencial',
    genericPinBadge: 'Recorte',
    timeNow: 'Ahora',
    timeMinutesAgo: (m: number) => `Hace ${m} min`,
    timeHoursAgo: (h: number) => `Hace ${h} h`,
    keptForever: 'Conservado',
    expiresInMin: (m: number) => `Se elimina en ${m} min`,
    pendingCleanup: 'Pendiente de limpieza',
    ocrTextLoadingNote: 'Reconociendo texto…',
    ocrTextErrorNote: 'Lectura de texto no disponible',
    keepBtn: 'Conservar',
    releaseBtn: 'Liberar',
    keepTitle: 'Conservar sin caducidad',
    releaseTitle: 'Volver a eliminar automáticamente en 30 minutos',
    memoryCopyTextError: 'No se pudo copiar el texto al portapapeles. Inténtalo de nuevo.',

    // Overlay View
    overlayAria: 'Lienzo de selección de captura',
    frozenBgAlt: 'Pantalla congelada para recorte',
    overlayErrorNonDesktop: 'La captura está disponible en la aplicación de escritorio.',
    overlayErrorNoMonitor: 'No se encontró el monitor. Cancela y vuelve a capturar.',
    overlayBgLoadError: 'No se pudo cargar la pantalla. Cancela e inténtalo de nuevo.',
    hintDrag: 'Arrastrar: Delimitar área',
    hintCancel: 'Esc / Clic Der.: Cancelar',
    hintArrows: 'Flechas: Ajuste fino',
    hintSpace: 'Espacio: Fijar ancla',
    hintEnter: 'Enter: Confirmar',
    savingCrop: 'Guardando recorte…',
  }
};

const LANGUAGE_STORAGE_KEY = 'screen_pin_language';

function getInitialLocale(): Locale {
  if (typeof window === 'undefined') return 'en';
  try {
    const stored = localStorage.getItem(LANGUAGE_STORAGE_KEY);
    if (stored === 'es' || stored === 'en') {
      return stored;
    }
  } catch {
    // Ignorar si localStorage no está disponible
  }
  return 'en'; // Default is English
}

class I18nManager {
  current = $state<Locale>(getInitialLocale());

  set(lang: Locale) {
    if (lang !== 'en' && lang !== 'es') return;
    this.current = lang;
    if (typeof window !== 'undefined') {
      try {
        localStorage.setItem(LANGUAGE_STORAGE_KEY, lang);
      } catch {
        // Ignorar
      }
      window.dispatchEvent(new CustomEvent('screen-pin:language-changed', { detail: lang }));
      if (isTauri()) {
        void emit('language:changed', lang);
      }
    }
  }

  get t() {
    return translations[this.current];
  }
}

export const i18n = new I18nManager();

if (typeof window !== 'undefined') {
  window.addEventListener('storage', (e) => {
    if (e.key === LANGUAGE_STORAGE_KEY && (e.newValue === 'en' || e.newValue === 'es')) {
      i18n.current = e.newValue;
    }
  });

  window.addEventListener('screen-pin:language-changed', (e) => {
    const detail = (e as CustomEvent<Locale>).detail;
    if (detail === 'en' || detail === 'es') {
      i18n.current = detail;
    }
  });

  if (isTauri()) {
    void listen<Locale>('language:changed', (event) => {
      if (event.payload === 'en' || event.payload === 'es') {
        i18n.current = event.payload;
      }
    });
  }
}
