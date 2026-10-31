<script>
  import { onMount, tick } from "svelte";
  import { api } from "./api.js";
  import Icon from "./Icon.svelte";

  let {
    classId,
    students,
    tab = "open",
    subject = "",
    saving = $bindable(false),
    disabled = false,
    onerror,
    onsettings,
    onfilters,
  } = $props();
  let entries = $state([]),
    subjects = $state([]),
    loading = $state(true),
    detailLoading = $state(false);
  let detail = $state(null),
    error = $state(""),
    notice = $state("");
  let editor = $state(false),
    dialog = $state(null),
    editingId = $state(""),
    persistedId = $state("");
  let title = $state(""),
    description = $state(""),
    dueDate = $state(""),
    subjectId = $state(""),
    file = $state(null),
    formError = $state("");
  let pupilSearch = $state("");
  let notePupil = $state(null),
    noteDialog = $state(null),
    noteInput = $state(null),
    noteText = $state(""),
    noteError = $state("");
  const qualityOptions = [
    { value: "all_correct", label: "alles richtig" },
    { value: "partially_correct", label: "teilweise korrekt" },
    { value: "missing", label: "fehlt" },
    { value: "all_incorrect", label: "alles falsch" },
  ];
  let detailVersion = 0;
  const locked = $derived(disabled || saving || loading || detailLoading);
  const openCount = $derived(entries.filter((h) => h.status === "open").length);
  const historyCount = $derived(entries.length - openCount);
  const visible = $derived(
    entries
      .filter(
        (h) =>
          (tab === "open" ? h.status === "open" : h.status !== "open") &&
          (!subject || h.subject_id === subject),
      )
      .sort((a, b) =>
        tab === "history"
          ? (b.closed_at || 0) - (a.closed_at || 0)
          : a.due_date.localeCompare(b.due_date),
      ),
  );
  const pupils = $derived(
    (detail?.submissions || []).filter((s) =>
      s.name
        .toLocaleLowerCase("de")
        .includes(pupilSearch.toLocaleLowerCase("de")),
    ),
  );
  const assignment = $derived(detail?.homework);
  const remaining = $derived(
    assignment ? assignment.assigned_count - assignment.handed_in_count : 0,
  );
  const base = $derived(`/classrooms/${classId}/homework`);

  function today() {
    const d = new Date();
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  }
  function dateLabel(date) {
    return new Date(date + "T12:00:00").toLocaleDateString("de-DE");
  }
  function statusLabel(h) {
    return h.status === "auto_closed"
      ? "Alle haben abgegeben"
      : h.status === "manually_closed"
        ? "Manuell abgeschlossen"
        : h.due_date < today()
          ? "Überfällig"
          : "Offen";
  }
  function fail(e) {
    error = e.message;
    if (e.status === 401) onerror(e);
  }
  onMount(() => {
    let active = true;
    Promise.all([api(base), api(`/classrooms/${classId}/subjects`)])
      .then(([homework, list]) => {
        if (!active) return;
        entries = homework;
        subjects = list;
        if (subject && !list.some((s) => s.id === subject)) onfilters(tab, "");
      })
      .catch((e) => {
        if (active) fail(e);
      })
      .finally(() => {
        if (active) loading = false;
      });
    return () => {
      active = false;
      detailVersion += 1;
    };
  });
  async function refresh(id = assignment?.id) {
    const [list, data] = await Promise.all([
      api(base),
      id ? api(`${base}/${id}`) : Promise.resolve(null),
    ]);
    entries = list;
    detail = data;
  }
  async function reload() {
    if (locked) return;
    detailLoading = true;
    try {
      const [, list] = await Promise.all([
        refresh(),
        api(`/classrooms/${classId}/subjects`),
      ]);
      subjects = list;
      error = "";
    } catch (e) {
      fail(e);
    } finally {
      detailLoading = false;
    }
  }
  async function selectHomework(id) {
    if (locked) return;
    const version = ++detailVersion;
    detailLoading = true;
    error = "";
    notice = "";
    pupilSearch = "";
    try {
      const data = await api(`${base}/${id}`);
      if (version === detailVersion) detail = data;
    } catch (e) {
      if (version === detailVersion) fail(e);
    } finally {
      if (version === detailVersion) detailLoading = false;
    }
  }
  function filters(nextTab, nextSubject) {
    if (locked) return;
    detail = null;
    notice = "";
    onfilters(nextTab, nextSubject);
  }
  async function openEditor(h = null) {
    if (locked) return;
    editingId = h?.id || "";
    persistedId = "";
    title = h?.title || "";
    description = h?.description || "";
    dueDate = h?.due_date || today();
    subjectId = h?.subject_id || subject || subjects[0]?.id || "";
    file = null;
    formError = "";
    editor = true;
    await tick();
    dialog.showModal();
  }
  async function closeEditor() {
    if (saving) return;
    dialog.close();
    editor = false;
    if (persistedId) {
      try {
        await refresh(persistedId);
      } catch (e) {
        fail(e);
      }
    }
  }
  function chooseFile(e) {
    const candidate = e.currentTarget.files?.[0] || null;
    if (
      candidate &&
      (!candidate.name.toLowerCase().endsWith(".pdf") ||
        candidate.size > 10 * 1024 * 1024)
    ) {
      formError = "Bitte eine PDF-Datei mit höchstens 10 MB auswählen.";
      e.currentTarget.value = "";
      file = null;
      return;
    }
    formError = "";
    file = candidate;
  }
  async function saveEditor(e) {
    e.preventDefault();
    if (saving || disabled) return;
    saving = true;
    formError = "";
    error = "";
    try {
      const id = editingId || persistedId;
      const result = await api(id ? `${base}/${id}` : base, {
        method: id ? "PATCH" : "POST",
        body: JSON.stringify({
          title,
          description,
          due_date: dueDate,
          subject_id: subjectId,
        }),
      });
      const savedId = id || result.id;
      persistedId = savedId;
      if (file) {
        await api(
          `${base}/${savedId}/worksheet?name=${encodeURIComponent(file.name)}`,
          {
            method: "PUT",
            headers: { "Content-Type": "application/pdf" },
            body: file,
          },
        );
      }
      await refresh(savedId);
      onfilters(assignment.status === "open" ? "open" : "history", "");
      notice = editingId ? "Hausaufgabe gespeichert" : "Hausaufgabe erstellt";
      dialog.close();
      editor = false;
    } catch (e) {
      formError =
        (persistedId ? "Die Hausaufgabe ist gespeichert. " : "") + e.message;
      if (e.status === 401) onerror(e);
    } finally {
      saving = false;
    }
  }
  async function saveSubmissions(changes) {
    if (locked || !changes.length) return;
    saving = true;
    error = "";
    notice = "";
    const wasOpen = assignment.status === "open";
    try {
      await api(`${base}/${assignment.id}/submissions`, {
        method: "PATCH",
        body: JSON.stringify({ changes }),
      });
      await refresh();
      notice =
        wasOpen && assignment.status === "auto_closed"
          ? "Alle Kinder haben abgegeben. Die Hausaufgabe wurde automatisch abgeschlossen und liegt jetzt im Verlauf."
          : "Abgaben gespeichert";
    } catch (e) {
      fail(e);
    } finally {
      saving = false;
    }
  }
  function notePreview(note) {
    const characters = Array.from(note.replace(/\s+/g, " ").trim());
    const truncated = characters.length > 140;
    return {
      text:
        characters.slice(0, 140).join("").trimEnd() + (truncated ? "…" : ""),
      truncated,
    };
  }
  async function openPupilNote(pupil) {
    if (locked || notePupil) return;
    notePupil = pupil;
    noteText = pupil.note || "";
    noteError = "";
    await tick();
    noteDialog.showModal();
    noteInput.focus();
  }
  function closePupilNote() {
    if (saving) return;
    noteDialog.close();
    notePupil = null;
  }
  async function savePupilNote(event) {
    event.preventDefault();
    if (locked || !notePupil) return;
    saving = true;
    noteError = "";
    notice = "";
    try {
      await api(`${base}/${assignment.id}/submissions`, {
        method: "PATCH",
        body: JSON.stringify({
          changes: [{ student_id: notePupil.student_id, note: noteText }],
        }),
      });
      await refresh();
      noteDialog.close();
      notePupil = null;
      notice = noteText.trim() ? "Notiz gespeichert" : "Notiz entfernt";
    } catch (e) {
      noteError = e.message;
      if (e.status === 401) onerror(e);
    } finally {
      saving = false;
    }
  }
  async function setClosed(closed) {
    if (locked) return;
    saving = true;
    error = "";
    notice = "";
    try {
      await api(`${base}/${assignment.id}/closure`, {
        method: "PATCH",
        body: JSON.stringify({ closed }),
      });
      await refresh();
      notice = closed
        ? "Hausaufgabe manuell abgeschlossen. Die fehlenden Abgaben bleiben im Verlauf sichtbar."
        : "Hausaufgabe wieder geöffnet";
    } catch (e) {
      fail(e);
    } finally {
      saving = false;
    }
  }
  async function removeWorksheet() {
    if (locked) return;
    saving = true;
    error = "";
    try {
      await api(`${base}/${assignment.id}/worksheet`, { method: "DELETE" });
      await refresh();
      notice = "Arbeitsblatt entfernt";
    } catch (e) {
      fail(e);
    } finally {
      saving = false;
    }
  }
</script>

<section class="page-heading with-actions">
  <div>
    <span class="eyebrow">DEINE KLASSE · AUFGABEN IM BLICK</span>
    <h1>Hausaufgaben<span>.</span></h1>
    <p>Aufgaben stellen, Abgaben festhalten und den Überblick behalten.</p>
  </div>
  <button
    class="primary"
    disabled={locked || !subjects.length || !students.length}
    onclick={() => openEditor()}
    ><Icon name="plus" /> Hausaufgabe erstellen</button
  >
</section>
{#if error}<div class="alert" role="alert">
    {error}<button class="text-button" disabled={locked} onclick={reload}
      >Erneut laden</button
    >
  </div>{/if}
{#if notice}<div class="info-box workspace-notice" role="status">
    ✓ {notice}
  </div>{/if}
<section
  class="workspace-card homework-workspace"
  aria-label="Hausaufgaben"
  aria-busy={loading || saving}
>
  <div class="workspace-subheader homework-toolbar">
    <div class="view-tabs" role="tablist" aria-label="Hausaufgabenansicht">
      <button
        role="tab"
        aria-selected={tab === "open"}
        class:active={tab === "open"}
        disabled={locked}
        onclick={() => filters("open", subject)}
        >Offen <span>{openCount}</span></button
      >
      <button
        role="tab"
        aria-selected={tab === "history"}
        class:active={tab === "history"}
        disabled={locked}
        onclick={() => filters("history", subject)}
        >Verlauf <span>{historyCount}</span></button
      >
    </div>
    <label class="homework-subject-filter"
      >Fach<select
        value={subject}
        disabled={locked}
        onchange={(e) => filters(tab, e.currentTarget.value)}
        ><option value="">Alle Fächer</option
        >{#each subjects as s (s.id)}<option value={s.id}>{s.name}</option
          >{/each}</select
      ></label
    >
  </div>
  {#if loading}<div class="empty-panel">
      <p>Hausaufgaben werden geladen …</p>
    </div>
  {:else if !subjects.length}<div class="empty-panel">
      <h3>Welche Fächer hat deine Klasse?</h3>
      <p>Lege zuerst die Fächer in den Klasseneinstellungen an.</p>
      <button class="primary" disabled={locked} onclick={onsettings}
        >Fächer anlegen</button
      >
    </div>
  {:else if !students.length && !entries.length}<div class="empty-panel">
      <h3>Noch keine Kinder in der Klasse</h3>
      <p>
        Füge unter „Kinder“ die Klasse hinzu, bevor du eine Hausaufgabe
        erstellst.
      </p>
    </div>
  {:else}
    <div class="homework-layout" class:has-detail={!!detail || detailLoading}>
      <div class="homework-list">
        {#each visible as h (h.id)}
          <button
            class="homework-list-item"
            class:selected={assignment?.id === h.id}
            disabled={locked}
            onclick={() => selectHomework(h.id)}
          >
            <span class="homework-item-meta"
              ><span>{h.subject_name}</span><span
                class:overdue={h.status === "open" && h.due_date < today()}
                >{statusLabel(h)}</span
              ></span
            >
            <strong>{h.title}</strong>
            <span class="homework-item-meta"
              ><span>Abgabe: {dateLabel(h.due_date)}</span><span
                >{h.handed_in_count}/{h.assigned_count} abgegeben</span
              ></span
            >
            {#if h.worksheet_name}<small>▤ {h.worksheet_name}</small>{/if}
            <span class="homework-progress" aria-hidden="true"
              ><i
                style:width={`${h.assigned_count ? (h.handed_in_count / h.assigned_count) * 100 : 0}%`}
              ></i></span
            >
          </button>
        {:else}<div class="empty-panel">
            <h3>
              {tab === "open"
                ? "Keine offenen Hausaufgaben"
                : "Noch kein Verlauf"}
            </h3>
            <p>
              {subject
                ? "Für dieses Fach sind hier keine Hausaufgaben vorhanden."
                : tab === "open"
                  ? "Erstelle eine neue Hausaufgabe für deine Klasse."
                  : "Abgeschlossene Hausaufgaben und ihre Abgaben erscheinen hier."}
            </p>
          </div>{/each}
      </div>
      {#if detailLoading}<div class="empty-panel">
          <p>Abgaben werden geladen …</p>
        </div>
      {:else if assignment}
        <section class="homework-detail" aria-labelledby="homework-title">
          <div class="homework-detail-top">
            <span class="eyebrow">{assignment.subject_name}</span><button
              class="text-button"
              disabled={locked}
              onclick={() => {
                detail = null;
                notice = "";
              }}>Schließen</button
            >
          </div>
          <h2 id="homework-title">{assignment.title}</h2>
          <div class="homework-detail-meta">
            <span>Abgabe: {dateLabel(assignment.due_date)}</span><span
              class:overdue={assignment.status === "open" &&
                assignment.due_date < today()}>{statusLabel(assignment)}</span
            >
          </div>
          {#if assignment.closed_at}<p class="muted">
              Abgeschlossen am {new Date(
                assignment.closed_at * 1000,
              ).toLocaleDateString("de-DE")}
            </p>{/if}
          {#if assignment.description}<p class="homework-description">
              {assignment.description}
            </p>{/if}
          <div class="homework-detail-actions">
            <button
              class="secondary small"
              disabled={locked}
              onclick={() => openEditor(assignment)}
              >Bearbeiten{assignment.worksheet_name
                ? ""
                : " / PDF hinzufügen"}</button
            >
            {#if assignment.status === "open"}<button
                class="secondary small"
                disabled={locked}
                onclick={() => setClosed(true)}>Manuell abschließen</button
              >
            {:else if assignment.status === "manually_closed" && remaining > 0}<button
                class="secondary small"
                disabled={locked}
                onclick={() => setClosed(false)}>Wieder öffnen</button
              >{/if}
          </div>
          {#if assignment.worksheet_name}<div class="worksheet-row">
              <a
                class="text-button"
                href={`/api${base}/${assignment.id}/worksheet`}
                >↓ {assignment.worksheet_name}</a
              ><button
                class="text-button danger-text"
                disabled={locked}
                onclick={removeWorksheet}>PDF entfernen</button
              >
            </div>{/if}
          <div class="homework-submission-heading">
            <h3>
              Abgaben · {assignment.handed_in_count}/{assignment.assigned_count}
            </h3>
            <span>{remaining} offen</span>
          </div>
          <p class="muted">
            Sobald alle Kinder abgegeben haben, wird die Hausaufgabe automatisch
            abgeschlossen. Abgaben lassen sich auch im Verlauf korrigieren. Eine
            Qualitätsbewertung markiert die Abgabe ebenfalls; „fehlt“ setzt sie
            auf ausstehend.
          </p>
          <div class="inline-actions homework-bulk">
            <button
              class="secondary small"
              disabled={locked || !remaining}
              onclick={() =>
                saveSubmissions(
                  detail.submissions
                    .filter((s) => !s.handed_in_at)
                    .map((s) => ({
                      student_id: s.student_id,
                      handed_in: true,
                    })),
                )}>Alle als abgegeben markieren</button
            >
          </div>
          <label class="pupil-search"
            >Kind suchen<input
              bind:value={pupilSearch}
              placeholder="Name …"
            /></label
          >
          <div class="homework-pupils">
            {#each pupils as pupil (pupil.student_id)}
              {@const preview = notePreview(pupil.note || "")}
              <div
                class="homework-pupil"
                class:handed-in={!!pupil.handed_in_at}
              >
                <label class="homework-submission"
                  ><input
                    type="checkbox"
                    checked={!!pupil.handed_in_at}
                    disabled={locked}
                    onchange={(e) => {
                      const checked = e.currentTarget.checked;
                      e.currentTarget.checked = !!pupil.handed_in_at;
                      saveSubmissions([
                        { student_id: pupil.student_id, handed_in: checked },
                      ]);
                    }}
                  /><span class="homework-pupil-info"
                    ><strong>{pupil.name}</strong><span
                      >{pupil.handed_in_at
                        ? "Abgegeben · " +
                          new Date(
                            pupil.handed_in_at * 1000,
                          ).toLocaleDateString("de-DE")
                        : "Noch offen"}</span
                    ></span
                  ></label
                >
                <label class="homework-quality">
                  <span>Qualität</span>
                  <select
                    aria-label={`Qualität für ${pupil.name}`}
                    value={pupil.quality || ""}
                    disabled={locked}
                    onchange={(e) => {
                      const quality = e.currentTarget.value || null;
                      e.currentTarget.value = pupil.quality || "";
                      saveSubmissions([
                        { student_id: pupil.student_id, quality },
                      ]);
                    }}
                  >
                    <option value="">Nicht bewertet</option>
                    {#each qualityOptions as option}
                      <option value={option.value}>{option.label}</option>
                    {/each}
                  </select>
                </label>
                <div class="homework-pupil-note">
                  {#if pupil.note}<p>{preview.text}</p>{/if}
                  <button
                    class="text-button homework-note-action"
                    disabled={locked}
                    aria-label={`Notiz für ${pupil.name} ${pupil.note ? "öffnen" : "hinzufügen"}`}
                    onclick={() => openPupilNote(pupil)}
                    >{pupil.note
                      ? preview.truncated
                        ? "Notiz öffnen"
                        : "Bearbeiten"
                      : "Notiz hinzufügen"}</button
                  >
                </div>
              </div>
            {:else}<p class="muted">Keine passenden Kinder.</p>{/each}
          </div>
          <p class="muted">
            Die Kinderliste entspricht dem Stand beim Erstellen. Später
            hinzugefügte Kinder erhalten bestehende Hausaufgaben nicht
            nachträglich.
          </p>
          <div class="save-status" role="status">
            <span class="save-dot" class:pending={saving}></span>{saving
              ? "Wird gespeichert …"
              : "Abgaben werden automatisch gespeichert"}
          </div>
        </section>
      {/if}
    </div>
  {/if}
</section>

{#if notePupil}
  <dialog
    bind:this={noteDialog}
    aria-labelledby="homework-note-title"
    oncancel={(event) => {
      event.preventDefault();
      closePupilNote();
    }}
  >
    <form class="modal-content" onsubmit={savePupilNote}>
      <div class="modal-top">
        <span class="eyebrow">HAUSAUFGABENNOTIZ</span>
        <button
          class="icon-button"
          type="button"
          disabled={saving}
          aria-label="Dialog schließen"
          onclick={closePupilNote}>×</button
        >
      </div>
      <h2 id="homework-note-title">Notiz für {notePupil.name}</h2>
      <p>{assignment.title}</p>
      {#if noteError}<div class="alert" role="alert">{noteError}</div>{/if}
      <label>
        Notiz
        <textarea
          bind:this={noteInput}
          bind:value={noteText}
          maxlength="10000"
          rows="7"
          placeholder="Was möchtest du zu dieser Hausaufgabe festhalten?"
          disabled={saving}
        ></textarea>
      </label>
      {#if notePupil.note}<p class="muted">
          Leeren und speichern, um die Notiz zu entfernen.
        </p>{/if}
      <div class="modal-actions">
        <button
          class="secondary"
          type="button"
          disabled={saving}
          onclick={closePupilNote}>Abbrechen</button
        >
        <button class="primary" disabled={saving || disabled}>
          {saving ? "Wird gespeichert …" : "Speichern"}
        </button>
      </div>
    </form>
  </dialog>
{/if}

{#if editor}
  <dialog
    bind:this={dialog}
    aria-labelledby="homework-editor-title"
    oncancel={(e) => {
      e.preventDefault();
      closeEditor();
    }}
  >
    <form class="modal-content" onsubmit={saveEditor}>
      <div class="modal-top">
        <span class="eyebrow">HAUSAUFGABEN</span><button
          class="icon-button"
          type="button"
          disabled={saving}
          aria-label="Dialog schließen"
          onclick={closeEditor}>×</button
        >
      </div>
      <h2 id="homework-editor-title">
        {editingId ? "Hausaufgabe bearbeiten" : "Neue Hausaufgabe"}
      </h2>
      {#if formError}<div class="alert" role="alert">{formError}</div>{/if}
      <label
        >Fach<select bind:value={subjectId} required disabled={saving}
          >{#each subjects as s (s.id)}<option value={s.id}>{s.name}</option
            >{/each}</select
        ></label
      >
      <label
        >Titel<input
          bind:value={title}
          required
          maxlength="200"
          placeholder="z. B. Zahlenraum bis 100 üben"
          disabled={saving}
        /></label
      >
      <label
        >Beschreibung <span class="muted">(optional)</span><textarea
          bind:value={description}
          maxlength="10000"
          rows="4"
          placeholder="Was sollen die Kinder bearbeiten?"
          disabled={saving}
        ></textarea></label
      >
      <label
        >Abgabedatum<input
          type="date"
          bind:value={dueDate}
          min="0001-01-01"
          max="9999-12-31"
          required
          disabled={saving}
        /></label
      >
      <label
        >Arbeitsblatt als PDF <span class="muted"
          >(optional, höchstens 10 MB)</span
        ><input
          type="file"
          accept="application/pdf,.pdf"
          onchange={chooseFile}
          disabled={saving}
        /></label
      >
      {#if editingId && assignment?.worksheet_name}<p class="muted">
          Aktuell: {assignment.worksheet_name}. Ein neues PDF ersetzt das
          bisherige Arbeitsblatt.
        </p>{/if}
      <div class="modal-actions">
        <button
          class="secondary"
          type="button"
          disabled={saving}
          onclick={closeEditor}>Abbrechen</button
        ><button class="primary" disabled={saving}
          >{saving ? "Wird gespeichert …" : "Speichern"}</button
        >
      </div>
    </form>
  </dialog>
{/if}
