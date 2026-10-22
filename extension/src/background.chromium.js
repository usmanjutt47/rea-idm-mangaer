/* Chromium MV3 worker. This file is deliberately classic, so importScripts is valid. */
importScripts('config.js');

const nativeHost = 'com.rea.idm_mangaer';
const mediaUrls = new Map();
const handoffs = new Set();
const directFile = /\.(zip|7z|rar|exe|msi|iso|dmg|pkg|mp4|mkv|webm|mp3|m4a|wav|flac|pdf|docx?|xlsx?|pptx?)(?:$|[?#])/i;
let autoIntercept = false;

chrome.storage.local.get({ autoIntercept: false }, result => { autoIntercept = result.autoIntercept === true; });
chrome.storage.onChanged.addListener((changes, area) => { if (area === 'local' && changes.autoIntercept) autoIntercept = changes.autoIntercept.newValue === true; });
chrome.runtime.onInstalled.addListener(createMenu);
chrome.runtime.onStartup.addListener(createMenu);
chrome.contextMenus.onClicked.addListener((info, tab) => { const url = info.linkUrl || info.srcUrl || tab?.url; if (url) void sendToDesktop(url, tab?.id); });
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message?.action !== 'submit-detected' || !sender.tab?.id) return;
  const url = mediaUrls.get(sender.tab.id);
  if (!url) { sendResponse({ ok: false, message: 'No media has been detected in this tab.' }); return; }
  sendToDesktop(url, sender.tab.id, sendResponse); return true;
});
chrome.downloads.onCreated.addListener(download => {
  if (!autoIntercept || !isDirect(download.finalUrl || download.url) || download.byExtensionId) return;
  void handoffThenCancel(download);
});
chrome.webRequest.onHeadersReceived.addListener(details => {
  const type = details.responseHeaders?.find(header => header.name.toLowerCase() === 'content-type')?.value || '';
  if (/^(video|audio)\//i.test(type) || isDirect(details.url)) { mediaUrls.set(details.tabId, details.url); chrome.storage.session.set({ [`detected:${details.tabId}`]: details.url }); chrome.action.setBadgeText({ tabId: details.tabId, text: '↓' }); chrome.action.setTitle({ tabId: details.tabId, title: 'Download detected — open REA popup to send' }); }
}, { urls: ['<all_urls>'], types: ['media', 'xmlhttprequest', 'other'] }, ['responseHeaders']);
chrome.webRequest.onBeforeRequest.addListener(details => {
  if (details.type === 'main_frame' && /(^https?:\/\/)(www\.)?(youtube\.com|youtu\.be)\//i.test(details.url)) { chrome.action.setBadgeText({ tabId: details.tabId, text: '!' }); chrome.action.setTitle({ tabId: details.tabId, title: 'YouTube detected — direct, authorized media URLs only' }); }
}, { urls: ['<all_urls>'], types: ['main_frame'] });
function createMenu() { chrome.contextMenus.removeAll(() => chrome.contextMenus.create({ id: 'rea-download', title: 'Download with REA Idm Mangaer', contexts: ['link', 'video', 'audio', 'image'] })); }
function isDirect(url) { return /^https?:\/\//i.test(url) && directFile.test(url); }
function handoffThenCancel(download) { const url = download.finalUrl || download.url; const key = `${download.id}:${url}`; if (handoffs.has(key)) return; handoffs.add(key); sendToDesktop(url, download.tabId, result => { handoffs.delete(key); if (!result.ok) { notify(download.tabId, 'REA unavailable — browser download kept'); return; } chrome.downloads.cancel(download.id, () => { if (!chrome.runtime.lastError) chrome.downloads.erase({ id: download.id }); }); }); }
function sendToDesktop(url, tabId, complete = () => {}) { if (!globalThis.REA_BRIDGE_TOKEN || globalThis.REA_BRIDGE_TOKEN === 'INSTALL_REQUIRED') { const message = 'Install the native bridge first'; notify(tabId, message); complete({ ok: false, message }); return; } chrome.runtime.sendNativeMessage(nativeHost, { token: globalThis.REA_BRIDGE_TOKEN, action: 'enqueue', url }, response => { if (chrome.runtime.lastError || !response?.ok) { const message = response?.message || 'Desktop bridge unavailable — browser download kept'; notify(tabId, message); complete({ ok: false, message }); return; } const message = response.message || 'Sent to REA Idm Mangaer'; notify(tabId, message); complete({ ok: true, message }); }); }
function notify(tabId, message) { if (typeof tabId === 'number' && tabId >= 0) chrome.action.setTitle({ tabId, title: message }); }
