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
export interface LocalRecord { account: string; note: ConferenceNote; history: Snapshot<ConferenceNote>[]; pending: boolean }
const drafts = new Map<string, LocalRecord>();
let generation = 0;
export function clearMemory() { generation++; drafts.clear(); }
export function memoryGeneration() { return generation; }
export async function records(account: string): Promise<LocalRecord[]> {
  return structuredClone([...drafts.values()].filter(row => row.account === account));
}
export async function persist(account: string, note: ConferenceNote, history: Snapshot<ConferenceNote>[], pending: boolean, expectedGeneration = generation) {
  if (expectedGeneration !== generation) return;
  drafts.set(`${account}:${note.id}`, structuredClone({ account, note: ConferenceSchema.parse(note), history, pending }));
}
export async function forget(account: string, id: string) { drafts.delete(`${account}:${id}`); }
// Delete legacy on-disk drafts without reading their sensitive contents.
export async function removeLegacyStorage(): Promise<void> {
  if (typeof indexedDB === 'undefined') return;
  await new Promise<void>((resolve, reject) => {
    const request = indexedDB.deleteDatabase('tamara-conferences');
    request.onsuccess = () => resolve();
    request.onerror = () => reject(new Error('Alte lokale Konferenzkopien konnten nicht gelöscht werden.'));
    request.onblocked = () => reject(new Error('Bitte andere Tamara-Tabs schließen und neu laden, um alte lokale Konferenzkopien zu löschen.'));
  });
}
