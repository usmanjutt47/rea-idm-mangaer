const isFirefox = typeof globalThis.browser !== 'undefined';
const api = isFirefox ? globalThis.browser : globalThis.chrome;
const toggle = document.querySelector('#intercept');
const send = document.querySelector('#send');
const status = document.querySelector('#status');
async function get(store, key) { if (isFirefox) return store.get(key); return new Promise(resolve => store.get(key, resolve)); }
async function set(store, value) { if (isFirefox) return store.set(value); return new Promise(resolve => store.set(value, resolve)); }
async function activeTab() { if (isFirefox) return (await api.tabs.query({ active: true, currentWindow: true }))[0]; return new Promise(resolve => api.tabs.query({ active: true, currentWindow: true }, tabs => resolve(tabs[0]))); }
async function sendMessage(message) { if (isFirefox) return api.runtime.sendMessage(message); return new Promise(resolve => api.runtime.sendMessage(message, response => resolve(api.runtime.lastError ? null : response))); }
void (async () => { const settings = await get(api.storage.local, { autoIntercept: false }); toggle.checked = settings.autoIntercept === true; const tab = await activeTab(); if (typeof tab?.id === 'number') { const detected = await get(api.storage.session, `detected:${tab.id}`); send.disabled = !detected[`detected:${tab.id}`]; } })();
toggle.addEventListener('change', () => { void set(api.storage.local, { autoIntercept: toggle.checked }).then(() => { status.textContent = toggle.checked ? 'Automatic interception enabled.' : 'Automatic interception disabled.'; }); });
send.addEventListener('click', () => { send.disabled = true; void sendMessage({ action: 'submit-detected' }).then(result => { status.textContent = result?.message || 'Desktop bridge unavailable.'; if (!result?.ok) send.disabled = false; }); });
