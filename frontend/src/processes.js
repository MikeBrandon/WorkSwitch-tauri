import { getConfig } from './config.js';

const { invoke } = window.__TAURI__.core;

let _pollInterval = null;
let _expanded = false;

export function toggleProcessPanel() {
  const panel = document.getElementById('process-panel');
  _expanded = !_expanded;

  if (_expanded) {
    panel.classList.remove('collapsed');
    startPolling();
    refreshProcesses();
  } else {
    panel.classList.add('collapsed');
    stopPolling();
  }
}

function startPolling() {
  stopPolling();
  _pollInterval = setInterval(refreshProcesses, 4000);
}

function stopPolling() {
  if (_pollInterval) {
    clearInterval(_pollInterval);
    _pollInterval = null;
  }
}

async function refreshProcesses() {
  const config = getConfig();
  if (!config) return;

  // Collect all unique process names across all profiles + startup apps
  const processMap = new Map(); // process_name -> step name (for display)
  for (const profile of config.profiles) {
    for (const step of profile.steps) {
      if (step.process_name && step.process_name.trim()) {
        processMap.set(step.process_name.toLowerCase(), step.process_name);
      }
    }
  }
  if (config.startup_apps) {
    for (const step of config.startup_apps) {
      if (step.process_name && step.process_name.trim()) {
        processMap.set(step.process_name.toLowerCase(), step.process_name);
      }
    }
  }

  const processNames = [...processMap.values()];
  if (processNames.length === 0) {
    renderProcessList([], []);
    return;
  }

  try {
    const running = await invoke('get_running_processes_for_steps', { processNames });
    renderProcessList(processNames, running);
  } catch (e) {
    console.error('Process poll error:', e);
  }
}

function renderProcessList(allNames, runningNames) {
  const list = document.getElementById('process-list');
  const runningSet = new Set(runningNames.map(n => n.toLowerCase()));

  if (allNames.length === 0) {
    list.innerHTML = '<div class="process-empty">No tracked processes. Add process names to your steps.</div>';
    return;
  }

  // Sort: running first, then alphabetical
  const sorted = [...allNames].sort((a, b) => {
    const aRunning = runningSet.has(a.toLowerCase());
    const bRunning = runningSet.has(b.toLowerCase());
    if (aRunning && !bRunning) return -1;
    if (!aRunning && bRunning) return 1;
    return a.localeCompare(b);
  });

  list.innerHTML = sorted.map(name => {
    const isRunning = runningSet.has(name.toLowerCase());
    return `
      <div class="process-item">
        <span class="process-dot ${isRunning ? 'running' : 'stopped'}"></span>
        <span class="process-name">${escapeHtml(name)}</span>
        <span class="process-status">${isRunning ? 'Running' : 'Stopped'}</span>
        ${isRunning ? '<button class="process-kill-btn" data-name="' + escapeAttr(name) + '">Kill</button>' : ''}
      </div>
    `;
  }).join('');

  // Wire kill buttons
  list.querySelectorAll('.process-kill-btn').forEach(btn => {
    btn.addEventListener('click', async () => {
      const name = btn.dataset.name;
      try {
        await invoke('kill_process', { name });
        // Quick refresh
        setTimeout(refreshProcesses, 500);
      } catch (e) {
        console.error('Kill failed:', e);
      }
    });
  });
}

// End All button
document.addEventListener('DOMContentLoaded', () => {
  const endAllBtn = document.getElementById('btn-end-all');
  if (endAllBtn) {
    endAllBtn.addEventListener('click', async () => {
      const config = getConfig();
      if (!config) return;

      const processNames = new Set();
      for (const profile of config.profiles) {
        for (const step of profile.steps) {
          if (step.process_name) processNames.add(step.process_name);
        }
      }
      if (config.startup_apps) {
        for (const step of config.startup_apps) {
          if (step.process_name) processNames.add(step.process_name);
        }
      }

      const names = [...processNames];
      if (names.length === 0) return;

      try {
        const running = await invoke('get_running_processes_for_steps', { processNames: names });
        for (const name of running) {
          try {
            await invoke('kill_process', { name });
          } catch (e) {
            console.error('Kill failed:', name, e);
          }
        }
        setTimeout(refreshProcesses, 500);
      } catch (e) {
        console.error('End all error:', e);
      }
    });
  }

  // Collapse button
  const collapseBtn = document.getElementById('btn-collapse-processes');
  if (collapseBtn) {
    collapseBtn.addEventListener('click', () => {
      if (_expanded) toggleProcessPanel();
    });
  }
});

function escapeHtml(str) {
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

function escapeAttr(str) {
  return str.replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/'/g, '&#39;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}
