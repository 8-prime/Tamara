<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from './api.ts';
  import { titleOf, snapshots, records, persist, forget } from './conference-store.ts';
  let { accountId, classrooms, onerror } = $props();
  let entries = $state([]), selected = $state(''), loading = $state(true);
  let error = $state(''), localError = $state(''), status = $state(''), historyOpen = $state(false), preview = $state(null);
  let histories = $state({});
  let pending = new Map(), timer, queue = Promise.resolve(), running = false, disposed = false;
  let current = $derived(entries.find(n => n.id === selected));
  let ordered = $derived([...entries].sort((a,b) => b.date.localeCompare(a.date)));
  function localWrite(note, history, dirty) {
    note = JSON.parse(JSON.stringify(note));
    history = JSON.parse(JSON.stringify(history));
    queue = queue.catch(() => {}).then(() => persist(accountId, note, history, dirty)).catch(() => {
      localError = 'Browserspeicher nicht verfügbar. Bitte vor dem Schließen auf „Gespeichert“ warten.';
    });
    return queue;
  }
  function change(values, force = false) {
    if (!histories[current.id]?.length) histories[current.id] = snapshots([], current);
    const note = { ...current, ...values };
    entries = entries.map(n => n.id === note.id ? note : n);
    histories[note.id] = snapshots(histories[note.id] || [], note, Date.now(), force);
    pending.set(note.id, note);
    localWrite(note, histories[note.id], true);
    status = 'Ungespeicherte Änderungen';
    clearTimeout(timer);
    timer = setTimeout(flush, 800);
  }
  async function flush() {
    if (running) return;
    running = true;
    error = '';
    try {
      while (pending.size) {
        const [id, note] = pending.entries().next().value;
        status = 'Speichert …';
        await api(`/conferences/${id}`, { method: 'PUT', body: JSON.stringify(note) });
        if (pending.get(id) === note) {
          pending.delete(id);
          histories[id] = snapshots(histories[id] || [], note, Date.now(), true);
          await localWrite(note, histories[id] || [], false);
        }
      }
      status = 'Gespeichert';
    } catch (e) {
      error = e.message;
      status = 'Noch nicht gespeichert';
      if (e.status === 401 && !disposed) onerror(e);
    } finally { running = false; }
  }
  function create() {
    const now = new Date();
    const date = `${now.getFullYear()}-${String(now.getMonth()+1).padStart(2,'0')}-${String(now.getDate()).padStart(2,'0')}`;
    const note = { id: crypto.randomUUID(), date, title: null, classroom_id: null, content: '' };
    entries = [note, ...entries]; selected = note.id; historyOpen = false; preview = null;
    change({}, true);
  }
  async function remove() {
    if (!confirm('Diese Konferenz und ihren lokalen Verlauf löschen?')) return;
    const id = selected;
    clearTimeout(timer);
    await flush();
    if (running || pending.has(id)) return;
    try {
      await api(`/conferences/${id}`, { method: 'DELETE' });
      await queue;
      await forget(accountId, id);
      entries = entries.filter(n => n.id !== id);
      delete histories[id]; selected = entries[0]?.id || ''; preview = null;
    } catch(e) { error = e.message; }
  }
  function restore() {
    histories[selected] = snapshots(histories[selected] || [], current, Date.now(), true);
    change({ date: preview.note.date, title: preview.note.title, classroom_id: preview.note.classroom_id, content: preview.note.content }, true);
    preview = null;
  }
  function beforeUnload(event) {
    if (pending.size) { event.preventDefault(); event.returnValue = ''; }
  }
  onMount(() => {
    (async () => {
      let local = [];
      try { local = await records(accountId); } catch { localError = 'Lokale Wiederherstellung ist in diesem Browser nicht verfügbar.'; }
      try { entries = await api('/conferences'); }
      catch(e) { error = e.message; entries = local.map(x => x.note); if(e.status === 401) onerror(e); }
      for (const row of local) {
        histories[row.note.id] = row.history.filter(x => Date.now() - x.at < 7*86400000);
        localWrite(row.note, histories[row.note.id], row.pending);
        if (row.pending) {
          entries = [...entries.filter(n => n.id !== row.note.id), row.note];
          pending.set(row.note.id, row.note);
        }
      }
      selected = [...entries].sort((a,b) => b.date.localeCompare(a.date))[0]?.id || '';
      loading = false;
      if (pending.size) { status = 'Lokale Änderungen wiederhergestellt'; flush(); }
    })();
    return () => { disposed = true; clearTimeout(timer); flush(); };
  });
</script>

<svelte:window onbeforeunload={beforeUnload} ononline={flush} />
<section class="page-heading with-actions">
  <div><span class="eyebrow">RAUM FÜR AUSTAUSCH</span><h1>Konferenzen<span>.</span></h1><p>Gedanken, Absprachen und nächste Schritte.</p></div>
  <button class="primary" onclick={create} disabled={loading}>Neue Konferenz</button>
</section>
{#if error}<div class="alert" role="alert">{error} <button class="secondary" onclick={flush}>Erneut speichern</button></div>{/if}
{#if localError}<div class="alert" role="alert">{localError}</div>{/if}
{#if loading}<p>Lädt …</p>{:else}
<div class="conference-layout">
  <aside class="workspace-card conference-list" aria-label="Konferenzen">
    {#each ordered as note (note.id)}
      <button class:chosen={selected === note.id} onclick={() => { selected = note.id; preview = null; historyOpen = false; }}>
        <strong>{titleOf(note)}</strong><span>{note.date.split('-').reverse().join('.')}</span>
        {#if note.classroom_id}<small>{classrooms.find(c => c.id === note.classroom_id)?.name || 'Klasse'}</small>{/if}
      </button>
    {:else}<p>Noch keine Konferenzen. Erstelle deine erste Notiz.</p>{/each}
  </aside>
  {#if current}
    <section class="workspace-card conference-editor" aria-label="Konferenz bearbeiten">
      <div class="conference-fields">
        <label>Datum<input type="date" value={current.date} oninput={e => change({ date: e.currentTarget.value })} /></label>
        <label>Klasse (optional)<select value={current.classroom_id || ''} onchange={e => change({ classroom_id: e.currentTarget.value || null })}><option value="">Keine Klasse</option>{#each classrooms as c}<option value={c.id}>{c.name}</option>{/each}</select></label>
      </div>
      <label>Titel (optional)<input maxlength="200" value={current.title || ''} placeholder={titleOf({ ...current, title: null })} oninput={e => change({ title: e.currentTarget.value || null })} /></label>
      <label class="notepad-label">Notiz<textarea class="notepad" maxlength="100000" value={current.content} placeholder="Was möchtest du festhalten?" oninput={e => change({ content: e.currentTarget.value })}></textarea></label>
      <div class="conference-actions"><small role="status">{status || 'Gespeichert'}</small><button class="text-button" onclick={() => { historyOpen = !historyOpen; preview = null; }}>Browser-Verlauf</button><button class="text-button danger-text" onclick={remove}>Löschen</button></div>
      {#if historyOpen}
        <div class="conference-history"><p>Nur in diesem Browser · bis zu 20 Stände · 7 Tage</p>
          {#each (histories[selected] || []).filter(x => Date.now()-x.at < 7*86400000).slice().reverse() as snapshot}
            <button class="secondary" onclick={() => preview = snapshot}>{new Date(snapshot.at).toLocaleString('de-DE')}</button>
          {:else}<p>Noch kein lokaler Verlauf.</p>{/each}
          {#if preview}<h3>{titleOf(preview.note)}</h3><p>{preview.note.date} · {classrooms.find(c => c.id === preview.note.classroom_id)?.name || 'Keine Klasse'}</p><pre>{preview.note.content || '(Leere Notiz)'}</pre><button class="primary" onclick={restore}>Diesen Stand wiederherstellen</button>{/if}
        </div>
      {/if}
    </section>
  {/if}
</div>
{/if}
<style>
  .conference-layout { display:grid; grid-template-columns:260px minmax(0,1fr); gap:24px; align-items:start; }
  .conference-list { padding:12px; display:grid; gap:8px; }
  .conference-list button { display:grid; gap:6px; text-align:left; padding:16px; background:transparent; border:1px solid transparent; border-radius:10px; overflow-wrap:anywhere; }
  .conference-list button.chosen { background:#f1f4ed; border-color:#b8c5ae; }
  .conference-list span,.conference-list small { color:#666; }
  .conference-editor { padding:24px; display:grid; gap:20px; }
  .conference-fields { display:grid; grid-template-columns:1fr 1fr; gap:16px; }
  label { display:grid; gap:8px; }
  .notepad { min-height:55vh; resize:vertical; line-height:1.8; padding:24px; font-size:16px; background:#fffef9; }
  .conference-actions { display:flex; gap:16px; align-items:center; flex-wrap:wrap; }
  .conference-actions small { margin-right:auto; }
  .conference-history { border-top:1px solid #ddd; padding-top:16px; }
  .conference-history button { margin:4px; }
  pre { white-space:pre-wrap; overflow-wrap:anywhere; max-height:320px; overflow:auto; font:inherit; }
  @media(max-width:800px) { .conference-layout { grid-template-columns:1fr; } .conference-list { max-height:240px; overflow:auto; } .conference-fields { grid-template-columns:1fr; } .conference-editor { padding:16px; } }
</style>
