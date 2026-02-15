import { loadConfig, getConfig, saveConfig } from './config.js';
import { renderProfiles, selectProfile, addProfile, getSelectedProfile, getSelectedProfileId, importProfile } from './profiles.js';
import { renderSteps, addStep } from './steps.js';
import { startLaunch, cancelLaunch, isLaunching } from './launcher.js';
import { showSettings, showCloseOnSwitch, showLaunchHistory, showKillAndWipe, showInfo } from './dialogs.js';
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
    await maybeShowPostLogoutMessage();
    await handleStartupFlags();
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

  // Kill & Wipe
  const killBtn = document.getElementById('btn-kill-wipe');
  if (killBtn) {
    killBtn.addEventListener('click', () => handleKillAndWipe(false));
  }

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

async function handleKillAndWipe(immediateOverride) {
  const config = getConfig();
  if (!config) return;

  const killWipe = normalizeKillWipeSettings(config.settings?.kill_wipe);
  const runImmediate = immediateOverride === true || killWipe.confirm_before === false;

  if (!runImmediate) {
    const result = await showKillAndWipe(killWipe);
    if (!result) return;
    config.settings.kill_wipe = result.settings;
    await saveConfig(config);

    if (result.create_shortcut) {
      try {
        await invoke('create_kill_and_wipe_shortcut', { immediate: result.settings.confirm_before === false });
      } catch (e) {
        console.error('Shortcut creation failed:', e);
      }
    }

    await runKillAndWipe(result.settings);
    return;
  }

  await runKillAndWipe(killWipe);
}

async function runKillAndWipe(settings) {
  const status = document.getElementById('status-text');
  if (status) status.textContent = 'Kill & Wipe in progress...';

  const options = {
    kill_processes: settings.kill_processes !== false,
    clear_temp: settings.clear_temp !== false,
    clear_browsers: settings.clear_browsers !== false,
    flush_dns: settings.flush_dns !== false,
    logout: settings.logout !== false
  };

  try {
    const report = await invoke('kill_and_wipe', { options });
    if (options.logout) {
      if (status) status.textContent = 'Logging out...';
      return;
    }

    const summary = [
      `Killed processes: ${report.killed_count}`,
      `Browser clears: ${report.browser_cleared.length > 0 ? report.browser_cleared.join(', ') : 'None'}`,
      `DNS flushed: ${report.dns_flushed ? 'Yes' : 'No'}`
    ].join(' | ');

    const warningCount = (report.kill_failures?.length || 0)
      + (report.temp_failures?.length || 0)
      + (report.browser_failures?.length || 0)
      + (options.flush_dns && !report.dns_flushed ? 1 : 0);

    if (status) {
      status.textContent = warningCount > 0 ? `${summary} | Warnings: ${warningCount}` : summary;
    }
  } catch (e) {
    console.error('Kill & Wipe failed:', e);
    if (status) status.textContent = 'Kill & Wipe failed. Check logs.';
  }
}

function normalizeKillWipeSettings(settings) {
  const base = {
    confirm_before: true,
    kill_processes: true,
    clear_temp: true,
    clear_browsers: true,
    flush_dns: true,
    logout: true
  };
  return { ...base, ...(settings || {}) };
}

async function maybeShowPostLogoutMessage() {
  const config = getConfig();
  if (!config?.settings?.post_logout_message_pending) return;

  await showInfo('Cleanup Complete', 'We cleaned up everything while you pannicked!');
  config.settings.post_logout_message_pending = false;
  await saveConfig(config);
}

async function handleStartupFlags() {
  try {
    const flags = await invoke('get_startup_flags');
    if (!flags?.kill_and_wipe) return;
    setTimeout(() => {
      handleKillAndWipe(flags.kill_and_wipe_immediate);
    }, 150);
  } catch (e) {
    console.error('Startup flags error:', e);
  }
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
