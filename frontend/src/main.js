import { loadConfig, getConfig, saveConfig } from './config.js';
import { renderProfiles, selectProfile, addProfile, getSelectedProfile, getSelectedProfileId, importProfile } from './profiles.js';
import { renderSteps, addStep } from './steps.js';
import { startLaunch, cancelLaunch, isLaunching } from './launcher.js';
import { showSettings, showCloseOnSwitch, showLaunchHistory } from './dialogs.js';
import { showStartupPanel } from './startup.js';
import { toggleProcessPanel } from './processes.js';
import { applyTheme } from './theme.js';

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { getCurrentWindow } = window.__TAURI__.window;

let _lastLaunchedProfileId = null;

async function init() {
  try {
    const config = await loadConfig();
    applyTheme(config.settings?.theme);

    renderProfiles();
    if (config.profiles.length > 0) {
      selectProfile(config.profiles[0].id);
    }

    wireEvents();
    await listenTrayEvents();
  } catch (err) {
    console.error('Init error:', err);
    document.getElementById('status-text').textContent = 'Error loading config: ' + err;
  }
}

function wireEvents() {
  // Custom titlebar
  const appWindow = getCurrentWindow();
  document.getElementById('titlebar-minimize').addEventListener('click', () => appWindow.minimize());
  document.getElementById('titlebar-maximize').addEventListener('click', () => appWindow.toggleMaximize());
  document.getElementById('titlebar-close').addEventListener('click', () => appWindow.close());

  // Add profile
  document.getElementById('btn-add-profile').addEventListener('click', addProfile);

  // Add step
  document.getElementById('btn-add-step').addEventListener('click', addStep);

  // Launch
  document.getElementById('btn-launch').addEventListener('click', handleLaunch);

  // Cancel
  document.getElementById('btn-cancel').addEventListener('click', cancelLaunch);

  // Settings
  document.getElementById('btn-settings').addEventListener('click', handleSettings);

  // Startup apps
  document.getElementById('btn-startup').addEventListener('click', showStartupPanel);

  // Process panel
  document.getElementById('btn-processes').addEventListener('click', toggleProcessPanel);

  // History
  document.getElementById('btn-history').addEventListener('click', handleHistory);

  // Import profile
  document.getElementById('btn-import-profile').addEventListener('click', importProfile);

  // Global hotkeys
  registerHotkeys();
}

async function handleLaunch() {
  const profile = getSelectedProfile();
  if (!profile || isLaunching()) return;

  try {
    const config = getConfig();

    // Close-on-switch: check if previous profile had running processes
    if (config.settings.close_on_switch && _lastLaunchedProfileId && _lastLaunchedProfileId !== profile.id) {
      const lastProfile = config.profiles.find(p => p.id === _lastLaunchedProfileId);
      if (lastProfile) {
        await handleCloseOnSwitch(lastProfile);
      }
    }

    _lastLaunchedProfileId = profile.id;
    const processNames = profile.steps
      .filter(s => s.enabled && s.process_name && s.keep_open !== true)
      .map(s => s.process_name);
    try {
      await invoke('set_last_launch_processes', { processNames });
    } catch (e) {
      console.error('Failed to set last launch processes:', e);
    }
    const enabledSteps = profile.steps.filter(s => s.enabled);
    await startLaunch(profile.steps, config.settings.launch_delay_ms || 500);
    // Record in history (count enabled steps as launched; errors handled by launcher events)
    recordLaunch(profile.id, profile.name, enabledSteps.length, 0);
  } catch (err) {
    console.error('Launch error:', err);
    document.getElementById('status-text').textContent = 'Error: ' + err;
  }
}

async function handleCloseOnSwitch(previousProfile) {
  // Get process names from previous profile that have process_name set
  const processNames = previousProfile.steps
    .filter(s => s.process_name && s.enabled && s.keep_open !== true)
    .map(s => s.process_name);

  if (processNames.length === 0) return;

  try {
    // Check which are actually running
    const running = await invoke('get_running_processes_for_steps', { processNames });
    if (running.length === 0) return;

    // Show dialog
    const toClose = await showCloseOnSwitch(running);
    if (toClose.length === 0) return;

    // Kill selected processes
    for (const name of toClose) {
      try {
        await invoke('kill_process', { name });
      } catch (e) {
        console.error('Failed to kill:', name, e);
      }
    }
  } catch (e) {
    console.error('Close-on-switch error:', e);
  }
}

async function handleSettings() {
  const config = getConfig();
  const result = await showSettings({ ...config.settings });
  if (!result) return;

  config.settings = result;
  await saveConfig(config);
  applyTheme(config.settings?.theme);
}

async function listenTrayEvents() {
  await listen('tray-launch-profile', async (event) => {
    const profileId = event.payload;
    selectProfile(profileId);
    renderProfiles();

    // Small delay so UI updates
    setTimeout(async () => {
      await handleLaunch();
    }, 100);
  });

  await listen('tray-show-window', async () => {
    try {
      await invoke('show_window');
    } catch (e) {
      console.error('Show window error:', e);
    }
  });
}

async function handleHistory() {
  const config = getConfig();
  const cleared = await showLaunchHistory(config.launch_history || []);
  if (cleared) {
    config.launch_history = [];
    await saveConfig(config);
  }
}

function recordLaunch(profileId, profileName, stepsLaunched, stepsFailed) {
  const config = getConfig();
  if (!config.launch_history) config.launch_history = [];
  config.launch_history.push({
    profile_id: profileId,
    profile_name: profileName,
    timestamp: new Date().toISOString(),
    success: stepsFailed === 0,
    steps_launched: stepsLaunched,
    steps_failed: stepsFailed
  });
  // Keep max 100 entries
  if (config.launch_history.length > 100) {
    config.launch_history = config.launch_history.slice(-100);
  }
  saveConfig(config);
}

function registerHotkeys() {
  document.addEventListener('keydown', (e) => {
    const parts = [];
    if (e.ctrlKey) parts.push('Ctrl');
    if (e.altKey) parts.push('Alt');
    if (e.shiftKey) parts.push('Shift');
    if (e.metaKey) parts.push('Super');
    const key = e.key;
    if (!['Control', 'Alt', 'Shift', 'Meta'].includes(key)) {
      parts.push(key.length === 1 ? key.toUpperCase() : key);
    }
    if (parts.length < 2) return; // Need at least modifier + key

    const combo = parts.join('+');
    const config = getConfig();
    if (!config) return;

    const match = config.profiles.find(p => p.hotkey === combo);
    if (match) {
      e.preventDefault();
      selectProfile(match.id);
      renderProfiles();
      setTimeout(() => handleLaunch(), 100);
    }
  });
}

// Start app
document.addEventListener('DOMContentLoaded', init);
