import { getConfig, saveConfig, newStep } from './config.js';
import { getSelectedProfile, getSelectedProfileId } from './profiles.js';
import { showStepEditor, showDuplicateNamePrompt } from './dialogs.js';

export function renderSteps() {
  const profile = getSelectedProfile();
  const list = document.getElementById('step-list');

  if (!profile) {
    list.innerHTML = '<div class="empty-state"><span class="empty-state-icon">&#9881;</span><span>Select a profile</span></div>';
    return;
  }

  if (profile.steps.length === 0) {
    list.innerHTML = '<div class="empty-state"><span class="empty-state-icon">&#10010;</span><span>No steps yet. Add one!</span></div>';
    return;
  }

  list.innerHTML = '';

  profile.steps.forEach((step, index) => {
    const card = document.createElement('div');
    card.className = 'step-card' + (step.enabled ? '' : ' disabled');
    card.dataset.stepId = step.id;

    const badgeLabel = step.type === 'terminal'
      ? (step.terminal_app === 'cmd' ? 'CMD' : 'WT')
      : step.type.toUpperCase();
    const detail = getStepDetail(step);

    card.innerHTML = `
      <input type="checkbox" class="step-checkbox" ${step.enabled ? 'checked' : ''} title="Enable/disable">
      <span class="step-badge ${step.type}">${badgeLabel}</span>
      <div class="step-info">
        <div class="step-name">${escapeHtml(step.name || '(unnamed)')}</div>
        <div class="step-detail">${escapeHtml(detail)}</div>
      </div>
      <div class="step-actions">
        ${index > 0 ? '<button class="step-action-btn move-top" title="Move to top">&#9194;</button>' : ''}
        ${index > 0 ? '<button class="step-action-btn move-up" title="Move up">&#9650;</button>' : ''}
        ${index < profile.steps.length - 1 ? '<button class="step-action-btn move-down" title="Move down">&#9660;</button>' : ''}
        ${index < profile.steps.length - 1 ? '<button class="step-action-btn move-bottom" title="Move to bottom">&#9193;</button>' : ''}
        <button class="step-action-btn edit-step" title="Edit">&#9998;</button>
        <button class="step-action-btn dup-step" title="Duplicate">&#10697;</button>
        <button class="step-action-btn danger del-step" title="Delete">&#10005;</button>
      </div>
    `;

    // Checkbox toggle
    card.querySelector('.step-checkbox').addEventListener('change', async (e) => {
      e.stopPropagation();
      step.enabled = e.target.checked;
      await saveConfig();
      renderSteps();
    });

    // Move to top
    const topBtn = card.querySelector('.move-top');
    if (topBtn) {
      topBtn.addEventListener('click', (e) => { e.stopPropagation(); moveStepToTop(index); });
    }

    // Move up
    const upBtn = card.querySelector('.move-up');
    if (upBtn) {
      upBtn.addEventListener('click', (e) => { e.stopPropagation(); moveStep(index, -1); });
    }

    // Move down
    const downBtn = card.querySelector('.move-down');
    if (downBtn) {
      downBtn.addEventListener('click', (e) => { e.stopPropagation(); moveStep(index, 1); });
    }

    // Move to bottom
    const bottomBtn = card.querySelector('.move-bottom');
    if (bottomBtn) {
      bottomBtn.addEventListener('click', (e) => { e.stopPropagation(); moveStepToBottom(index); });
    }

    // Edit
    card.querySelector('.edit-step').addEventListener('click', (e) => {
      e.stopPropagation();
      editStep(step);
    });

    // Duplicate
    card.querySelector('.dup-step').addEventListener('click', (e) => {
      e.stopPropagation();
      duplicateStep(step);
    });

    // Delete
    card.querySelector('.del-step').addEventListener('click', (e) => {
      e.stopPropagation();
      deleteStep(step.id);
    });

    list.appendChild(card);
  });
}

function getStepDetail(step) {
  switch (step.type) {
    case 'app': return step.target || '';
    case 'terminal': {
      const defaultLabel = step.terminal_app === 'cmd'
        ? 'Open Command Prompt'
        : 'Open Windows Terminal';
      return step.command || defaultLabel;
    }
    case 'folder': return step.target || '';
    case 'url': return step.target || '';
    default: return '';
  }
}

async function moveStep(index, direction) {
  const profile = getSelectedProfile();
  if (!profile) return;

  const newIndex = index + direction;
  if (newIndex < 0 || newIndex >= profile.steps.length) return;

  const steps = profile.steps;
  [steps[index], steps[newIndex]] = [steps[newIndex], steps[index]];
  await saveConfig();
  renderSteps();
}

async function moveStepToTop(index) {
  const profile = getSelectedProfile();
  if (!profile || index <= 0) return;

  const [step] = profile.steps.splice(index, 1);
  profile.steps.unshift(step);
  await saveConfig();
  renderSteps();
}

async function moveStepToBottom(index) {
  const profile = getSelectedProfile();
  if (!profile || index >= profile.steps.length - 1) return;

  const [step] = profile.steps.splice(index, 1);
  profile.steps.push(step);
  await saveConfig();
  renderSteps();
}

export async function addStep() {
  const profile = getSelectedProfile();
  if (!profile) return;

  const step = newStep('app');
  const result = await showStepEditor(step, true);
  if (!result) return;

  profile.steps.push(result);
  await saveConfig();
  renderSteps();
}

async function editStep(step) {
  const result = await showStepEditor({ ...step }, false);
  if (!result) return;

  const profile = getSelectedProfile();
  if (!profile) return;

  const idx = profile.steps.findIndex(s => s.id === step.id);
  if (idx === -1) return;

  profile.steps[idx] = result;
  await saveConfig();
  renderSteps();
}

async function duplicateStep(step) {
  const profile = getSelectedProfile();
  if (!profile) return;

  const defaultName = step.name + ' (copy)';
  const name = await showDuplicateNamePrompt(defaultName);
  if (name === null) return;

  const { generateId } = await import('./config.js');
  const dup = { ...step, id: generateId(), name };
  profile.steps.push(dup);
  await saveConfig();
  renderSteps();
}

async function deleteStep(id) {
  const profile = getSelectedProfile();
  if (!profile) return;

  profile.steps = profile.steps.filter(s => s.id !== id);
  await saveConfig();
  renderSteps();
}

function escapeHtml(str) {
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}
