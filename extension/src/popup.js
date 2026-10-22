const api = globalThis.browser ?? globalThis.chrome;
const toggle = document.querySelector('#intercept');
const status = document.querySelector('#status');
api.storage.local.get({ autoIntercept: false }, result => { toggle.checked = result.autoIntercept === true; });
toggle.addEventListener('change', () => { api.storage.local.set({ autoIntercept: toggle.checked }, () => { status.textContent = toggle.checked ? 'Automatic interception enabled.' : 'Automatic interception disabled.'; }); });
