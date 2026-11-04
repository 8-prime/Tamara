import { test } from 'node:test';
import assert from 'node:assert/strict';
import { attendeeNames, groupConferences } from './parent-conferences.ts';
import type { ParentConference } from './models.ts';
import { ParentConferenceSchema } from './models.ts';
import { workspaceUrl, readRoute } from './navigation.ts';
const meeting = (id: string, student_id: string, student_name: string, date: string, attendees: string[]): ParentConference => ({ id, student_id, student_name, date, attendees, classroom_id: 'class', class_name: '1a', content: '', review_snapshot: null });
test('daily groups count meetings, distinct pupils and attending guardians, including duplicate pupil names', () => {
  const entries = [meeting('a','1','Mia','2026-10-01',['Mother','Father']), meeting('b','2','Mia','2026-10-01',[]), meeting('c','1','Mia','2026-10-01',['Mother']), meeting('d','3','Jonas','2026-11-02',['Father'])];
  const groups = groupConferences(entries,'');
  assert.deepEqual(groups.map(g => [g.date,g.meetings.length,g.pupils,g.parents]), [['2026-11-02',1,1,1],['2026-10-01',3,2,3]]);
  assert.equal(entries[0]?.id,'a');
  assert.equal(groupConferences(entries,' MIA ')[0]?.meetings.length,3);
  assert.equal(groupConferences(entries,'absent').length,0);
});
test('search finds pupil meetings across dates and attendance ignores empty lines', () => {
  assert.equal(groupConferences([meeting('a','1','Änne','2026-10-01',[]),meeting('b','1','Änne','2026-09-01',[])],'änne').length,2);
  assert.deepEqual(attendeeNames(' Mother \n\n Father\n'),['Mother','Father']);
});
test('review response rejects invalid assessment state', () => {
  const entry=meeting('a','1','Mia','2026-10-01',[]);
  assert.equal(ParentConferenceSchema.safeParse(entry).success,true);
  assert.equal(ParentConferenceSchema.safeParse({...entry, review_snapshot:{period_name:'Herbst',school_year:'2026/27',captured_at:123,assessments:[{criterion_id:'x',label:'X',domain:'D',section:'S',level:'01',value:'maybe',achieved_on:null}]}}).success,false);
});
test('parent conference area has a global route without class context', () => {
  assert.equal(workspaceUrl('','parent-conferences'),'/parent-conferences');
  Object.defineProperty(globalThis,'window',{value:{location:new URL('https://tamara.test/parent-conferences')},configurable:true});
  try { assert.equal(readRoute().feature,'parent-conferences'); assert.equal(readRoute().classId,''); }
  finally { Reflect.deleteProperty(globalThis,'window'); }
});

test('unsaved parent notes survive navigation in memory and are cleared on logout', async () => {
  const { parentDraft, writeParentDraft, memoryGeneration, clearMemory } = await import('./conference-store.ts');
  clearMemory();
  const generation = memoryGeneration();
  const draft = { note: meeting('a','1','Mia','2026-10-01',[]), attendees: 'Mother\n', history: [] };
  writeParentDraft('owner',draft,generation);
  draft.note.content = 'later';
  assert.equal(parentDraft('owner')?.note.content,'');
  assert.equal(parentDraft('other'),undefined);
  clearMemory();
  writeParentDraft('owner',draft,generation);
  assert.equal(parentDraft('owner'),undefined);
});
