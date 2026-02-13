const { invoke } = window.__TAURI__.core;

let _config = null;

export async function loadConfig() {
  _config = await invoke('get_config');
  return _config;
}

export async function saveConfig(config) {
  if (config) _config = config;
  await invoke('save_config', { config: _config });
}

export function getConfig() {
  return _config;
}

export function setConfig(config) {
  _config = config;
}

export function generateId() {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 8);
}

export function newProfile() {
  return {
    id: generateId(),
    name: 'New Profile',
    description: '',
    steps: [],
    tags: [],
    hotkey: '',
    schedule: null
  };
}

export function newStep(type = 'app') {
  const base = {
    id: generateId(),
    name: '',
    type: type,
    enabled: true,
    delay_after: 500,
    process_name: ''
  };

  switch (type) {
    case 'app':
      return { ...base, target: '', check_running: true };
    case 'terminal':
      return { ...base, command: '', working_dir: '', keep_open: true };
    case 'folder':
      return { ...base, target: '' };
    case 'url':
      return { ...base, target: '' };
    default:
      return { ...base, target: '' };
  }
}
