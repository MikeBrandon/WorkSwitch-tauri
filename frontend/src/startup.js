import { getConfig, saveConfig, newStep, generateId } from './config.js';
import { showStepEditor, showDuplicateNamePrompt } from './dialogs.js';

const { invoke } = window.__TAURI__.core;

let _visible = false;

export function showStartupPanel() {
  const overlay = document.getElementById('modal-overlay');
  const content = document.getElementById('modal-content');

  content.innerHTML = buildStartupHTML();
  overlay.classList.remove('hidden');
  _visible = true;

  wireStartupEvents();
}

function buildStartupHTML() {
  const config = getConfig();
  const apps = config.startup_apps || [];

  let stepsHtml = '';
  if (apps.length === 0) {
    stepsHtml = '<div class="empty-state" style="padding:24px 0"><span class="empty-state-icon">&#9889;</span><span>No startup apps</span></div>';
  } else {
    stepsHtml = apps.map((step, index) => {
      const badgeLabel = step.type === 'terminal' ? 'CMD' : step.type.toUpperCase();
      const detail = step.target || step.command || '';
      return `
        <div class="step-card startup-step-card ${step.enabled ? '' : 'disabled'}" data-index="${index}">
          <input type="checkbox" class="step-checkbox startup-toggle" ${step.enabled ? 'checked' : ''} data-index="${index}">
          <span class="step-badge ${step.type}">${badgeLabel}</span>
          <div class="step-info">
            <div class="step-name">${escapeHtml(step.name || '(unnamed)')}</div>
            <div class="step-detail">${escapeHtml(detail)}</div>
          </div>
          <div class="step-actions" style="display:flex">
            ${index > 0 ? '<button class="step-action-btn startup-move-up" data-index="' + index + '" title="Move up">&#9650;</button>' : ''}
            ${index < apps.length - 1 ? '<button class="step-action-btn startup-move-down" data-index="' + index + '" title="Move down">&#9660;</button>' : ''}
            <button class="step-action-btn startup-edit" data-index="${index}" title="Edit">&#9998;</button>
            <button class="step-action-btn danger startup-delete" data-index="${index}" title="Delete">&#10005;</button>
          </div>
        </div>
      `;
    }).join('');
  }

  return `
    <div class="modal-title">Startup Apps</div>
    <p style="color: var(--text-secondary); margin-bottom: 12px; font-size: 12px;">
      These apps launch automatically when WorkSwitch starts.
    </p>
    <div style="max-height: 300px; overflow-y: auto; margin-bottom: 12px;">
      ${stepsHtml}
    </div>
    <div class="modal-actions" style="justify-content: space-between">
      <button class="secondary-btn" id="startup-add">+ Add App</button>
      <button class="btn-secondary" id="startup-close">Close</button>
    </div>
  `;
}

function wireStartupEvents() {
  document.getElementById('startup-close').addEventListener('click', closeStartup);
  document.getElementById('startup-add').addEventListener('click', addStartupApp);

  // Toggle enable/disable
  document.querySelectorAll('.startup-toggle').forEach(cb => {
    cb.addEventListener('change', async (e) => {
      const idx = parseInt(e.target.dataset.index);
      const config = getConfig();
      if (config.startup_apps[idx]) {
        config.startup_apps[idx].enabled = e.target.checked;
        await saveConfig(config);
        refreshStartup();
      }
    });
  });

  // Move up
  document.querySelectorAll('.startup-move-up').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      const idx = parseInt(e.target.dataset.index);
      const config = getConfig();
      if (idx > 0) {
        [config.startup_apps[idx - 1], config.startup_apps[idx]] = [config.startup_apps[idx], config.startup_apps[idx - 1]];
        await saveConfig(config);
        refreshStartup();
      }
    });
  });

  // Move down
  document.querySelectorAll('.startup-move-down').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      const idx = parseInt(e.target.dataset.index);
      const config = getConfig();
      if (idx < config.startup_apps.length - 1) {
        [config.startup_apps[idx], config.startup_apps[idx + 1]] = [config.startup_apps[idx + 1], config.startup_apps[idx]];
        await saveConfig(config);
        refreshStartup();
      }
    });
  });

  // Edit
  document.querySelectorAll('.startup-edit').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      const idx = parseInt(e.target.dataset.index);
      const config = getConfig();
      const step = config.startup_apps[idx];
      if (!step) return;

      closeStartup();
      const result = await showStepEditor({ ...step }, false);
      if (result) {
        config.startup_apps[idx] = result;
        await saveConfig(config);
      }
      showStartupPanel();
    });
  });

  // Delete
  document.querySelectorAll('.startup-delete').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      const idx = parseInt(e.target.dataset.index);
      const config = getConfig();
      config.startup_apps.splice(idx, 1);
      await saveConfig(config);
      refreshStartup();
    });
  });
}

async function addStartupApp() {
  closeStartup();
  const step = newStep('app');
  const result = await showStepEditor(step, true);
  if (result) {
    const config = getConfig();
    if (!config.startup_apps) config.startup_apps = [];
    config.startup_apps.push(result);
    await saveConfig(config);
  }
  showStartupPanel();
}

function refreshStartup() {
  const content = document.getElementById('modal-content');
  content.innerHTML = buildStartupHTML();
  wireStartupEvents();
}

function closeStartup() {
  document.getElementById('modal-overlay').classList.add('hidden');
  document.getElementById('modal-content').innerHTML = '';
  _visible = false;
}

function escapeHtml(str) {
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}
