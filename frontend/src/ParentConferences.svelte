<script lang="ts">
  import { onMount } from 'svelte';
  import { api, errorMessage, errorStatus } from './api.ts';
  import { ParentConferenceSchema, ParentPupilSchema, ConferenceDocumentSchema, WorkspaceSchema } from './models.ts';
  import type { ParentConference, ParentPupil, ConferenceDocument, Period, ErrorHandler } from './models.ts';
  import { groupConferences, attendeeNames } from './parent-conferences.ts';
  import { snapshots, parentDraft, writeParentDraft, memoryGeneration } from './conference-store.ts';
  import type { Snapshot } from './conference-store.ts';
  let { accountId, onerror }: { accountId: string; onerror: ErrorHandler } = $props();
  const generation = memoryGeneration();
  function today() { const d = new Date(); return `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,'0')}-${String(d.getDate()).padStart(2,'0')}`; }
  let entries = $state<ParentConference[]>([]), pupils = $state<ParentPupil[]>([]), selected = $state<ParentConference | null>(null);
  let documents = $state<ConferenceDocument[]>([]), periods = $state<Period[]>([]), periodId = $state('');
  let date = $state(''), content = $state(''), attendees = $state(''), search = $state(''), pupilSearch = $state('');
  let newOpen = $state(false), newPupil = $state(''), newDate = $state(today());
  let error = $state(''), loading = $state(true), busy = $state(false), status = $state(''), dirty = $state(false), historyOpen = $state(false);
  let histories = $state<Record<string, Snapshot<{ content: string }>[]>>({});
  let timer: ReturnType<typeof setTimeout> | undefined, saving: Promise<boolean> | undefined;
  const groups = $derived(groupConferences(entries, search));
  const matches = $derived(pupils.filter(p => p.name.toLocaleLowerCase('de').includes(pupilSearch.trim().toLocaleLowerCase('de'))));
  function fail(e: unknown) { error = errorMessage(e); if (errorStatus(e) === 401) onerror(e); }
  function changed() {
    if (selected) writeParentDraft(accountId, { note: { ...$state.snapshot(selected), date, content, attendees: attendeeNames(attendees) }, attendees, history: $state.snapshot(histories[selected.id] ?? []) }, generation);
    dirty = true; status = 'Ungespeicherte Änderungen'; clearTimeout(timer); timer = setTimeout(() => { void save(); }, 800); }
  async function save(): Promise<boolean> {
    clearTimeout(timer);
    if (saving) { const success = await saving; if (!success) return false; return save(); }
    if (!selected || !dirty) return true;
    const entry = { ...selected, date, content, attendees: attendeeNames(attendees) };
    const rawAttendees = attendees;
    saving = (async () => {
      status = 'Speichert …';
      try {
        await api(`/parent-conferences/${entry.id}`, { method: 'PUT', body: JSON.stringify(entry) });
        entries = entries.map(n => n.id === entry.id ? entry : n);
        selected = entry;
        dirty = date !== entry.date || content !== entry.content || attendees !== rawAttendees;
        histories[entry.id] = snapshots(histories[entry.id] ?? [], { content: entry.content }, Date.now(), true);
        const draft = parentDraft(accountId);
        if (!dirty && draft?.note.id === entry.id && draft.note.content === entry.content && draft.note.date === entry.date && draft.attendees === rawAttendees) writeParentDraft(accountId, null, generation);
        status = dirty ? 'Ungespeicherte Änderungen' : 'Gespeichert'; error = ''; return true;
      } catch (e) { status = 'Noch nicht gespeichert'; fail(e); return false; }
    })();
    let success: boolean;
    try { success = await saving; } finally { saving = undefined; }
    return success && dirty ? save() : success;
  }
  async function reload() { entries = await api(ParentConferenceSchema.array(), '/parent-conferences'); selected = entries.find(n => n.id === selected?.id) ?? null; }
  async function select(entry: ParentConference) {
    if (busy || !await save() || busy) return;
    entry = entries.find(n => n.id === entry.id) ?? entry;
    busy = true; error = '';
    try {
      const [docs, workspace] = await Promise.all([
        api(ConferenceDocumentSchema.array(), `/parent-conferences/${entry.id}/documents`),
        api(WorkspaceSchema, `/classrooms/${entry.classroom_id}/workspace`),
      ]);
      selected = entry; date = entry.date; content = entry.content; attendees = entry.attendees.join('\n'); dirty = false;
      documents = docs; periods = workspace.periods; periodId = periods[0]?.id ?? ''; historyOpen = false; status = 'Gespeichert';
      histories[entry.id] ??= snapshots([], { content: entry.content });
    } catch (e) { fail(e); } finally { busy = false; }
  }
  async function create() {
    if (busy || !newPupil || !await save() || busy) return;
    busy = true; error = '';
    const id = crypto.randomUUID();
    try {
      await api(`/parent-conferences/${id}`, { method: 'PUT', body: JSON.stringify({ student_id: newPupil, date: newDate, content: '', attendees: [] }) });
      await reload(); newOpen = false;
      const entry = entries.find(n => n.id === id); busy = false; if (entry) await select(entry);
    } catch(e) { fail(e); } finally { busy = false; }
  }
  async function action(work: () => Promise<unknown>) {
    if (busy || !await save() || busy) return;
    busy = true; error = '';
    try { await work(); await reload(); } catch(e) { fail(e); } finally { busy = false; }
  }
  async function upload(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const files = [...(input.files ?? [])];
    const id = selected?.id;
    if (!id || !files.length) return;
    await action(async () => {
      for (const file of files) {
        if (!file.size || file.size > 10*1024*1024) throw new Error('Bitte Dateien mit 1 Byte bis 10 MB auswählen.');
        await api(`/parent-conferences/${id}/documents?name=${encodeURIComponent(file.name)}`, { method: 'POST', headers: { 'Content-Type': 'application/octet-stream' }, body: file });
      }
    });
    try { const docs = await api(ConferenceDocumentSchema.array(), `/parent-conferences/${id}/documents`); if (selected?.id === id) documents = docs; } catch(e) { fail(e); }
    input.value = '';
  }
  async function remove() {
    const id = selected?.id;
    if (!id || !confirm('Dieses Elterngespräch mit allen Dokumenten und dem gespeicherten Leistungsstand löschen?')) return;
    await action(async () => { await api(`/parent-conferences/${id}`, { method: 'DELETE' }); selected = null; delete histories[id]; });
  }
  function beforeUnload(e: BeforeUnloadEvent) { if (dirty) { e.preventDefault(); e.returnValue = ''; } }
  onMount(() => {
    void (async () => {
      try {
        [entries, pupils] = await Promise.all([api(ParentConferenceSchema.array(), '/parent-conferences'), api(ParentPupilSchema.array(), '/parent-conferences/pupils')]);
        const draft = parentDraft(accountId);
        const entry = entries.find(n => n.id === draft?.note.id);
        if (draft && entry) {
          await select(entry);
          if (selected) { date = draft.note.date; content = draft.note.content; attendees = draft.attendees; histories[entry.id] = draft.history; changed(); }
        }
      }
      catch(e) { fail(e); } finally { loading = false; }
    })();
    return () => { clearTimeout(timer); void save(); };
  });
</script>

<svelte:window onbeforeunload={beforeUnload} ononline={() => { void save(); }} />
<section class="page-heading with-actions">
  <div><span class="eyebrow">GEMEINSAM FÜR DAS KIND</span><h1>Elterngespräche<span>.</span></h1><p>Gespräche, Vereinbarungen und Lernentwicklung an einem Ort.</p></div>
  <button class="primary" disabled={loading || busy} onclick={() => newOpen = !newOpen}>Neues Elterngespräch</button>
</section>
{#if error}<div class="alert" role="alert">{error} {#if dirty}<button class="secondary" onclick={() => { void save(); }}>Erneut speichern</button>{/if}</div>{/if}
{#if newOpen}
  <form class="workspace-card create-form" onsubmit={e => { e.preventDefault(); void create(); }}>
    <h2>Neues Elterngespräch</h2>
    <label>Kind suchen<input type="search" bind:value={pupilSearch} oninput={() => newPupil = ''} placeholder="Name des Kindes" /></label>
    <label>Kind<select bind:value={newPupil} required><option value="">Kind auswählen</option>{#each matches as p}<option value={p.id}>{p.name} · {p.class_name}</option>{/each}</select></label>
    {#if !matches.length}<p>Keine passenden Kinder in deinen Klassen.</p>{/if}
    <label>Datum<input type="date" bind:value={newDate} required /></label>
    <div><button class="primary" disabled={busy || !newPupil}>Erstellen</button> <button type="button" class="secondary" onclick={() => newOpen = false}>Abbrechen</button></div>
  </form>
{/if}
{#if loading}<p>Lädt …</p>{:else}
<div class="parent-layout">
  <aside class="workspace-card meeting-list" aria-label="Elterngespräche nach Tag">
    <label>Nach Kind suchen<input type="search" bind:value={search} placeholder="Name des Kindes" /></label>
    {#each groups as group (group.date)}
      <section class="day">
        <h2>{group.date.split('-').reverse().join('.')}</h2>
        <small>{group.date > today() ? 'Geplant' : 'Gespräche'}: {group.meetings.length} · Kinder: {group.pupils} · Anwesende Eltern: {group.parents}</small>
        {#each group.meetings as entry (entry.id)}
          <button class:chosen={selected?.id === entry.id} disabled={busy} onclick={() => { void select(entry); }}><strong>{entry.student_name}</strong><span>{entry.class_name}</span></button>
        {/each}
      </section>
    {:else}<p>{search ? 'Keine Gespräche für dieses Kind gefunden.' : 'Noch keine Elterngespräche. Lege das erste Gespräch an.'}</p>{/each}
    {#if search && groups.length}<small>Die Tageszahlen beziehen sich auf die Suchergebnisse.</small>{/if}
  </aside>
  {#if selected}
    <section class="workspace-card meeting-editor" aria-label="Elterngespräch bearbeiten">
      <h2>{selected.student_name} <small>· {selected.class_name}</small></h2>
      <fieldset disabled={busy}>
        <label>Datum<input type="date" bind:value={date} oninput={() => queueMicrotask(changed)} required /></label>
        <label>Anwesende Eltern / Erziehungsberechtigte<textarea rows="3" maxlength="4020" bind:value={attendees} oninput={() => queueMicrotask(changed)} placeholder="Ein Name pro Zeile. Nur tatsächlich anwesende Personen eintragen."></textarea></label>
        <label>Notiz<textarea class="notepad" maxlength="100000" bind:value={content} oninput={() => queueMicrotask(changed)} placeholder="Gespräch, Vereinbarungen und nächste Schritte …"></textarea></label>
      </fieldset>
      <div class="actions"><small role="status">{status}</small><button class="secondary" disabled={busy || !dirty} onclick={() => { void save(); }}>Speichern</button><button class="text-button" onclick={() => historyOpen = !historyOpen}>Notizverlauf</button><button class="text-button danger-text" disabled={busy} onclick={remove}>Löschen</button></div>
      {#if historyOpen}<div><p>Nur während dieser Sitzung · bis zu 20 Stände · 7 Tage</p>{#each [...(histories[selected.id] ?? [])].reverse() as snapshot}<details><summary>{new Date(snapshot.at).toLocaleString('de-DE')}</summary><pre>{snapshot.note.content || '(Leere Notiz)'}</pre><button class="secondary" disabled={busy} onclick={() => { content = snapshot.note.content; changed(); }}>Notiz wiederherstellen</button></details>{/each}</div>{/if}
      <section class="attachments"><h3>Dokumente</h3><label>Dokumente hochladen (je höchstens 10 MB)<input type="file" multiple disabled={busy} onchange={upload} /></label>
        {#each documents as doc (doc.id)}<div class="document"><a href={`/api/parent-conferences/${selected.id}/documents/${doc.id}`}>{doc.name}</a><small>{Math.ceil(doc.size/1024)} KB</small><button class="text-button danger-text" disabled={busy} onclick={() => { const id = selected?.id; if (id && confirm('Dieses Dokument löschen?')) void action(async () => { await api(`/parent-conferences/${id}/documents/${doc.id}`, { method: 'DELETE' }); documents = documents.filter(d => d.id !== doc.id); }); }}>Entfernen</button></div>{:else}<p>Noch keine Dokumente.</p>{/each}
      </section>
      <section class="review"><h3>Leistungsstand als Momentaufnahme</h3>
        {#if selected.review_snapshot}
          {@const review = selected.review_snapshot}
          <p>{review.period_name} · {review.school_year} · gespeichert am {new Date(review.captured_at*1000).toLocaleString('de-DE')}</p>
          <p>Dieser Stand bleibt bei späteren Änderungen der Lernentwicklung unverändert.</p>
          <details><summary>Gespeicherten Leistungsstand ansehen ({review.assessments.filter(a => a.value !== null).length} bewertete Kompetenzen)</summary><div class="review-table"><table><thead><tr><th>Kompetenz</th><th>Stand</th><th>Erreicht am</th></tr></thead><tbody>{#each review.assessments as a}<tr><td><small>{a.domain} · {a.section} · {a.level}</small><br />{a.label}</td><td>{a.value === 'yes' ? 'Erreicht' : a.value === 'no' ? 'Noch nicht erreicht' : 'Nicht bewertet'}</td><td>{a.achieved_on || '—'}</td></tr>{/each}</tbody></table></div></details>
          <button class="text-button danger-text" disabled={busy} onclick={() => { const id = selected?.id; if (id && confirm('Gespeicherten Leistungsstand entfernen?')) void action(() => api(`/parent-conferences/${id}/review`, { method: 'DELETE' })); }}>Momentaufnahme entfernen</button>
        {:else}
          <p>Den aktuellen Stand der Lernentwicklung für dieses Gespräch unveränderlich speichern.</p>
          <label>Bewertungszeitraum<select bind:value={periodId} disabled={busy}><option value="">Zeitraum auswählen</option>{#each periods as period}<option value={period.id}>{period.name}</option>{/each}</select></label>
          <button class="secondary" disabled={busy || !periodId} onclick={() => { const id = selected?.id; if (id) void action(() => api(`/parent-conferences/${id}/review`, { method: 'POST', body: JSON.stringify({ period_id: periodId }) })); }}>Leistungsstand festhalten</button>
          {#if !periods.length}<p>Für diese Klasse gibt es noch keinen Bewertungszeitraum.</p>{/if}
        {/if}
      </section>
    </section>
  {:else}<section class="workspace-card empty-editor"><p>Wähle ein Gespräch aus oder erstelle ein neues.</p></section>{/if}
</div>
{/if}
<style>
  .parent-layout { display:grid; grid-template-columns:300px minmax(0,1fr); gap:24px; align-items:start; }
  .meeting-list,.meeting-editor,.create-form,.empty-editor { padding:24px; display:grid; gap:20px; min-width:0; }
  .create-form { margin-bottom:24px; }
  label { display:grid; gap:8px; }
  fieldset { border:0; padding:0; margin:0; display:grid; gap:20px; min-width:0; }
  .day { display:grid; gap:8px; }
  .day h2 { font-size:18px; margin:0; }
  small { color:#666; }
  .day button { display:grid; gap:6px; text-align:left; padding:14px; background:transparent; border:1px solid #ddd; border-radius:10px; overflow-wrap:anywhere; }
  .day button.chosen { background:#f1f4ed; border-color:#b8c5ae; }
  .day span { color:#666; }
  .notepad { min-height:35vh; resize:vertical; line-height:1.8; padding:20px; font-size:16px; background:#fffef9; }
  .actions,.document { display:flex; gap:12px; align-items:center; flex-wrap:wrap; }
  .actions small { margin-right:auto; }
  .document a { overflow-wrap:anywhere; }
  .attachments,.review { border-top:1px solid #ddd; padding-top:16px; display:grid; gap:12px; }
  pre { white-space:pre-wrap; overflow-wrap:anywhere; max-height:320px; overflow:auto; font:inherit; }
  .review-table { overflow:auto; max-height:480px; }
  th,td { text-align:left; padding:10px; border-bottom:1px solid #ddd; }
  @media(max-width:850px) { .parent-layout { grid-template-columns:1fr; } .meeting-list { max-height:420px; overflow:auto; } .meeting-editor { padding:16px; } }
</style>
