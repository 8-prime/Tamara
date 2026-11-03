import { z } from "zod";
import { ConferenceSchema } from "./models.ts";
export function titleOf(note: { title?: string | null; content: string }) {
  return note.title?.trim() || note.content.trim().split(/\s+/).slice(0, 7).join(' ').slice(0, 90) || 'Neue Konferenz';
}
export function snapshots<T>(history: Snapshot<T>[], note: T, now = Date.now(), force = false) {
  const recent = history.filter(x => now - x.at < 7 * 86400000);
  const last = recent.at(-1);
  if (!last || (JSON.stringify(last.note) !== JSON.stringify(note) && (force || now - last.at >= 30000))) {
    recent.push({ at: now, note: structuredClone(note) });
  }
  return recent.slice(-20);
}
export interface Snapshot<T> { at: number; note: T }
export type ConferenceNote = z.infer<typeof ConferenceSchema>;
const LocalRecordsSchema = z.array(z.object({account: z.string(), note: ConferenceSchema, history: z.array(z.object({at: z.number(), note: ConferenceSchema})), pending: z.boolean()}));
export type LocalRecord = z.infer<typeof LocalRecordsSchema>[number];
let database: Promise<IDBDatabase> | null;
function open() {
  return database ||= new Promise<IDBDatabase>((resolve, reject) => {
    const request = indexedDB.open('tamara-conferences', 1);
    request.onupgradeneeded = () => request.result.createObjectStore('records', { keyPath: 'key' });
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => { database = null; reject(request.error); };
  });
}
export async function records(account: string) {
  const db = await open();
  return new Promise<LocalRecord[]>((resolve, reject) => {
    const request = db.transaction('records').objectStore('records').getAll();
    request.onsuccess = () => { try { resolve(LocalRecordsSchema.parse(request.result).filter(x => x.account === account)); } catch (error) { reject(error); } };
    request.onerror = () => reject(request.error);
  });
}
export async function persist(account: string, note: ConferenceNote, history: Snapshot<ConferenceNote>[], pending: boolean) {
  const db = await open();
  return new Promise<void>((resolve, reject) => {
    const tx = db.transaction('records', 'readwrite');
    tx.objectStore('records').put({ key: `${account}:${note.id}`, account, note: ConferenceSchema.parse(note), history: history.map(x => ({at: x.at, note: ConferenceSchema.parse(x.note)})), pending });
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
  });
}
export async function forget(account: string, id: string) {
  const db = await open();
  return new Promise<void>((resolve, reject) => {
    const tx = db.transaction('records', 'readwrite');
    tx.objectStore('records').delete(`${account}:${id}`);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
  });
}
