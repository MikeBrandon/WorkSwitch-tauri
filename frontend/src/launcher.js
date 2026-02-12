const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

let _launching = false;
let _progressUnlisten = null;
let _completeUnlisten = null;
let _cancelledUnlisten = null;
let _errorUnlisten = null;

export function isLaunching() {
  return _launching;
}

export async function startLaunch(steps, defaultDelay) {
  if (_launching) return;
  _launching = true;

  const enabledSteps = steps.filter(s => s.enabled);
  if (enabledSteps.length === 0) {
    setStatus('No enabled steps to launch');
    _launching = false;
    return;
  }

  updateLaunchUI(true);
  setStatus('Launching...');
  showProgress(true);
  setProgress(0);

  // Listen for progress events
  _progressUnlisten = await listen('launch-progress', (event) => {
    const { step_name, current, total } = event.payload;
    setStatus(`Launching: ${step_name} (${current}/${total})`);
    setProgress((current / total) * 100);

    // Highlight the current step card
    highlightStep(current - 1, 'launching');
    if (current > 1) highlightStep(current - 2, 'launched');
  });

  _completeUnlisten = await listen('launch-complete', () => {
    cleanup();
    setStatus('Launch complete');
    // Mark last step as launched
    const cards = document.querySelectorAll('.step-card');
    if (cards.length > 0) {
      cards.forEach(c => { c.classList.remove('launching'); c.classList.add('launched'); });
    }
    setTimeout(() => {
      clearStepHighlights();
      setStatus('');
      showProgress(false);
    }, 2000);
  });

  _cancelledUnlisten = await listen('launch-cancelled', () => {
    cleanup();
    setStatus('Launch cancelled');
    clearStepHighlights();
    setTimeout(() => {
      setStatus('');
      showProgress(false);
    }, 1500);
  });

  _errorUnlisten = await listen('launch-step-error', (event) => {
    const { step_name, error } = event.payload;
    setStatus(`Warning: ${step_name} failed (${error}) - continuing...`);
  });

  // Don't await - the invoke resolves when launch is done, but events handle UI updates.
  // We catch errors separately so the UI never gets stuck.
  invoke('launch_profile', { steps: enabledSteps, defaultDelay }).catch((err) => {
    cleanup();
    setStatus('Launch error: ' + err);
    showProgress(false);
    clearStepHighlights();
  });
}

export async function cancelLaunch() {
  if (!_launching) return;
  try {
    await invoke('cancel_launch');
  } catch (e) {
    console.error('Cancel error:', e);
  }
}

function cleanup() {
  _launching = false;
  updateLaunchUI(false);
  if (_progressUnlisten) { _progressUnlisten(); _progressUnlisten = null; }
  if (_completeUnlisten) { _completeUnlisten(); _completeUnlisten = null; }
  if (_cancelledUnlisten) { _cancelledUnlisten(); _cancelledUnlisten = null; }
  if (_errorUnlisten) { _errorUnlisten(); _errorUnlisten = null; }
}

function updateLaunchUI(launching) {
  const launchBtn = document.getElementById('btn-launch');
  const cancelBtn = document.getElementById('btn-cancel');
  launchBtn.style.display = launching ? 'none' : '';
  cancelBtn.style.display = launching ? '' : 'none';
}

function setStatus(text) {
  document.getElementById('status-text').textContent = text;
}

function showProgress(show) {
  document.getElementById('progress-bar-container').style.display = show ? '' : 'none';
}

function setProgress(pct) {
  document.getElementById('progress-bar').style.width = pct + '%';
}

function highlightStep(index, cls) {
  const cards = document.querySelectorAll('.step-card');
  if (cards[index]) {
    cards[index].classList.remove('launching', 'launched');
    cards[index].classList.add(cls);
  }
}

function clearStepHighlights() {
  document.querySelectorAll('.step-card').forEach(c => {
    c.classList.remove('launching', 'launched');
  });
}
