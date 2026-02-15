const THEMES = new Set(['dark', 'light', 'auto', 'frosted']);

export function applyTheme(theme) {
  const root = document.documentElement;
  const value = (theme || 'dark').toLowerCase();
  root.dataset.theme = THEMES.has(value) ? value : 'dark';
}
