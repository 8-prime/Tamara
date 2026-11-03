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
