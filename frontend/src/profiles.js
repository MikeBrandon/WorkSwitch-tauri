import { getConfig, saveConfig, newProfile } from './config.js';
import { renderSteps } from './steps.js';
import { showProfileEditor } from './dialogs.js';

let _selectedProfileId = null;

export function getSelectedProfileId() {
  return _selectedProfileId;
}

export function getSelectedProfile() {
  const config = getConfig();
  if (!config || !_selectedProfileId) return null;
  return config.profiles.find(p => p.id === _selectedProfileId) || null;
}

export function renderProfiles() {
  const config = getConfig();
  const list = document.getElementById('profile-list');

  if (!config || config.profiles.length === 0) {
    list.innerHTML = '<div class="empty-state"><span class="empty-state-icon">&#128203;</span><span>No profiles</span></div>';
    updateContentHeader(null);
    return;
  }

  list.innerHTML = '';
  for (const profile of config.profiles) {
    const card = document.createElement('div');
    card.className = 'profile-card' + (profile.id === _selectedProfileId ? ' active' : '');
    card.innerHTML = `
      <div class="profile-card-name">${escapeHtml(profile.name)}</div>
      <div class="profile-card-desc">${escapeHtml(profile.description || '')}</div>
      <div class="profile-card-count">${profile.steps.length} step${profile.steps.length !== 1 ? 's' : ''}</div>
      <div class="profile-card-actions">
        <button class="edit-profile-btn" title="Edit">Edit</button>
        <button class="delete-profile-btn danger" title="Delete">Del</button>
      </div>
    `;

    card.addEventListener('click', (e) => {
      if (e.target.closest('.profile-card-actions')) return;
      selectProfile(profile.id);
    });

    card.querySelector('.edit-profile-btn').addEventListener('click', async (e) => {
      e.stopPropagation();
      await editProfile(profile);
    });

    card.querySelector('.delete-profile-btn').addEventListener('click', async (e) => {
      e.stopPropagation();
      await deleteProfile(profile.id);
    });

    list.appendChild(card);
  }
}

export function selectProfile(id) {
  _selectedProfileId = id;
  renderProfiles();
  const profile = getSelectedProfile();
  updateContentHeader(profile);
  renderSteps();

  const launchBtn = document.getElementById('btn-launch');
  const addStepBtn = document.getElementById('btn-add-step');
  launchBtn.disabled = !profile;
  addStepBtn.disabled = !profile;
}

function updateContentHeader(profile) {
  const nameEl = document.getElementById('profile-name');
  const descEl = document.getElementById('profile-desc');
  if (profile) {
    nameEl.textContent = profile.name;
    descEl.textContent = profile.description || '';
  } else {
    nameEl.textContent = 'No profile selected';
    descEl.textContent = '';
  }
}

export async function addProfile() {
  const profile = newProfile();
  const result = await showProfileEditor(profile, true);
  if (!result) return;

  const config = getConfig();
  config.profiles.push(result);
  await saveConfig(config);
  selectProfile(result.id);
  renderProfiles();
}

async function editProfile(profile) {
  const result = await showProfileEditor({ ...profile }, false);
  if (!result) return;

  const config = getConfig();
  const idx = config.profiles.findIndex(p => p.id === profile.id);
  if (idx === -1) return;

  config.profiles[idx].name = result.name;
  config.profiles[idx].description = result.description;
  await saveConfig(config);
  renderProfiles();
  updateContentHeader(config.profiles[idx]);
}

async function deleteProfile(id) {
  const { showConfirm } = await import('./dialogs.js');
  const ok = await showConfirm('Delete this profile?', 'This cannot be undone.');
  if (!ok) return;

  const config = getConfig();
  config.profiles = config.profiles.filter(p => p.id !== id);
  await saveConfig(config);

  if (_selectedProfileId === id) {
    _selectedProfileId = config.profiles.length > 0 ? config.profiles[0].id : null;
  }
  renderProfiles();
  selectProfile(_selectedProfileId);
}

function escapeHtml(str) {
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}
