export function titleOf(note) {
  return note.title?.trim() || note.content.trim().split(/\s+/).slice(0, 7).join(' ').slice(0, 90) || 'Neue Konferenz';
}
export function snapshots(history, note, now = Date.now(), force = false) {
  const recent = history.filter(x => now - x.at < 7 * 86400000);
  const last = recent.at(-1);
  if (!last || (JSON.stringify(last.note) !== JSON.stringify(note) && (force || now - last.at >= 30000))) {
    recent.push({ at: now, note: JSON.parse(JSON.stringify(note)) });
  }
  return recent.slice(-20);
}
let database;
function open() {
  return database ||= new Promise((resolve, reject) => {
    const request = indexedDB.open('tamara-conferences', 1);
    request.onupgradeneeded = () => request.result.createObjectStore('records', { keyPath: 'key' });
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => { database = null; reject(request.error); };
  });
}
export async function records(account) {
  const db = await open();
  return new Promise((resolve, reject) => {
    const request = db.transaction('records').objectStore('records').getAll();
    request.onsuccess = () => resolve(request.result.filter(x => x.account === account));
    request.onerror = () => reject(request.error);
  });
}
export async function persist(account, note, history, pending) {
  const db = await open();
  return new Promise((resolve, reject) => {
    const tx = db.transaction('records', 'readwrite');
    tx.objectStore('records').put({ key: `${account}:${note.id}`, account, note: JSON.parse(JSON.stringify(note)), history: JSON.parse(JSON.stringify(history)), pending });
    tx.oncomplete = resolve;
    tx.onerror = () => reject(tx.error);
  });
}
export async function forget(account, id) {
  const db = await open();
  return new Promise((resolve, reject) => {
    const tx = db.transaction('records', 'readwrite');
    tx.objectStore('records').delete(`${account}:${id}`);
    tx.oncomplete = resolve;
    tx.onerror = () => reject(tx.error);
  });
}
