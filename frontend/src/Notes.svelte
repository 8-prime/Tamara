<script>
  import { onMount, tick } from "svelte";
  import { api } from "./api.js";
  import Icon from "./Icon.svelte";
  import { initials } from "./teaching.js";

  let {
    classId,
    students,
    saving = $bindable(false),
    disabled = false,
    onerror,
    onstudents,
    context = null,
    className = "",
  } = $props();
  let entries = $state([]),
    loading = $state(true),
    error = $state(""),
    notice = $state("");
  let search = $state(""),
    pupilFilter = $state("");
  let step = $state(""),
    dialog = $state(null),
    nameInput = $state(null),
    noteInput = $state(null),
    newButton = $state(null);
  let nameSearch = $state(""),
    selectedIndex = $state(0),
    studentId = $state(""),
    content = $state(""),
    editing = $state(null),
    formError = $state(""),
    deleting = $state(false);
  const base = $derived(`/classrooms/${classId}/notes`);
  const locked = $derived(disabled || saving);
  const pupil = $derived(students.find((s) => s.id === studentId));
  const pupilNames = $derived(new Map(students.map((s) => [s.id, s.name])));
  const matches = $derived(
    students.filter((s) => normalize(s.name).includes(normalize(nameSearch))),
  );
  const visible = $derived(
    entries.filter(
      (note) =>
        (!pupilFilter || note.student_id === pupilFilter) &&
        normalize(
          `${pupilNames.get(note.student_id) || ""}\n${note.content}`,
        ).includes(normalize(search)),
    ),
  );

  function normalize(value) {
    return value.trim().toLocaleLowerCase("de");
  }
  function dateLabel(timestamp) {
    return new Date(timestamp * 1000).toLocaleString("de-DE", {
      dateStyle: "medium",
      timeStyle: "short",
    });
  }
  function duplicateLabel(student) {
    return students.filter((s) => s.name === student.name).length > 1
      ? `Kind ${students.findIndex((s) => s.id === student.id) + 1}`
      : "";
  }
  function fail(e) {
    error = e.message;
    if (e.status === 401) onerror(e);
  }
  onMount(() => {
    let active = true;
    api(base)
      .then((list) => {
        if (active) entries = list;
      })
      .catch((e) => {
        if (active) fail(e);
      })
      .finally(() => {
        if (active) loading = false;
      });
    return () => {
      active = false;
    };
  });
  async function reload() {
    if (locked || loading) return;
    loading = true;
    error = "";
    try {
      entries = await api(base);
    } catch (e) {
      fail(e);
    } finally {
      loading = false;
    }
  }
  export async function openNew() {
    if (locked || step || !students.length) return;
    editing = null;
    content = "";
    studentId = "";
    nameSearch = "";
    selectedIndex = 0;
    formError = "";
    deleting = false;
    notice = "";
    step = "select";
    await tick();
    dialog.showModal();
    nameInput.focus();
  }
  async function openEditor(note) {
    if (locked || step) return;
    editing = note;
    studentId = note.student_id;
    content = note.content;
    formError = "";
    deleting = false;
    step = "write";
    await tick();
    dialog.showModal();
    noteInput.focus();
  }
  async function chooseStudent(student) {
    if (!student || locked) return;
    studentId = student.id;
    step = "write";
    formError = "";
    await tick();
    noteInput.focus();
  }
  function selectionKey(event) {
    if (event.isComposing) return;
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      if (!matches.length) return;
      selectedIndex =
        (selectedIndex +
          (event.key === "ArrowDown" ? 1 : -1) +
          matches.length) %
        matches.length;
      document
        .getElementById(`note-pupil-${selectedIndex}`)
        ?.scrollIntoView({ block: "nearest" });
    } else if (event.key === "Enter") {
      event.preventDefault();
      if (!event.repeat) chooseStudent(matches[selectedIndex]);
    }
  }
  function noteKey(event) {
    if (event.key === "Enter" && !event.shiftKey && !event.isComposing) {
      event.preventDefault();
      if (!event.repeat && !deleting) save();
    }
  }
  async function closeEditor() {
    if (saving) return;
    dialog.close();
    step = "";
    // The overview must not retain text-input focus: n starts the next note.
    await tick();
    newButton?.focus();
  }
  async function save() {
    if (locked || loading || !content.trim() || step !== "write") return;
    saving = true;
    formError = "";
    try {
      const result = await api(editing ? `${base}/${editing.id}` : base, {
        method: editing ? "PATCH" : "POST",
        body: JSON.stringify({
          student_id: studentId,
          content,
          ...(!editing && context ? { lesson_id: context.id } : {}),
        }),
      });
      entries = editing
        ? entries.map((note) => (note.id === result.id ? result : note))
        : [result, ...entries];
      // A fresh note should be visible immediately, even after a filtered search.
      if (!editing) {
        search = "";
        pupilFilter = "";
      }
      notice = "Notiz gespeichert. Bereit für die nächste Notiz mit N.";
      saving = false;
      closeEditor();
    } catch (e) {
      formError = e.message;
      if (e.status === 401) onerror(e);
    } finally {
      saving = false;
      await tick();
      if (step === "write" && !deleting) noteInput?.focus();
    }
  }
  async function remove() {
    if (locked || !editing) return;
    saving = true;
    formError = "";
    try {
      await api(`${base}/${editing.id}`, { method: "DELETE" });
      entries = entries.filter((note) => note.id !== editing.id);
      notice = "Notiz gelöscht.";
      saving = false;
      closeEditor();
    } catch (e) {
      formError = e.message;
      if (e.status === 401) onerror(e);
    } finally {
      saving = false;
    }
  }
</script>

<section class="page-heading with-actions">
  <div>
    <span class="eyebrow">DEINE KLASSE · JEDES KIND IM BLICK</span>
    <h1>Notizen<span>.</span></h1>
    <p>
      Beobachtungen festhalten und wiederfinden. Neue Notiz mit <kbd>N</kbd>.
    </p>
  </div>
  <button
    class="primary"
    bind:this={newButton}
    disabled={locked || !students.length}
    onclick={openNew}
    aria-keyshortcuts="n"><Icon name="plus" /> Neue Notiz <kbd>N</kbd></button
  >
</section>

{#if error}<div class="alert" role="alert">{error}</div>{/if}
{#if notice}<p class="workspace-notice green-text" role="status">
    {notice}
  </p>{/if}

<section class="workspace-card notes-workspace" aria-label="Notizenübersicht">
  <div class="notes-toolbar">
    <label
      >Notizen durchsuchen
      <input type="search" bind:value={search} placeholder="Text oder Name …" />
    </label>
    <label
      >Kind
      <select bind:value={pupilFilter}>
        <option value="">Alle Kinder</option>
        {#each students as student (student.id)}
          <option value={student.id}
            >{student.name}{duplicateLabel(student)
              ? ` · ${duplicateLabel(student)}`
              : ""}</option
          >
        {/each}
      </select>
    </label>
    <button class="secondary" onclick={reload} disabled={locked || loading}
      >Aktualisieren</button
    >
  </div>
  {#if loading}
    <div class="empty-panel"><p>Notizen werden geladen …</p></div>
  {:else if !students.length}
    <div class="empty-panel">
      <h3>Wer lernt in deiner Klasse?</h3>
      <p>Füge zuerst die Kinder deiner Klasse hinzu.</p>
      <button class="primary" onclick={onstudents}>Kinder verwalten</button>
    </div>
  {:else if !visible.length}
    <div class="empty-panel">
      <h3>
        {search || pupilFilter
          ? "Keine passenden Notizen"
          : "Platz für deine Beobachtungen"}
      </h3>
      <p>
        {search || pupilFilter
          ? "Versuche einen anderen Suchtext oder wähle ein anderes Kind."
          : "Drücke N, wähle ein Kind und halte deine erste Notiz fest."}
      </p>
    </div>
  {:else}
    <div class="notes-count muted">
      {visible.length}
      {visible.length === 1 ? "Notiz" : "Notizen"} · Neueste zuerst
    </div>
    <ul class="notes-list">
      {#each visible as note (note.id)}
        <li class="note-card">
          <div class="note-heading">
            <span class="note-avatar"
              >{initials(pupilNames.get(note.student_id))}</span
            >
            <div>
              <strong>{pupilNames.get(note.student_id)}</strong>
              {#if students.find((s) => s.id === note.student_id)}
                <small class="muted"
                  >{duplicateLabel(
                    students.find((s) => s.id === note.student_id),
                  )}</small
                >
              {/if}
              <time datetime={new Date(note.created_at * 1000).toISOString()}
                >{dateLabel(note.created_at)}</time
              >
              {#if note.lesson_id}<small class="muted"
                  >Mit Unterrichtsstunde verknüpft</small
                >{/if}
            </div>
            <button
              class="text-button"
              disabled={locked}
              onclick={() => openEditor(note)}
              aria-label={`Notiz für ${pupilNames.get(note.student_id)} vom ${dateLabel(note.created_at)} bearbeiten`}
              >Bearbeiten</button
            >
          </div>
          <p class="note-text">{note.content}</p>
          {#if note.updated_at > note.created_at}<small class="muted"
              >Bearbeitet: {dateLabel(note.updated_at)}</small
            >{/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>

{#if step}
  <dialog
    bind:this={dialog}
    aria-labelledby="note-dialog-title"
    oncancel={(event) => {
      event.preventDefault();
      closeEditor();
    }}
  >
    <form
      class="modal-content"
      onsubmit={(event) => {
        event.preventDefault();
        if (step === "write" && !deleting) save();
      }}
    >
      <div class="modal-top">
        <h2 id="note-dialog-title">
          {editing ? "Notiz bearbeiten" : "Neue Notiz"}
        </h2>
        <button
          type="button"
          class="icon-button"
          aria-label="Dialog schließen"
          onclick={closeEditor}
          disabled={saving}>×</button
        >
      </div>
      {#if !editing}<div class="note-steps">
          <span class:active={step === "select"}><b>1</b> Kind auswählen</span
          ><i></i><span class:active={step === "write"}
            ><b>2</b> Notiz festhalten</span
          >
        </div>{/if}
      <div class="note-context">
        <Icon name="book" size={28} />
        <div>
          <strong
            >{context
              ? `Aktueller Kontext: ${context.subject_name} · Klasse ${context.class_name}`
              : `Klasse ${className}`}</strong
          >
          <p>Es werden nur Kinder aus dieser Klasse angezeigt.</p>
        </div>
      </div>
      {#if step === "write"}<h3>{pupil?.name}</h3>{/if}
      {#if formError}<div class="alert" role="alert">{formError}</div>{/if}
      {#if step === "select"}
        <label for="note-name">Name suchen</label>
        <input
          id="note-name"
          bind:this={nameInput}
          bind:value={nameSearch}
          oninput={() => {
            selectedIndex = 0;
          }}
          onkeydown={selectionKey}
          placeholder="Name eingeben …"
          autocomplete="off"
          role="combobox"
          aria-autocomplete="list"
          aria-expanded="true"
          aria-controls="note-pupils"
          aria-activedescendant={matches.length
            ? `note-pupil-${selectedIndex}`
            : undefined}
          aria-describedby="note-selection-help"
        />
        <p id="note-selection-help" class="note-help">
          <kbd>↑</kbd> <kbd>↓</kbd> auswählen · <kbd>Enter</kbd> weiter
        </p>
        <ul
          id="note-pupils"
          class="note-pupil-list"
          role="listbox"
          aria-label="Kinder"
        >
          {#each matches as student, index (student.id)}
            <li
              id={`note-pupil-${index}`}
              role="option"
              aria-selected={selectedIndex === index}
            >
              <button
                type="button"
                class:active={selectedIndex === index}
                onclick={() => chooseStudent(student)}
              >
                <span class="note-avatar">{initials(student.name)}</span>
                <span
                  >{student.name}
                  {#if duplicateLabel(student)}<small
                      >{duplicateLabel(student)}</small
                    >{/if}</span
                >
                {#if selectedIndex === index}<span aria-hidden="true">↵</span
                  >{/if}
              </button>
            </li>
          {/each}
        </ul>
        {#if !matches.length}<p class="muted" role="status">
            Kein passendes Kind gefunden.
          </p>{/if}
        <div class="modal-actions">
          <button type="button" class="secondary" onclick={closeEditor}
            >Abbrechen</button
          ><button
            type="button"
            class="primary"
            onclick={() => chooseStudent(matches[selectedIndex])}
            disabled={!matches.length}>Weiter <Icon name="arrow" /></button
          >
        </div>
      {:else}
        <p class="note-date">
          {editing
            ? dateLabel(editing.created_at)
            : new Date().toLocaleDateString("de-DE", {
                dateStyle: "long",
              })}{pupil && duplicateLabel(pupil)
            ? ` · ${duplicateLabel(pupil)}`
            : ""}
        </p>
        <label for="note-content">Notiz</label>
        <textarea
          id="note-content"
          bind:this={noteInput}
          bind:value={content}
          onkeydown={noteKey}
          rows="7"
          maxlength="10000"
          placeholder="Was möchtest du festhalten?"
          aria-describedby="note-write-help"
          disabled={saving || deleting}
        ></textarea>
        <p id="note-write-help" class="note-help">
          <kbd>Enter</kbd> speichern & schließen · <kbd>Shift</kbd> +
          <kbd>Enter</kbd> neue Zeile
        </p>
        {#if deleting}
          <div class="info-box">
            <p>Diese Notiz löschen?</p>
            <div class="modal-actions">
              <button
                type="button"
                class="secondary"
                onclick={() => {
                  deleting = false;
                }}
                disabled={saving}>Behalten</button
              >
              <button
                type="button"
                class="primary danger"
                onclick={remove}
                disabled={saving}>Notiz löschen</button
              >
            </div>
          </div>
        {:else}
          <div class="modal-actions note-actions">
            {#if editing}
              <button
                type="button"
                class="text-button danger-text"
                onclick={() => {
                  deleting = true;
                }}
                disabled={saving}>Löschen</button
              >
            {:else}
              <button
                type="button"
                class="text-button"
                onclick={async () => {
                  step = "select";
                  await tick();
                  nameInput.focus();
                }}
                disabled={saving}>Anderes Kind</button
              >
            {/if}
            <button
              type="button"
              class="secondary"
              onclick={closeEditor}
              disabled={saving}>Abbrechen</button
            >
            <button
              type="submit"
              class="primary"
              disabled={locked || loading || !content.trim()}
              >{saving ? "Speichert …" : "Speichern & schließen"}</button
            >
          </div>
        {/if}
      {/if}
    </form>
  </dialog>
{/if}
