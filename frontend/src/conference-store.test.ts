import test from 'node:test';
import assert from 'node:assert/strict';
import { titleOf, snapshots } from './conference-store.ts';
test('titles follow note unless overridden', () => {
  assert.equal(titleOf({content:'one two three four five six seven eight'}),'one two three four five six seven');
  assert.equal(titleOf({content:'new text',title:'Custom'}),'Custom');
  assert.equal(titleOf({content:''}),'Neue Konferenz');
});
test('history is bounded, expires, throttles, and preserves forced restore points', () => {
  let h: import("./conference-store.ts").Snapshot<{content: string}>[]=[];
  for(let i=0;i<30;i++) h=snapshots(h,{content:String(i)},i*31000);
  assert.equal(h.length,20);
  assert.equal(h[0]?.note.content,'10');
  assert.equal(snapshots(h,{content:'typing'},29*31000+100).length,20);
  assert.equal(snapshots(h,{content:'restore'},29*31000+100,true).at(-1)?.note.content,'restore');
  assert.equal(snapshots(h,{content:'later'},9*86400000).length,1);
});

test('drafts are isolated, copied, and cannot be repopulated after logout', async () => {
  const {persist, records, clearMemory, memoryGeneration, forget} = await import('./conference-store.ts');
  clearMemory();
  const generation = memoryGeneration();
  const note = {id:'n', date:'2026-11-03', title:null, classroom_id:null, content:'private', updated_at:1};
  await persist('a', note, [], true, generation);
  note.content = 'changed';
  assert.equal((await records('a'))[0]?.note.content, 'private');
  assert.deepEqual(await records('b'), []);
  await forget('a', 'n');
  assert.deepEqual(await records('a'), []);
  clearMemory();
  await persist('a', note, [], true, generation);
  assert.deepEqual(await records('a'), []);
});
