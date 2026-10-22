/* REA Idm Mangaer browser bridge. It never extracts DRM media or credentials. */
importScripts('config.js');
const nativeHost = 'com.rea.idm_mangaer';
const mediaUrls = new Map();
const directFile = /\.(zip|7z|rar|exe|msi|iso|dmg|pkg|mp4|mkv|webm|mp3|m4a|wav|flac|pdf|docx?|xlsx?|pptx?)(?:$|[?#])/i;

chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.removeAll(() => chrome.contextMenus.create({ id: 'rea-download', title: 'Download with REA Idm Mangaer', contexts: ['link', 'video', 'audio', 'image'] }));
});
chrome.contextMenus.onClicked.addListener((info, tab) => {
  const url = info.linkUrl || info.srcUrl || tab?.url;
  if (url) sendToDesktop(url, tab?.id);
});
chrome.action.onClicked.addListener(tab => { if (tab?.id && mediaUrls.get(tab.id)) sendToDesktop(mediaUrls.get(tab.id), tab.id); });
chrome.webRequest.onHeadersReceived.addListener(details => {
  const type = details.responseHeaders?.find(h => h.name.toLowerCase() === 'content-type')?.value || '';
  if (/^(video|audio)\//i.test(type) || directFile.test(details.url)) { mediaUrls.set(details.tabId, details.url); chrome.action.setBadgeText({ tabId: details.tabId, text: '↓' }); chrome.action.setTitle({ tabId: details.tabId, title: 'Download detected — click to send to REA' }); }
}, { urls: ['<all_urls>'], types: ['media', 'xmlhttprequest', 'other'] }, ['responseHeaders']);
chrome.webRequest.onBeforeRequest.addListener(details => {
  if (details.type === 'main_frame' && /(^https?:\/\/)(www\.)?(youtube\.com|youtu\.be)\//i.test(details.url)) { chrome.action.setBadgeText({ tabId: details.tabId, text: '!' }); chrome.action.setTitle({ tabId: details.tabId, title: 'YouTube detected — only authorized, direct media URLs are supported' }); }
}, { urls: ['<all_urls>'], types: ['main_frame'] });
function sendToDesktop(url, tabId) {
  if (!globalThis.REA_BRIDGE_TOKEN || globalThis.REA_BRIDGE_TOKEN === 'INSTALL_REQUIRED') { notify(tabId, 'Install the native bridge first'); return; }
  chrome.runtime.sendNativeMessage(nativeHost, { token: globalThis.REA_BRIDGE_TOKEN, action: 'enqueue', url }, response => {
    if (chrome.runtime.lastError) { notify(tabId, 'Desktop bridge unavailable — see extension setup'); return; }
    notify(tabId, response?.message || 'Sent to REA Idm Mangaer');
  });
}
function notify(tabId, message) { if (tabId) chrome.action.setTitle({ tabId, title: message }); }
