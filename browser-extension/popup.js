const portInput = document.getElementById("port");
const tokenInput = document.getElementById("token");
const saveButton = document.getElementById("save");
const revealButton = document.getElementById("reveal");
const refreshButton = document.getElementById("refresh");
const statusEl = document.getElementById("status");
const entriesEl = document.getElementById("entries");
const emptyEl = document.getElementById("empty");

let revealToken = false;

function setStatus(message, tone = "") {
  statusEl.textContent = message;
  statusEl.classList.remove("good", "bad");
  if (tone) statusEl.classList.add(tone);
}

function baseUrl() {
  const port = portInput.value.trim() || "17832";
  return `http://127.0.0.1:${port}`;
}

function tokenValue() {
  return tokenInput.value.trim();
}

async function loadSettings() {
  const stored = await chrome.storage.local.get(["port", "token"]);
  if (stored.port) portInput.value = stored.port;
  if (stored.token) tokenInput.value = stored.token;
}

async function saveSettings() {
  const port = portInput.value.trim() || "17832";
  const token = tokenValue();
  await chrome.storage.local.set({ port, token });
  setStatus("Settings saved.", "good");
  await loadEntries();
}

async function getActiveTab() {
  const tabs = await chrome.tabs.query({ active: true, currentWindow: true });
  return tabs[0];
}

async function fetchJson(path) {
  const token = tokenValue();
  if (!token) {
    throw new Error("Missing token. Paste the pairing token.");
  }
  const response = await fetch(`${baseUrl()}${path}`, {
    headers: {
      "X-Organizer-Token": token
    }
  });
  if (!response.ok) {
    let message = `Request failed (${response.status}).`;
    try {
      const data = await response.json();
      if (data && data.error) message = data.error;
    } catch {
      // ignore parse errors
    }
    throw new Error(message);
  }
  return await response.json();
}

function clearEntries() {
  entriesEl.innerHTML = "";
}

function renderEntries(entries) {
  clearEntries();
  if (!entries.length) {
    emptyEl.textContent = "No matching entries found for this site.";
    emptyEl.style.display = "block";
    return;
  }

  emptyEl.style.display = "none";
  entries.forEach((entry, index) => {
    const card = document.createElement("div");
    card.className = "entry";
    card.style.animationDelay = `${index * 30}ms`;

    const title = document.createElement("div");
    title.className = "entry-title";
    title.textContent = entry.title || "Untitled";

    const meta = document.createElement("div");
    meta.className = "entry-meta";
    meta.textContent = entry.username || entry.url || "No username";

    const actions = document.createElement("div");
    actions.className = "entry-actions";

    const fillButton = document.createElement("button");
    fillButton.className = "btn";
    fillButton.textContent = "Fill";
    fillButton.type = "button";
    fillButton.addEventListener("click", () => fillEntry(entry));

    actions.appendChild(fillButton);
    card.appendChild(title);
    card.appendChild(meta);
    card.appendChild(actions);
    entriesEl.appendChild(card);
  });
}

async function loadEntries() {
  clearEntries();
  emptyEl.textContent = "Looking for matches...";
  emptyEl.style.display = "block";

  const tab = await getActiveTab();
  if (!tab || !tab.url) {
    emptyEl.textContent = "Open a login page to see matches.";
    return;
  }

  try {
    const data = await fetchJson(`/v1/entries?url=${encodeURIComponent(tab.url)}`);
    renderEntries(data.entries || []);
    setStatus("", "");
  } catch (err) {
    renderEntries([]);
    setStatus(err.message, "bad");
  }
}

async function fillEntry(entry) {
  const tab = await getActiveTab();
  if (!tab || !tab.id) {
    setStatus("No active tab available.", "bad");
    return;
  }

  try {
    const secret = await fetchJson(`/v1/secret?id=${encodeURIComponent(entry.id)}`);
    const result = await chrome.scripting.executeScript({
      target: { tabId: tab.id },
      func: fillCredentials,
      args: [entry.username || "", secret.password]
    });
    const outcome = result && result[0] && result[0].result;
    if (outcome && outcome.ok === false) {
      setStatus(outcome.message || "Unable to fill the form.", "bad");
    } else {
      setStatus("Credentials filled.", "good");
    }
  } catch (err) {
    setStatus(err.message, "bad");
  }
}

function fillCredentials(username, password) {
  const isVisible = (input) => {
    const rects = input.getClientRects();
    return rects.length > 0 && input.offsetParent !== null;
  };

  const isTextField = (input) => {
    const type = (input.type || "text").toLowerCase();
    return type === "text" || type === "email" || type === "username";
  };

  const setValue = (input, value) => {
    const setter = Object.getOwnPropertyDescriptor(input.__proto__, "value");
    if (setter && setter.set) {
      setter.set.call(input, value);
    } else {
      input.value = value;
    }
    input.dispatchEvent(new Event("input", { bubbles: true }));
    input.dispatchEvent(new Event("change", { bubbles: true }));
  };

  const inputs = Array.from(document.querySelectorAll("input"));
  const passwordInput = inputs.find(
    (input) => input.type === "password" && isVisible(input)
  );

  if (!passwordInput) {
    return { ok: false, message: "No password field found on this page." };
  }

  const formInputs = passwordInput.form
    ? Array.from(passwordInput.form.querySelectorAll("input"))
    : inputs;
  const passwordIndex = formInputs.indexOf(passwordInput);

  let usernameInput = null;
  for (let i = passwordIndex - 1; i >= 0; i -= 1) {
    const candidate = formInputs[i];
    if (candidate && isTextField(candidate) && isVisible(candidate)) {
      usernameInput = candidate;
      break;
    }
  }

  if (!usernameInput) {
    usernameInput = formInputs.find(
      (candidate) => isTextField(candidate) && isVisible(candidate)
    );
  }

  if (usernameInput && username) {
    setValue(usernameInput, username);
  }
  setValue(passwordInput, password);

  return { ok: true };
}

saveButton.addEventListener("click", () => {
  saveSettings().catch((err) => setStatus(err.message, "bad"));
});

revealButton.addEventListener("click", () => {
  revealToken = !revealToken;
  tokenInput.type = revealToken ? "text" : "password";
  revealButton.textContent = revealToken ? "Hide token" : "Reveal token";
});

refreshButton.addEventListener("click", () => {
  loadEntries().catch((err) => setStatus(err.message, "bad"));
});

loadSettings()
  .then(loadEntries)
  .catch((err) => setStatus(err.message, "bad"));
