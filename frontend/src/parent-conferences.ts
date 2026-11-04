import type { ParentConference } from './models.ts';
export function groupConferences(entries: ParentConference[], search: string) {
  const term = search.trim().toLocaleLowerCase('de');
  const groups = new Map<string, ParentConference[]>();
  for (const entry of [...entries].sort((a,b) => b.date.localeCompare(a.date))) {
    if (term && !entry.student_name.toLocaleLowerCase('de').includes(term)) continue;
    const day = groups.get(entry.date) ?? [];
    day.push(entry); groups.set(entry.date, day);
  }
  return [...groups].map(([date, meetings]) => ({ date, meetings, pupils: new Set(meetings.map(m => m.student_id)).size, parents: meetings.reduce((n,m) => n + m.attendees.length, 0) }));
}
export function attendeeNames(text: string) { return text.split('\n').map(n => n.trim()).filter(Boolean); }
