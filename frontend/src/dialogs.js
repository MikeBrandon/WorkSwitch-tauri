const { invoke } = window.__TAURI__.core;

function getOverlay() { return document.getElementById('modal-overlay'); }
function getContent() { return document.getElementById('modal-content'); }

function showModal(html) {
  getContent().innerHTML = html;
  getOverlay().classList.remove('hidden');
}

function hideModal() {
  getOverlay().classList.add('hidden');
  getContent().innerHTML = '';
}

// ── Confirm dialog ──
export function showConfirm(title, message) {
  return new Promise((resolve) => {
    showModal(`
      <div class="modal-title">${escapeHtml(title)}</div>
      <p style="color: var(--text-secondary); margin-bottom: 8px;">${escapeHtml(message || '')}</p>
      <div class="modal-actions">
        <button class="btn-secondary" id="confirm-no">Cancel</button>
        <button class="btn-danger" id="confirm-yes">Delete</button>
      </div>
    `);
    document.getElementById('confirm-no').addEventListener('click', () => { hideModal(); resolve(false); });
    document.getElementById('confirm-yes').addEventListener('click', () => { hideModal(); resolve(true); });
  });
}

// ── Profile editor ──
export function showProfileEditor(profile, isNew) {
  return new Promise((resolve) => {
    const tags = (profile.tags || []).join(', ');
    const schedule = profile.schedule || { enabled: false, time: '09:00', days: [] };
    const dayNames = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
    const dayChecks = dayNames.map((d, i) =>
      `<label class="day-check"><input type="checkbox" class="pe-day" value="${i}" ${schedule.days && schedule.days.includes(i) ? 'checked' : ''}>${d}</label>`
    ).join('');

    showModal(`
      <div class="modal-title">${isNew ? 'New Profile' : 'Edit Profile'}</div>
      <div class="form-group">
        <label>Name</label>
        <input type="text" id="pe-name" value="${escapeAttr(profile.name)}" placeholder="Profile name">
      </div>
      <div class="form-group">
        <label>Description</label>
        <input type="text" id="pe-desc" value="${escapeAttr(profile.description || '')}" placeholder="Optional description">
      </div>
      <div class="form-group">
        <label>Tags (comma separated)</label>
        <input type="text" id="pe-tags" value="${escapeAttr(tags)}" placeholder="Work, Dev, Gaming">
      </div>
      <div class="form-group">
        <label>Hotkey (e.g. Ctrl+Shift+1)</label>
        <input type="text" id="pe-hotkey" value="${escapeAttr(profile.hotkey || '')}" placeholder="Click and press keys..." readonly>
      </div>
      <div class="settings-section" style="margin-top:12px">
        <h3>Schedule</h3>
        <div class="form-check">
          <input type="checkbox" id="pe-sched-enabled" ${schedule.enabled ? 'checked' : ''}>
          <label for="pe-sched-enabled">Auto-launch on schedule</label>
        </div>
        <div class="form-row">
          <div class="form-group">
            <label>Time</label>
            <input type="time" id="pe-sched-time" value="${escapeAttr(schedule.time || '09:00')}">
          </div>
          <div class="form-group">
            <label>Days</label>
            <div class="day-checks">${dayChecks}</div>
          </div>
        </div>
      </div>
      <div class="modal-actions">
        <button class="btn-secondary" id="pe-cancel">Cancel</button>
        <button class="btn-primary" id="pe-save">Save</button>
      </div>
    `);

    // Hotkey capture
    const hotkeyInput = document.getElementById('pe-hotkey');
    hotkeyInput.addEventListener('keydown', (e) => {
      e.preventDefault();
      const parts = [];
      if (e.ctrlKey) parts.push('Ctrl');
      if (e.altKey) parts.push('Alt');
      if (e.shiftKey) parts.push('Shift');
      if (e.metaKey) parts.push('Super');
      const key = e.key;
      if (!['Control', 'Alt', 'Shift', 'Meta'].includes(key)) {
        parts.push(key.length === 1 ? key.toUpperCase() : key);
      }
      if (parts.length > 0) hotkeyInput.value = parts.join('+');
    });
    hotkeyInput.addEventListener('click', () => hotkeyInput.value = '');

    document.getElementById('pe-name').focus();
    document.getElementById('pe-cancel').addEventListener('click', () => { hideModal(); resolve(null); });
    document.getElementById('pe-save').addEventListener('click', () => {
      profile.name = document.getElementById('pe-name').value.trim() || 'Unnamed';
      profile.description = document.getElementById('pe-desc').value.trim();
      profile.tags = document.getElementById('pe-tags').value.split(',').map(t => t.trim()).filter(Boolean);
      profile.hotkey = document.getElementById('pe-hotkey').value.trim();

      const schedEnabled = document.getElementById('pe-sched-enabled').checked;
      const schedTime = document.getElementById('pe-sched-time').value || '09:00';
      const schedDays = [...document.querySelectorAll('.pe-day:checked')].map(cb => parseInt(cb.value));

      if (schedEnabled || schedTime !== '09:00' || schedDays.length > 0) {
        profile.schedule = { enabled: schedEnabled, time: schedTime, days: schedDays };
      } else {
        profile.schedule = null;
      }

      hideModal();
      resolve(profile);
    });
    document.getElementById('pe-name').addEventListener('keydown', (e) => {
      if (e.key === 'Enter') document.getElementById('pe-save').click();
    });
  });
}

// ── Step editor ──
export function showStepEditor(step, isNew) {
  return new Promise((resolve) => {
    const typeOptions = ['app', 'terminal', 'folder', 'url']
      .map(t => `<option value="${t}" ${step.type === t ? 'selected' : ''}>${t === 'terminal' ? 'Terminal (CMD)' : t.charAt(0).toUpperCase() + t.slice(1)}</option>`)
      .join('');

    showModal(`
      <div class="modal-title">${isNew ? 'New Step' : 'Edit Step'}</div>
      <div class="form-row">
        <div class="form-group">
          <label>Name</label>
          <input type="text" id="se-name" value="${escapeAttr(step.name)}" placeholder="Step name">
        </div>
        <div class="form-group" style="max-width:160px">
          <label>Type</label>
          <select id="se-type">${typeOptions}</select>
        </div>
      </div>
      <div id="se-fields"></div>
      <div class="form-row">
        <div class="form-group">
          <label>Delay after (ms)</label>
          <input type="number" id="se-delay" value="${step.delay_after || 500}" min="0" step="100">
        </div>
        <div class="form-group">
          <label>Process name</label>
          <input type="text" id="se-process" value="${escapeAttr(step.process_name || '')}" placeholder="e.g. chrome.exe">
        </div>
      </div>
      <div class="modal-actions">
        <button class="btn-secondary" id="se-cancel">Cancel</button>
        <button class="btn-primary" id="se-save">Save</button>
      </div>
    `);

    const typeSelect = document.getElementById('se-type');
    renderStepFields(step);

    typeSelect.addEventListener('change', () => {
      step.type = typeSelect.value;
      renderStepFields(step);
    });

    document.getElementById('se-name').focus();
    document.getElementById('se-cancel').addEventListener('click', () => { hideModal(); resolve(null); });
    document.getElementById('se-save').addEventListener('click', () => {
      step.name = document.getElementById('se-name').value.trim() || 'Unnamed';
      step.type = document.getElementById('se-type').value;
      step.delay_after = parseInt(document.getElementById('se-delay').value) || 500;
      step.process_name = document.getElementById('se-process').value.trim();
      readStepFields(step);
      hideModal();
      resolve(step);
    });
  });
}

function renderStepFields(step) {
  const container = document.getElementById('se-fields');
  const type = document.getElementById('se-type').value;

  switch (type) {
    case 'app':
      container.innerHTML = `
        <div class="form-group">
          <label>Target (path, URI, or command)</label>
          <div class="browse-row">
            <input type="text" id="se-target" value="${escapeAttr(step.target || '')}" placeholder="C:\\path\\app.exe or spotify:">
            <button class="browse-btn" id="se-browse-file">Browse</button>
            <button class="browse-btn" id="se-pick-app">Pick App</button>
          </div>
        </div>
        <div class="form-check">
          <input type="checkbox" id="se-check-running" ${step.check_running !== false ? 'checked' : ''}>
          <label for="se-check-running">Skip if already running</label>
        </div>
      `;
      document.getElementById('se-browse-file').addEventListener('click', async () => {
        try {
          const path = await invoke('browse_file');
          if (path) document.getElementById('se-target').value = path;
        } catch (e) { console.error(e); }
      });
      document.getElementById('se-pick-app').addEventListener('click', async () => {
        const picked = await showAppPicker();
        if (picked) {
          document.getElementById('se-target').value = picked.target;
          document.getElementById('se-name').value = picked.name;
          document.getElementById('se-process').value = picked.process_name;
        }
      });
      break;

    case 'terminal':
      container.innerHTML = `
        <div class="form-group">
          <label>Command</label>
          <input type="text" id="se-command" value="${escapeAttr(step.command || '')}" placeholder="npm run dev">
        </div>
        <div class="form-group">
          <label>Working Directory</label>
          <div class="browse-row">
            <input type="text" id="se-workdir" value="${escapeAttr(step.working_dir || '')}" placeholder="C:\\project">
            <button class="browse-btn" id="se-browse-dir">Browse</button>
          </div>
        </div>
        <div class="form-check">
          <input type="checkbox" id="se-keep-open" ${step.keep_open !== false ? 'checked' : ''}>
          <label for="se-keep-open">Keep terminal open</label>
        </div>
      `;
      document.getElementById('se-browse-dir').addEventListener('click', async () => {
        try {
          const path = await invoke('browse_folder');
          if (path) document.getElementById('se-workdir').value = path;
        } catch (e) { console.error(e); }
      });
      break;

    case 'folder':
      container.innerHTML = `
        <div class="form-group">
          <label>Folder Path</label>
          <div class="browse-row">
            <input type="text" id="se-target" value="${escapeAttr(step.target || '')}" placeholder="%USERPROFILE%\\Downloads">
            <button class="browse-btn" id="se-browse-folder">Browse</button>
          </div>
        </div>
      `;
      document.getElementById('se-browse-folder').addEventListener('click', async () => {
        try {
          const path = await invoke('browse_folder');
          if (path) document.getElementById('se-target').value = path;
        } catch (e) { console.error(e); }
      });
      break;

    case 'url':
      container.innerHTML = `
        <div class="form-group">
          <label>URL</label>
          <input type="text" id="se-target" value="${escapeAttr(step.target || '')}" placeholder="https://example.com">
        </div>
      `;
      break;
  }
}

function readStepFields(step) {
  const type = step.type;

  // Clean up fields from other types
  delete step.target;
  delete step.check_running;
  delete step.command;
  delete step.working_dir;
  delete step.keep_open;

  switch (type) {
    case 'app': {
      const target = document.getElementById('se-target');
      const checkRunning = document.getElementById('se-check-running');
      step.target = target ? target.value.trim() : '';
      step.check_running = checkRunning ? checkRunning.checked : true;
      break;
    }
    case 'terminal': {
      const command = document.getElementById('se-command');
      const workdir = document.getElementById('se-workdir');
      const keepOpen = document.getElementById('se-keep-open');
      step.command = command ? command.value.trim() : '';
      step.working_dir = workdir ? workdir.value.trim() : '';
      step.keep_open = keepOpen ? keepOpen.checked : true;
      break;
    }
    case 'folder':
    case 'url': {
      const target = document.getElementById('se-target');
      step.target = target ? target.value.trim() : '';
      break;
    }
  }
}

// ── Settings dialog ──
export function showSettings(settings) {
  return new Promise((resolve) => {
    showModal(`
      <div class="modal-title">Settings</div>
      <div class="settings-section">
        <div class="form-group">
          <label>Default launch delay (ms)</label>
          <input type="number" id="set-delay" value="${settings.launch_delay_ms || 500}" min="0" step="100">
        </div>
        <div class="form-check">
          <input type="checkbox" id="set-minimized" ${settings.start_minimized ? 'checked' : ''}>
          <label for="set-minimized">Start minimized</label>
        </div>
        <div class="form-check">
          <input type="checkbox" id="set-tray" ${settings.minimize_to_tray !== false ? 'checked' : ''}>
          <label for="set-tray">Minimize to tray on close</label>
        </div>
        <div class="form-check">
          <input type="checkbox" id="set-close-switch" ${settings.close_on_switch !== false ? 'checked' : ''}>
          <label for="set-close-switch">Offer to close apps when switching profiles</label>
        </div>
        <div class="form-check">
          <input type="checkbox" id="set-autostart" ${settings.auto_start_with_windows ? 'checked' : ''}>
          <label for="set-autostart">Launch with Windows</label>
        </div>
      </div>
      <div class="modal-actions">
        <button class="btn-secondary" id="set-cancel">Cancel</button>
        <button class="btn-primary" id="set-save">Save</button>
      </div>
    `);

    document.getElementById('set-cancel').addEventListener('click', () => { hideModal(); resolve(null); });
    document.getElementById('set-save').addEventListener('click', async () => {
      const autoStart = document.getElementById('set-autostart').checked;
      // Update Windows registry for auto-start
      try {
        await invoke('set_auto_start', { enabled: autoStart });
      } catch (e) {
        console.error('Failed to set auto-start:', e);
      }
      const result = {
        ...settings,
        launch_delay_ms: parseInt(document.getElementById('set-delay').value) || 500,
        start_minimized: document.getElementById('set-minimized').checked,
        minimize_to_tray: document.getElementById('set-tray').checked,
        close_on_switch: document.getElementById('set-close-switch').checked,
        auto_start_with_windows: autoStart
      };
      hideModal();
      resolve(result);
    });
  });
}

// ── Close-on-switch dialog ──
export function showCloseOnSwitch(processes) {
  return new Promise((resolve) => {
    const items = processes.map((p, i) => `
      <li class="close-process-item">
        <input type="checkbox" id="cos-${i}" checked>
        <label for="cos-${i}">${escapeHtml(p)}</label>
      </li>
    `).join('');

    showModal(`
      <div class="modal-title">Close Running Apps?</div>
      <p style="color: var(--text-secondary); margin-bottom: 4px;">The following apps from the previous profile are still running:</p>
      <ul class="close-process-list">${items}</ul>
      <div class="modal-actions">
        <button class="btn-secondary" id="cos-skip">Skip</button>
        <button class="btn-primary" id="cos-close">Close Selected</button>
      </div>
    `);

    document.getElementById('cos-skip').addEventListener('click', () => { hideModal(); resolve([]); });
    document.getElementById('cos-close').addEventListener('click', () => {
      const toClose = [];
      processes.forEach((p, i) => {
        if (document.getElementById(`cos-${i}`).checked) toClose.push(p);
      });
      hideModal();
      resolve(toClose);
    });
  });
}

// ── Launch history dialog ──
export function showLaunchHistory(history) {
  return new Promise((resolve) => {
    let rows = '';
    if (!history || history.length === 0) {
      rows = '<div class="process-empty" style="padding:20px 0">No launch history yet.</div>';
    } else {
      rows = [...history].reverse().slice(0, 50).map(h => {
        const date = new Date(h.timestamp);
        const dateStr = date.toLocaleDateString() + ' ' + date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
        const statusIcon = h.success ? '<span style="color:var(--success)">&#10003;</span>' : '<span style="color:var(--danger)">&#10007;</span>';
        return `
          <div class="history-item">
            ${statusIcon}
            <span class="history-name">${escapeHtml(h.profile_name)}</span>
            <span class="history-stats">${h.steps_launched} launched${h.steps_failed > 0 ? ', ' + h.steps_failed + ' failed' : ''}</span>
            <span class="history-date">${escapeHtml(dateStr)}</span>
          </div>
        `;
      }).join('');
    }

    showModal(`
      <div class="modal-title">Launch History</div>
      <div style="max-height:350px;overflow-y:auto">${rows}</div>
      <div class="modal-actions">
        ${history && history.length > 0 ? '<button class="btn-secondary" id="hist-clear">Clear History</button>' : ''}
        <button class="btn-primary" id="hist-close">Close</button>
      </div>
    `);

    document.getElementById('hist-close').addEventListener('click', () => { hideModal(); resolve(false); });
    const clearBtn = document.getElementById('hist-clear');
    if (clearBtn) {
      clearBtn.addEventListener('click', () => { hideModal(); resolve(true); });
    }
  });
}

// ── Duplicate name prompt ──
export function showDuplicateNamePrompt(defaultName) {
  return new Promise((resolve) => {
    showModal(`
      <div class="modal-title">Duplicate Step</div>
      <div class="form-group">
        <label>Name for duplicate</label>
        <input type="text" id="dup-name" value="${escapeAttr(defaultName)}" placeholder="Step name">
      </div>
      <div class="modal-actions">
        <button class="btn-secondary" id="dup-cancel">Cancel</button>
        <button class="btn-primary" id="dup-ok">Duplicate</button>
      </div>
    `);

    const input = document.getElementById('dup-name');
    input.focus();
    input.select();
    document.getElementById('dup-cancel').addEventListener('click', () => { hideModal(); resolve(null); });
    document.getElementById('dup-ok').addEventListener('click', () => {
      const name = input.value.trim() || defaultName;
      hideModal();
      resolve(name);
    });
    input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') document.getElementById('dup-ok').click();
    });
  });
}

// ── App picker ──
function showAppPicker() {
  return new Promise((resolve) => {
    const content = getContent();

    // Hide existing step editor children (preserves their event listeners)
    const originalChildren = [...content.children];
    originalChildren.forEach(ch => ch.style.display = 'none');

    // Build picker wrapper and append it
    const picker = document.createElement('div');
    picker.id = 'app-picker-wrapper';
    picker.innerHTML = `
      <div class="modal-title">Pick an Application</div>
      <div class="form-group" style="margin-bottom:8px">
        <input type="text" id="app-picker-search" class="app-picker-search" placeholder="Search apps...">
      </div>
      <div id="app-picker-body" class="app-picker-list">
        <div class="app-picker-loading">Scanning installed apps...</div>
      </div>
      <div class="modal-actions">
        <button class="btn-secondary" id="app-picker-cancel">Cancel</button>
      </div>
    `;
    content.appendChild(picker);

    const closePicker = () => {
      picker.remove();
      originalChildren.forEach(ch => ch.style.display = '');
    };

    document.getElementById('app-picker-cancel').addEventListener('click', () => {
      closePicker();
      resolve(null);
    });
    document.getElementById('app-picker-search').focus();

    // Scan apps
    invoke('scan_apps').then((apps) => {
      const body = document.getElementById('app-picker-body');
      if (!body) return; // picker was closed

      if (apps.length === 0) {
        body.innerHTML = '<div class="app-picker-empty">No applications found</div>';
        return;
      }

      // Group by source
      const groups = {};
      const sourceOrder = ['steam', 'epic', 'windows'];
      const sourceLabels = { steam: 'Steam', epic: 'Epic Games', windows: 'Installed' };

      for (const app of apps) {
        if (!groups[app.source]) groups[app.source] = [];
        groups[app.source].push(app);
      }

      const renderList = (filter) => {
        const lowerFilter = (filter || '').toLowerCase();
        let html = '';

        for (const src of sourceOrder) {
          const items = groups[src];
          if (!items) continue;

          const filtered = lowerFilter
            ? items.filter(a => a.name.toLowerCase().includes(lowerFilter))
            : items;

          if (filtered.length === 0) continue;

          html += '<div class="app-picker-group-header">' + escapeHtml(sourceLabels[src] || src) + '</div>';
          for (let i = 0; i < filtered.length; i++) {
            const a = filtered[i];
            html += '<div class="app-picker-item" data-source="' + escapeAttr(a.source) + '" data-idx="' + escapeAttr(a.name) + '">'
              + '<span class="app-picker-item-name">' + escapeHtml(a.name) + '</span>'
              + '<span class="app-picker-item-badge badge-' + escapeAttr(a.source) + '">' + escapeHtml(a.source) + '</span>'
              + '</div>';
          }
        }

        if (!html) {
          html = '<div class="app-picker-empty">No matching apps</div>';
        }

        body.innerHTML = html;

        // Attach click handlers
        body.querySelectorAll('.app-picker-item').forEach(el => {
          el.addEventListener('click', () => {
            const name = el.getAttribute('data-idx');
            const source = el.getAttribute('data-source');
            const app = apps.find(a => a.name === name && a.source === source);
            if (app) {
              closePicker();
              resolve({ name: app.name, target: app.target, process_name: app.process_name });
            }
          });
        });
      };

      renderList('');

      const searchInput = document.getElementById('app-picker-search');
      if (searchInput) {
        searchInput.addEventListener('input', () => renderList(searchInput.value));
      }
    }).catch(() => {
      const body = document.getElementById('app-picker-body');
      if (body) body.innerHTML = '<div class="app-picker-empty">Failed to scan apps</div>';
    });
  });
}

function escapeHtml(str) {
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

function escapeAttr(str) {
  return str.replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/'/g, '&#39;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}
