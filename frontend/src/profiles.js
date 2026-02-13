import { getConfig, saveConfig, newProfile, generateId } from './config.js';
import { renderSteps } from './steps.js';
import { showProfileEditor } from './dialogs.js';

const { invoke } = window.__TAURI__.core;

let _selectedProfileId = null;
let _tagFilter = '';

export function getSelectedProfileId() {
  return _selectedProfileId;
}

export function getSelectedProfile() {
  const config = getConfig();
  if (!config || !_selectedProfileId) return null;
  return config.profiles.find(p => p.id === _selectedProfileId) || null;
}

export function setTagFilter(tag) {
  _tagFilter = tag;
  renderProfiles();
}

export function getTagFilter() {
  return _tagFilter;
}

export function renderProfiles() {
  const config = getConfig();
  const list = document.getElementById('profile-list');

  if (!config || config.profiles.length === 0) {
    list.innerHTML = '<div class="empty-state"><span class="empty-state-icon">&#128203;</span><span>No profiles</span></div>';
    updateContentHeader(null);
    return;
  }

  // Collect all tags for filter
  const allTags = new Set();
  for (const p of config.profiles) {
    if (p.tags) p.tags.forEach(t => allTags.add(t));
  }

  list.innerHTML = '';

  // Tag filter bar
  if (allTags.size > 0) {
    const filterBar = document.createElement('div');
    filterBar.className = 'tag-filter-bar';
    filterBar.innerHTML = `<span class="tag-pill clickable ${!_tagFilter ? 'active' : ''}" data-tag="">All</span>` +
      [...allTags].map(t =>
        `<span class="tag-pill clickable ${_tagFilter === t ? 'active' : ''}" data-tag="${escapeAttr(t)}">${escapeHtml(t)}</span>`
      ).join('');
    filterBar.querySelectorAll('.tag-pill').forEach(pill => {
      pill.addEventListener('click', () => {
        _tagFilter = pill.dataset.tag;
        renderProfiles();
      });
    });
    list.appendChild(filterBar);
  }

  const filtered = _tagFilter
    ? config.profiles.filter(p => p.tags && p.tags.includes(_tagFilter))
    : config.profiles;

  for (const profile of filtered) {
    const card = document.createElement('div');
    card.className = 'profile-card' + (profile.id === _selectedProfileId ? ' active' : '');

    const tagHtml = (profile.tags || []).map(t => `<span class="tag-pill small">${escapeHtml(t)}</span>`).join('');
    const hotkeyHtml = profile.hotkey ? `<span class="profile-hotkey">${escapeHtml(profile.hotkey)}</span>` : '';
    const scheduleIcon = profile.schedule && profile.schedule.enabled ? ' <span title="Scheduled" style="font-size:11px">&#128339;</span>' : '';

    card.innerHTML = `
      <div class="profile-card-name">${escapeHtml(profile.name)}${scheduleIcon}</div>
      <div class="profile-card-desc">${escapeHtml(profile.description || '')}</div>
      ${tagHtml ? '<div class="profile-card-tags">' + tagHtml + '</div>' : ''}
      <div class="profile-card-meta">
        <span class="profile-card-count">${profile.steps.length} step${profile.steps.length !== 1 ? 's' : ''}</span>
        ${hotkeyHtml}
      </div>
      <div class="profile-card-actions">
        <button class="edit-profile-btn" title="Edit">Edit</button>
        <button class="export-profile-btn" title="Export">Exp</button>
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

    card.querySelector('.export-profile-btn').addEventListener('click', async (e) => {
      e.stopPropagation();
      await exportProfile(profile);
    });

    card.querySelector('.delete-profile-btn').addEventListener('click', async (e) => {
      e.stopPropagation();
      await deleteProfile(profile.id);
    });

    list.appendChild(card);
  }
}

function escapeAttr(str) {
  return str.replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/'/g, '&#39;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
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

  config.profiles[idx] = { ...config.profiles[idx], ...result };
  await saveConfig(config);
  renderProfiles();
  updateContentHeader(config.profiles[idx]);
}

async function exportProfile(profile) {
  try {
    // Use backend browse_folder to pick a directory, then save there
    const path = await invoke('browse_save_profile', {
      defaultName: profile.name.replace(/[^a-zA-Z0-9]/g, '_') + '.json'
    });
    if (path) {
      await invoke('save_profile_file', { profileId: profile.id, path });
    }
  } catch (e) {
    console.error('Export failed:', e);
  }
}

export async function importProfile() {
  try {
    const path = await invoke('browse_import_profile');
    if (!path) return;

    const profile = await invoke('load_profile_file', { path });
    // Give it a new ID so it doesn't collide
    profile.id = generateId();
    // Rename steps IDs too
    for (const step of profile.steps) {
      step.id = generateId();
    }

    const config = getConfig();
    config.profiles.push(profile);
    await saveConfig(config);
    selectProfile(profile.id);
    renderProfiles();
  } catch (e) {
    console.error('Import failed:', e);
  }
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
