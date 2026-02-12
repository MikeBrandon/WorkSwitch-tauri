import { loadConfig, getConfig, saveConfig } from './config.js';
import { renderProfiles, selectProfile, addProfile, getSelectedProfile, getSelectedProfileId } from './profiles.js';
import { renderSteps, addStep } from './steps.js';
import { startLaunch, cancelLaunch, isLaunching } from './launcher.js';
import { showSettings, showCloseOnSwitch } from './dialogs.js';

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { getCurrentWindow } = window.__TAURI__.window;

let _lastLaunchedProfileId = null;

async function init() {
  try {
    const config = await loadConfig();

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

  // Close via window manager - intercept is handled by Rust backend
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
    await startLaunch(profile.steps, config.settings.launch_delay_ms || 500);
  } catch (err) {
    console.error('Launch error:', err);
    document.getElementById('status-text').textContent = 'Error: ' + err;
  }
}

async function handleCloseOnSwitch(previousProfile) {
  // Get process names from previous profile that have process_name set
  const processNames = previousProfile.steps
    .filter(s => s.process_name && s.enabled)
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

// Start app
document.addEventListener('DOMContentLoaded', init);
