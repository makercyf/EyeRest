const appEl = document.getElementById("app");
const view = new URLSearchParams(location.search).get("view") || "settings";
const tauriApi = window.__TAURI__;
const invoke = (command, args) => tauriApi.core.invoke(command, args);

const WORK_RING_CIRCUMFERENCE = 2 * Math.PI * 96;
const REST_RING_CIRCUMFERENCE = 2 * Math.PI * 76;
const DEFAULT_OVERLAY_THEME = Object.freeze({
  id: "default-dark",
  name: "Default Dark",
  background: "#111111",
  textColor: "#ffffff",
  mutedTextColor: "#c8c8c8",
  accentColor: "#62d26f",
  dangerColor: "#f05d5e",
  buttonBackground: "#ffffff",
  buttonTextColor: "#111111",
  buttonRadius: 12,
  fontFamily: "Inter, Segoe UI, sans-serif",
  backgroundImage: null,
  layout: "centered",
});

let currentSettings = null;
let currentStatus = null;
let availableMonitors = [];
let statusCapturedAt = Date.now();
let toastTimeout = null;

function secondsToMinutes(seconds) {
  return Math.max(1, Math.round(seconds / 60));
}

function minutesToSeconds(minutes) {
  return Math.max(1, Number(minutes) || 1) * 60;
}

function formatDuration(seconds) {
  const wholeSeconds = Math.max(0, Math.floor(seconds || 0));
  const minutes = Math.floor(wholeSeconds / 60);
  const remainder = wholeSeconds % 60;
  return `${String(minutes).padStart(2, "0")}:${String(remainder).padStart(2, "0")}`;
}

function statusDetail(status) {
  const elapsedSeconds = Math.floor((Date.now() - statusCapturedAt) / 1000);
  if (status.state === "Working") {
    return `Next reminder in ${formatDuration((status.workRemainingSeconds || 0) - elapsedSeconds)}`;
  }
  if (status.state === "Resting") {
    return `Rest ends in ${formatDuration((status.restRemainingSeconds || 0) - elapsedSeconds)}`;
  }
  if (status.state === "PausedByUser") return "Reminders are paused";
  if (status.state === "IdleSuspended") return "Timer paused while idle";
  if (status.state === "ReminderPending") return "Reminder is waiting";
  if (status.state === "ReminderShown") return "Reminder needs a response";
  return "Preparing timer";
}

function humanizeState(state) {
  const map = {
    Working: "Working",
    Resting: "Resting",
    PausedByUser: "Paused",
    IdleSuspended: "Idle",
    ReminderPending: "Pending",
    ReminderShown: "Reminder",
  };
  return map[state] || state || "Working";
}

function heroCountdownDisplay(status) {
  const elapsedSeconds = Math.floor((Date.now() - statusCapturedAt) / 1000);
  if (status.state === "Working") {
    return formatDuration((status.workRemainingSeconds || 0) - elapsedSeconds);
  }
  if (status.state === "Resting") {
    return formatDuration((status.restRemainingSeconds || 0) - elapsedSeconds);
  }
  if (status.state === "PausedByUser") return "II";
  if (status.state === "IdleSuspended") return "\u2026";
  return "--:--";
}

function clamp01(value) {
  return Math.max(0, Math.min(1, value || 0));
}

function setArcFraction(element, circumference, fraction) {
  if (!element) return;
  element.style.strokeDasharray = `${circumference}`;
  element.style.strokeDashoffset = `${circumference * (1 - fraction)}`;
}

function updateHeroProgress(status) {
  const workArc = document.getElementById("heroWorkArc");
  const restArc = document.getElementById("heroRestArc");
  if (!workArc || !restArc || !currentSettings || !status) return;

  const workTotal = Math.max(1, currentSettings.workIntervalSeconds || 1);
  const restTotal = Math.max(1, currentSettings.restDurationSeconds || 1);
  const elapsedSeconds = Math.floor((Date.now() - statusCapturedAt) / 1000);

  let workFraction = 0;
  let restFraction = 0;

  if (status.state === "Working") {
    const remaining = Math.max(0, (status.workRemainingSeconds || 0) - elapsedSeconds);
    workFraction = 1 - remaining / workTotal;
  } else if (status.state === "Resting") {
    workFraction = 1;
    const remaining = Math.max(0, (status.restRemainingSeconds || 0) - elapsedSeconds);
    restFraction = 1 - remaining / restTotal;
  } else if (status.state === "ReminderPending" || status.state === "ReminderShown") {
    workFraction = 1;
  }

  setArcFraction(workArc, WORK_RING_CIRCUMFERENCE, clamp01(workFraction));
  setArcFraction(restArc, REST_RING_CIRCUMFERENCE, clamp01(restFraction));
}

function updateStatusSummary() {
  const state = document.getElementById("statusState");
  const timer = document.getElementById("statusTimer");
  const heroCountdown = document.getElementById("heroCountdown");
  if (!currentStatus) return;
  if (state) state.textContent = humanizeState(currentStatus.state);
  if (timer) timer.textContent = statusDetail(currentStatus);
  if (heroCountdown) heroCountdown.textContent = heroCountdownDisplay(currentStatus);
  const workLegend = document.getElementById("workMinutesLegend");
  const restLegend = document.getElementById("restSecondsLegend");
  if (workLegend && currentSettings) workLegend.textContent = secondsToMinutes(currentSettings.workIntervalSeconds);
  if (restLegend && currentSettings) restLegend.textContent = currentSettings.restDurationSeconds;
  updateHeroProgress(currentStatus);
}

function setActionMessage(text, error = false) {
  showToast(text, error);
}

function showToast(text, error = false) {
  const toast = document.getElementById("toast");
  if (!toast || !text) return;
  toast.textContent = String(text).replace(/^\s*([a-z])/, (_, letter) => letter.toUpperCase());
  toast.classList.toggle("error", error);
  toast.classList.add("visible");
  if (toastTimeout) clearTimeout(toastTimeout);
  toastTimeout = setTimeout(() => toast.classList.remove("visible"), 3200);
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function whitelistText(settings) {
  return (settings.whitelistedProcesses || []).join("\n");
}

function monitorRows(settings) {
  if (!availableMonitors.length) {
    return `<div class="monitor-item">No monitors were reported by Windows.</div>`;
  }

  return availableMonitors
    .map((monitor) => {
      const checked = settings.monitorMode === "all" || settings.selectedMonitorIds.includes(monitor.id);
      const disabled = settings.monitorMode === "all" ? "disabled" : "";
      const primary = monitor.isPrimary ? " Primary" : "";
      return `
        <label class="monitor-item">
          <input class="monitorChoice" type="checkbox" value="${escapeHtml(monitor.id)}" ${checked ? "checked" : ""} ${disabled} />
          ${escapeHtml(monitor.name)} (${monitor.width}x${monitor.height}${primary})
        </label>
      `;
    })
    .join("");
}

function themeControl(id, label, type = "color") {
  const theme = currentSettings.theme;
  return `
    <label>
      ${label}
      <input id="${id}" type="${type}" value="${escapeHtml(theme[id])}" />
    </label>
  `;
}

function renderSettings() {
  const status = currentStatus || {};
  const settings = currentSettings;
  appEl.innerHTML = `
    <div class="page">
      <div class="wrap">
        <header class="masthead">
          <div class="wordmark">EyeRest<span class="wordmark-dot">.</span></div>
          <p class="tagline">Every 20 minutes, look 20 feet away for 20 seconds.</p>
        </header>

        <section class="hero" aria-live="polite">
          <div class="hero-ring">
            <svg viewBox="0 0 220 220" role="img" aria-label="Reminder cycle progress">
              <circle class="ring-track" cx="110" cy="110" r="96"></circle>
              <circle class="ring-track" cx="110" cy="110" r="76"></circle>
              <circle id="heroWorkArc" class="ring-arc ring-arc--work" cx="110" cy="110" r="96" transform="rotate(-90 110 110)"></circle>
              <circle id="heroRestArc" class="ring-arc ring-arc--rest" cx="110" cy="110" r="76" transform="rotate(-90 110 110)"></circle>
            </svg>
            <div class="hero-center">
              <div class="hero-label" id="statusState">${escapeHtml(humanizeState(status.state))}</div>
              <div class="hero-countdown" id="heroCountdown">${escapeHtml(heroCountdownDisplay(status))}</div>
            </div>
          </div>
          <div class="hero-side">
            <div class="hero-caption" id="statusTimer">${escapeHtml(statusDetail(status))}</div>
            <ul class="hero-legend">
              <li><span class="dot dot--work"></span><strong id="workMinutesLegend">${secondsToMinutes(settings.workIntervalSeconds)}</strong><span>min work</span></li>
              <li><span class="dot dot--fixed"></span><strong>20</strong><span>ft distance</span></li>
              <li><span class="dot dot--rest"></span><strong id="restSecondsLegend">${settings.restDurationSeconds}</strong><span>sec rest</span></li>
            </ul>
          </div>
        </section>

        <main class="sections">
          <section class="panel">
            <div class="panel-head">
              <div>
                <h2>Timing</h2>
                <p class="panel-note">How often reminders trigger, and how long you rest.</p>
              </div>
            </div>
            <div class="card grid">
              <label>
                Work interval (minutes)
                <input id="workInterval" type="number" min="1" value="${secondsToMinutes(settings.workIntervalSeconds)}" />
              </label>
              <label>
                Rest duration (seconds)
                <input id="restDuration" type="number" min="1" value="${settings.restDurationSeconds}" />
              </label>
              <label>
                Idle threshold (seconds)
                <input id="idleThreshold" type="number" min="1" value="${settings.idleThresholdSeconds}" />
              </label>
              <label>
                Suppression mode
                <select id="suppressionMode">
                  <option value="delay" ${settings.suppressionMode === "delay" ? "selected" : ""}>Delay</option>
                  <option value="skip" ${settings.suppressionMode === "skip" ? "selected" : ""}>Skip</option>
                </select>
              </label>
            </div>
          </section>

          <section class="panel">
            <div class="panel-head">
              <div>
                <h2>Suppression &amp; focus</h2>
                <p class="panel-note">Hold off reminders during fullscreen apps, idle time, or chosen programs.</p>
              </div>
            </div>
            <div class="card">
              <label class="check-row"><input id="fullscreenSuppressionEnabled" type="checkbox" ${settings.fullscreenSuppressionEnabled ? "checked" : ""} /> Suppress during fullscreen apps</label>
              <label class="check-row"><input id="idleSuppressionEnabled" type="checkbox" ${settings.idleSuppressionEnabled ? "checked" : ""} /> Suppress while idle</label>
              <label class="check-row"><input id="autostart" type="checkbox" ${settings.autostart ? "checked" : ""} /> Start EyeRest when I sign in to Windows</label>
              <label>
                Whitelisted foreground processes
                <textarea id="whitelistedProcesses" spellcheck="false" placeholder="POWERPNT.EXE&#10;TEAMS.EXE">${escapeHtml(whitelistText(settings))}</textarea>
              </label>
              <div class="field-actions">
                <button id="chooseExecutable" type="button" class="secondary">Choose Executable</button>
              </div>
            </div>
          </section>

          <section class="panel">
            <div class="panel-head">
              <div>
                <h2>Monitors</h2>
                <p class="panel-note">Choose which screens show the reminder overlay.</p>
              </div>
            </div>
            <div class="card">
              <label>
                Monitor coverage
                <select id="monitorMode">
                  <option value="all" ${settings.monitorMode === "all" ? "selected" : ""}>All monitors</option>
                  <option value="selected" ${settings.monitorMode === "selected" ? "selected" : ""}>Selected monitors</option>
                </select>
              </label>
              <div id="monitorList" class="monitor-list">${monitorRows(settings)}</div>
            </div>
          </section>

          <section class="panel">
            <div class="panel-head">
              <div>
                <h2>Overlay design</h2>
                <p class="panel-note">Set the colors, shape, and layout of the reminder screen.</p>
              </div>
              <button id="restoreOverlayDefaults" type="button" class="secondary">Restore Defaults</button>
            </div>
            <div class="card grid">
              ${themeControl("background", "Background")}
              ${themeControl("textColor", "Text")}
              ${themeControl("mutedTextColor", "Muted text")}
              ${themeControl("accentColor", "Accent")}
              ${themeControl("dangerColor", "Danger")}
              ${themeControl("buttonBackground", "Button background")}
              ${themeControl("buttonTextColor", "Button text")}
              <label>
                Button radius
                <input id="buttonRadius" type="number" min="0" max="32" value="${settings.theme.buttonRadius}" />
              </label>
              <label>
                Layout
                <select id="layout">
                  <option value="centered" ${settings.theme.layout === "centered" ? "selected" : ""}>Centered</option>
                  <option value="calm" ${settings.theme.layout === "calm" ? "selected" : ""}>Calm</option>
                  <option value="compact" ${settings.theme.layout === "compact" ? "selected" : ""}>Compact</option>
                </select>
              </label>
              <label>
                Font family
                <input id="fontFamily" type="text" value="${escapeHtml(settings.theme.fontFamily)}" />
              </label>
              <div class="wide preview-frame">
                <div class="preview-frame-bar"><span></span><span></span><span></span></div>
                <div class="theme-preview" id="themePreview">
                  <div>
                    <h3>Time for a reset</h3>
                    <p>Look at something at least 6 meters away.</p>
                    <button type="button">Start Rest</button>
                  </div>
                </div>
              </div>
            </div>
          </section>

          <section class="panel">
            <div class="panel-head">
              <div>
                <h2>Accessibility</h2>
                <p class="panel-note">Motion and contrast preferences for the overlay.</p>
              </div>
            </div>
            <div class="card">
              <label class="check-row"><input id="soundEnabled" type="checkbox" ${settings.soundEnabled ? "checked" : ""} /> Play a sound when a rest ends</label>
              <label class="check-row"><input id="reducedMotion" type="checkbox" ${settings.theme.reducedMotion ? "checked" : ""} /> Reduced motion</label>
              <label class="check-row"><input id="highContrast" type="checkbox" ${settings.theme.highContrast ? "checked" : ""} /> High-contrast overlay</label>
            </div>
          </section>

        </main>
      </div>

      <footer class="action-bar">
        <div class="inner">
          <button id="startRestNow" class="secondary">Start Rest Now</button>
          <button id="resume" class="secondary">Resume</button>
				<button id="pause" class="secondary">Pause</button>
          <button id="save" class="save-action">Save changes</button>
        </div>
      </footer>
    </div>
  `;

  bindSettingsEvents();
  updateThemePreview();
  updateStatusSummary();
}

function bindSettingsEvents() {
  document.getElementById("save").addEventListener("click", saveSettings);
  document.getElementById("startRestNow").addEventListener("click", () => runSettingsAction("start_rest_now", "Rest started."));
  document.getElementById("pause").addEventListener("click", () => runSettingsAction("pause_reminders", "Reminders paused."));
  document.getElementById("resume").addEventListener("click", () => runSettingsAction("resume_reminders", "Reminders resumed."));
  document.getElementById("chooseExecutable").addEventListener("click", chooseExecutable);
  document.getElementById("restoreOverlayDefaults").addEventListener("click", restoreOverlayDefaults);
  document.getElementById("monitorMode").addEventListener("change", () => {
    currentSettings.monitorMode = document.getElementById("monitorMode").value;
    renderSettings();
  });

  for (const id of ["background", "textColor", "mutedTextColor", "accentColor", "dangerColor", "buttonBackground", "buttonTextColor", "buttonRadius", "layout", "fontFamily", "reducedMotion", "highContrast"]) {
    const element = document.getElementById(id);
    if (element) element.addEventListener("input", updateThemePreview);
  }
}

function collectSettingsFromForm() {
  const next = structuredClone(currentSettings);
  next.workIntervalSeconds = minutesToSeconds(document.getElementById("workInterval").value);
  next.restDurationSeconds = Math.max(1, Number(document.getElementById("restDuration").value) || 1);
  next.idleThresholdSeconds = Math.max(1, Number(document.getElementById("idleThreshold").value) || 1);
  next.suppressionMode = document.getElementById("suppressionMode").value;
  next.monitorMode = document.getElementById("monitorMode").value;
  next.selectedMonitorIds = Array.from(document.querySelectorAll(".monitorChoice:checked")).map((input) => input.value);
  next.whitelistedProcesses = document.getElementById("whitelistedProcesses").value
    .split(/[\n,;]/)
    .map((entry) => entry.trim().toUpperCase())
    .filter(Boolean);
  next.fullscreenSuppressionEnabled = document.getElementById("fullscreenSuppressionEnabled").checked;
  next.idleSuppressionEnabled = document.getElementById("idleSuppressionEnabled").checked;
  next.autostart = document.getElementById("autostart").checked;
  next.soundEnabled = document.getElementById("soundEnabled").checked;
  next.theme.background = document.getElementById("background").value;
  next.theme.textColor = document.getElementById("textColor").value;
  next.theme.mutedTextColor = document.getElementById("mutedTextColor").value;
  next.theme.accentColor = document.getElementById("accentColor").value;
  next.theme.dangerColor = document.getElementById("dangerColor").value;
  next.theme.buttonBackground = document.getElementById("buttonBackground").value;
  next.theme.buttonTextColor = document.getElementById("buttonTextColor").value;
  next.theme.buttonRadius = Math.max(0, Math.min(32, Number(document.getElementById("buttonRadius").value) || 0));
  next.theme.layout = document.getElementById("layout").value;
  next.theme.fontFamily = document.getElementById("fontFamily").value.trim() || "Inter, Segoe UI, sans-serif";
  next.theme.reducedMotion = document.getElementById("reducedMotion").checked;
  next.theme.highContrast = document.getElementById("highContrast").checked;
  return next;
}

async function saveSettings() {
  try {
    currentSettings = await invoke("save_settings", { settings: collectSettingsFromForm() });
    currentStatus = await invoke("get_app_status");
    statusCapturedAt = Date.now();
    updateStatusSummary();
    setActionMessage("Settings saved.");
  } catch (error) {
    setActionMessage(String(error), true);
  }
}

async function runSettingsAction(command, successMessage) {
  try {
    await invoke(command);
    await refreshStatus();
    setActionMessage(successMessage);
  } catch (error) {
    setActionMessage(String(error), true);
  }
}

async function chooseExecutable() {
  try {
    const processName = await invoke("pick_whitelisted_executable");
    if (!processName) {
      return;
    }
    const field = document.getElementById("whitelistedProcesses");
    const entries = field.value
      .split(/[\n,;]/)
      .map((entry) => entry.trim().toUpperCase())
      .filter(Boolean);
    if (!entries.includes(processName)) entries.push(processName);
    field.value = entries.join("\n");
    setActionMessage(`${processName} added. Save settings to apply it.`);
  } catch (error) {
    setActionMessage(String(error), true);
  }
}

function restoreOverlayDefaults() {
  currentSettings.theme = {
    ...DEFAULT_OVERLAY_THEME,
    reducedMotion: currentSettings.theme.reducedMotion,
    highContrast: currentSettings.theme.highContrast,
  };
  renderSettings();
  setActionMessage("Overlay design restored to defaults. Save changes to apply it.");
}

function updateThemePreview() {
  const preview = document.getElementById("themePreview");
  if (!preview) return;
  preview.style.setProperty("--preview-bg", document.getElementById("background").value);
  preview.style.setProperty("--preview-text", document.getElementById("textColor").value);
  preview.style.setProperty("--preview-muted", document.getElementById("mutedTextColor").value);
  preview.style.setProperty("--preview-button-bg", document.getElementById("buttonBackground").value);
  preview.style.setProperty("--preview-button-text", document.getElementById("buttonTextColor").value);
  preview.style.setProperty("--preview-radius", `${document.getElementById("buttonRadius").value}px`);
  preview.style.fontFamily = document.getElementById("fontFamily").value;
}

function applyOverlayTheme(theme) {
  const overlayTheme = theme.highContrast
    ? {
        ...theme,
        background: "#000000",
        textColor: "#ffffff",
        mutedTextColor: "#ffffff",
        buttonBackground: "#ffffff",
        buttonTextColor: "#000000",
      }
    : theme;
  document.documentElement.style.setProperty("--overlay-bg", overlayTheme.background);
  document.documentElement.style.setProperty("--overlay-text", overlayTheme.textColor);
  document.documentElement.style.setProperty("--overlay-muted", overlayTheme.mutedTextColor);
  document.documentElement.style.setProperty("--accent", overlayTheme.accentColor);
  document.documentElement.style.setProperty("--danger", overlayTheme.dangerColor);
  document.documentElement.style.setProperty("--button-bg", overlayTheme.buttonBackground);
  document.documentElement.style.setProperty("--button-text", overlayTheme.buttonTextColor);
  document.documentElement.style.setProperty("--button-radius", `${overlayTheme.buttonRadius}px`);
  document.body.style.fontFamily = overlayTheme.fontFamily;
}

function renderOverlay() {
  const status = currentStatus || {};
  const settings = status.settings || currentSettings;
  applyOverlayTheme(settings.theme);
  const resting = status.state === "Resting";
  const actionable = status.state === "ReminderShown" || resting;

  if (!actionable) {
    appEl.innerHTML = `
      <main class="overlay" role="dialog" aria-modal="true" aria-labelledby="overlayTitle" data-layout="${escapeHtml(settings.theme.layout)}" data-reduced-motion="${settings.theme.reducedMotion}">
        <section class="overlay-panel">
          <h1 id="overlayTitle">All set</h1>
          <p>Your next work interval has started.</p>
          <div class="overlay-actions">
            <button id="dismissOverlay" aria-label="Close overlay">Close</button>
          </div>
        </section>
      </main>
    `;
    document.getElementById("dismissOverlay").addEventListener("click", () => invoke("dismiss_overlay"));
    invoke("dismiss_overlay");
    return;
  }

  const mode = resting ? "resting" : "reminder";
  const existingOverlay = document.querySelector(`.overlay[data-mode="${mode}"]`);
  if (existingOverlay) {
    if (resting) updateRestCountdown(status.restRemainingSeconds);
    return;
  }

  appEl.innerHTML = `
    <main class="overlay" role="dialog" aria-modal="true" aria-labelledby="overlayTitle" data-mode="${mode}" data-layout="${escapeHtml(settings.theme.layout)}" data-reduced-motion="${settings.theme.reducedMotion}">
      <section class="overlay-panel">
        <h1 id="overlayTitle">${resting ? "Rest your eyes" : "Time for a reset"}</h1>
        <p>${resting ? "Look at something far away and breathe easy." : "Look at something at least 6 meters away for a short break."}</p>
        ${resting ? `<div id="restCountdown" class="countdown" aria-live="polite" aria-label="${status.restRemainingSeconds ?? settings.restDurationSeconds} seconds remaining">${status.restRemainingSeconds ?? settings.restDurationSeconds}</div>` : ""}
        <div class="overlay-actions">
          ${resting ? `<button id="cancelRest" class="danger" aria-label="Cancel rest">Cancel</button>` : `<button id="startRest" aria-label="Start rest">Start Rest</button><button id="skip" class="danger" aria-label="Skip reminder">Skip</button>`}
        </div>
        <div class="sr-only" role="status" aria-live="polite">${resting ? "Rest started." : "Reminder shown."}</div>
      </section>
    </main>
  `;

  const start = document.getElementById("startRest");
  if (start) {
    start.addEventListener("click", () => invoke("start_rest"));
  }
  const skip = document.getElementById("skip");
  if (skip) {
    skip.addEventListener("click", () => invoke("skip_reminder"));
  }
  const cancel = document.getElementById("cancelRest");
  if (cancel) {
    cancel.addEventListener("click", () => invoke("cancel_rest"));
  }
  trapOverlayFocus();
}

function updateRestCountdown(remainingSeconds) {
  const countdown = document.getElementById("restCountdown");
  if (!countdown || remainingSeconds === undefined || remainingSeconds === null) return;
  countdown.textContent = remainingSeconds;
  countdown.setAttribute("aria-label", `${remainingSeconds} seconds remaining`);
}

function trapOverlayFocus() {
  const overlay = document.querySelector(".overlay");
  if (!overlay) return;
  overlay.addEventListener("keydown", (event) => {
    if (event.key !== "Tab") return;
    const focusable = Array.from(overlay.querySelectorAll("button, [href], input, select, textarea, [tabindex]:not([tabindex='-1'])"));
    if (!focusable.length) return;
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  });
}

async function loadSettings() {
  currentSettings = await invoke("get_settings");
  currentStatus = await invoke("get_app_status");
  statusCapturedAt = Date.now();
  if (view !== "overlay") {
    try {
      availableMonitors = await invoke("list_monitors");
    } catch {
      availableMonitors = [];
    }
  }
  if (view === "overlay") {
    renderOverlay();
  } else {
    renderSettings();
  }
}

async function refreshStatus() {
  currentStatus = await invoke("get_app_status");
  if (currentStatus.settings) currentSettings = currentStatus.settings;
  statusCapturedAt = Date.now();
  if (view === "overlay") {
    renderOverlay();
  } else {
    updateStatusSummary();
  }
}

function handleEvent(event) {
  if (!event || !event.payload) return;
  if (event.payload.type === "RestTick") {
    const remainingSeconds = event.payload.payload?.remaining_seconds;
    if (currentStatus) {
      currentStatus.restRemainingSeconds = remainingSeconds;
      currentStatus.state = "Resting";
      statusCapturedAt = Date.now();
    }
    updateRestCountdown(remainingSeconds);
    updateStatusSummary();
    return;
  }
  refreshStatus().catch(() => {});
}

window.addEventListener("keydown", (event) => {
  if (event.key !== "Escape") return;
  if (view === "overlay" && currentStatus?.state === "Resting") {
    invoke("cancel_rest").then(() => invoke("dismiss_overlay"));
  } else if (view === "overlay") {
    invoke("skip_reminder");
  }
});

loadSettings();
if (view !== "overlay") setInterval(updateStatusSummary, 1000);
if (tauriApi?.event?.listen) {
  tauriApi.event.listen("eyerest://event", handleEvent);
}
