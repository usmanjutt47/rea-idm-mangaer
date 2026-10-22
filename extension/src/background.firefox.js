/* Firefox MV3 uses background.scripts, not Chromium's service-worker entry point. */
const nativeHost = 'com.rea.idm_mangaer';
const mediaUrls = new Map();
const directFile = /\.(zip|7z|rar|exe|msi|iso|dmg|pkg|mp4|mkv|webm|mp3|m4a|wav|flac|pdf|docx?|xlsx?|pptx?)(?:$|[?#])/i;
let autoIntercept = false;

async function initialise() {
  ({ autoIntercept = false } = await browser.storage.local.get({ autoIntercept: false }));
  await createMenu();
}
browser.runtime.onInstalled.addListener(() => { void createMenu(); });
browser.runtime.onStartup.addListener(() => { void createMenu(); });
browser.storage.onChanged.addListener((changes, area) => { if (area === 'local' && changes.autoIntercept) autoIntercept = changes.autoIntercept.newValue === true; });
browser.contextMenus.onClicked.addListener((info, tab) => { const url = info.linkUrl || info.srcUrl || tab?.url; if (url) void sendToDesktop(url, tab?.id); });
browser.action.onClicked.addListener(tab => { if (tab?.id && mediaUrls.get(tab.id)) void sendToDesktop(mediaUrls.get(tab.id), tab.id); });
browser.downloads.onCreated.addListener(download => { if (autoIntercept && isDirect(download.finalUrl || download.url) && !download.byExtensionId) void intercept(download); });
browser.webRequest.onHeadersReceived.addListener(details => {
  const type = details.responseHeaders?.find(header => header.name.toLowerCase() === 'content-type')?.value || '';
  if (/^(video|audio)\//i.test(type) || isDirect(details.url)) { mediaUrls.set(details.tabId, details.url); void browser.action.setBadgeText({ tabId: details.tabId, text: '↓' }); void browser.action.setTitle({ tabId: details.tabId, title: 'Download detected — click to send to REA' }); }
}, { urls: ['<all_urls>'], types: ['media', 'xmlhttprequest', 'other'] }, ['responseHeaders']);
browser.webRequest.onBeforeRequest.addListener(details => { if (details.type === 'main_frame' && /(^https?:\/\/)(www\.)?(youtube\.com|youtu\.be)\//i.test(details.url)) { void browser.action.setBadgeText({ tabId: details.tabId, text: '!' }); void browser.action.setTitle({ tabId: details.tabId, title: 'YouTube detected — direct, authorized media URLs only' }); } }, { urls: ['<all_urls>'], types: ['main_frame'] });
async function createMenu() { await browser.contextMenus.removeAll(); await browser.contextMenus.create({ id: 'rea-download', title: 'Download with REA Idm Mangaer', contexts: ['link', 'video', 'audio', 'image'] }); }
async function intercept(download) { try { await browser.downloads.cancel(download.id); await browser.downloads.erase({ id: download.id }); await sendToDesktop(download.finalUrl || download.url, download.tabId); } catch { /* Browser denied cancellation; it remains a browser download. */ } }
function isDirect(url) { return /^https?:\/\//i.test(url) && directFile.test(url); }
async function sendToDesktop(url, tabId) { if (!globalThis.REA_BRIDGE_TOKEN || globalThis.REA_BRIDGE_TOKEN === 'INSTALL_REQUIRED') { await notify(tabId, 'Install the native bridge first'); return; } try { const response = await browser.runtime.sendNativeMessage(nativeHost, { token: globalThis.REA_BRIDGE_TOKEN, action: 'enqueue', url }); await notify(tabId, response?.message || 'Sent to REA Idm Mangaer'); } catch { await notify(tabId, 'Desktop bridge unavailable — see setup'); } }
async function notify(tabId, message) { if (typeof tabId === 'number' && tabId >= 0) await browser.action.setTitle({ tabId, title: message }); }
void initialise();
