<script>
  import { onMount, tick } from "svelte";
  import { api } from "./api.js";
  import Icon from "./Icon.svelte";
  import { dateKey, dateLabel, taskDone, addDays } from "./teaching.js";
  let {
    classId,
    lessonId = "",
    compact = false,
    saving = $bindable(false),
    onerror = () => {},
  } = $props();
  let entries = $state([]),
    pupils = $state([]),
    lessons = $state([]),
    selected = $state(null);
  let loading = $state(true),
    error = $state(""),
    tab = $state("open"),
    modal = $state(false),
    dialog = $state(null);
  let editing = $state(null),
    title = $state(""),
    description = $state(""),
    mode = $state("once"),
    due = $state(""),
    linkedLesson = $state("");
  let deleting = $state(false);
  const visible = $derived(
    entries.filter(
      (t) =>
        (!compact || t.lesson_id === lessonId) &&
        (tab === "history" ? taskDone(t) : !taskDone(t)),
    ),
  );
  function fail(e) {
    error = e.message;
    if (e.status === 401) onerror(e);
  }
  onMount(reload);
  async function reload() {
    loading = true;
    try {
      entries = await api(`/classrooms/${classId}/tasks`);
      const today = dateKey();
      lessons = await api(
        `/lessons?from=${addDays(today, -365)}&until=${addDays(today, 366)}&classroom_id=${classId}`,
      );
      if (lessonId && !lessons.some((l) => l.id === lessonId))
        lessons = [await api(`/lessons/${lessonId}`), ...lessons];
      if (selected) await select(entries.find((t) => t.id === selected.id));
    } catch (e) {
      fail(e);
    } finally {
      loading = false;
    }
  }
  export async function openNew() {
    await edit(null);
  }
  async function edit(task) {
    editing = task;
    title = task?.title || "";
    description = task?.description || "";
    mode = task?.completion_mode || "once";
    due = task?.due_date || "";
    linkedLesson = task?.lesson_id || lessonId;
    error = "";
    deleting = false;
    modal = true;
    await tick();
    dialog.showModal();
  }
  function close() {
    if (!saving) {
      dialog.close();
      modal = false;
    }
  }
  async function select(task) {
    selected = task;
    pupils = [];
    if (task?.completion_mode === "per_pupil") {
      try {
        const result = await api(`/classrooms/${classId}/tasks/${task.id}`);
        if (selected?.id === task.id) pupils = result.pupils;
      } catch (e) {
        fail(e);
      }
    }
  }
  async function save(event) {
    event.preventDefault();
    saving = true;
    error = "";
    try {
      await api(
        `/classrooms/${classId}/tasks${editing ? "/" + editing.id : ""}`,
        {
          method: editing ? "PATCH" : "POST",
          body: JSON.stringify({
            title,
            description,
            completion_mode: mode,
            due_date: due || null,
            lesson_id: linkedLesson || null,
          }),
        },
      );
      saving = false;
      close();
      await reload();
    } catch (e) {
      fail(e);
    } finally {
      saving = false;
    }
  }
  async function complete(task, completed, studentId = null) {
    saving = true;
    error = "";
    try {
      await api(`/classrooms/${classId}/tasks/${task.id}/completion`, {
        method: "PATCH",
        body: JSON.stringify({ completed, student_id: studentId }),
      });
      await reload();
    } catch (e) {
      fail(e);
    } finally {
      saving = false;
    }
  }
  async function remove() {
    saving = true;
    try {
      await api(`/classrooms/${classId}/tasks/${editing.id}`, {
        method: "DELETE",
      });
      selected = null;
      saving = false;
      close();
      await reload();
    } catch (e) {
      fail(e);
    } finally {
      saving = false;
    }
  }
</script>

{#if !compact}
  <section class="page-heading with-actions">
    <div>
      <h1>Aufgaben</h1>
      <p>Das Wichtige im Blick. Für die Klasse und für jedes Kind.</p>
    </div>
    <button class="primary" onclick={openNew} disabled={saving}
      ><Icon name="plus" /> Neue Aufgabe</button
    >
  </section>
{:else}
  <div class="panel-heading">
    <h3><Icon name="check" /> Checkliste zur Stunde</h3>
    <button class="text-button" onclick={openNew} disabled={saving}
      ><Icon name="plus" /> Aufgabe</button
    >
  </div>
{/if}
{#if error && !modal}<div class="alert" role="alert">{error}</div>{/if}
{#if !compact}<div class="tab-row" aria-label="Aufgabenstatus">
    <button class:active={tab === "open"} onclick={() => (tab = "open")}
      >Offen</button
    ><button class:active={tab === "history"} onclick={() => (tab = "history")}
      >Erledigt</button
    >
  </div>{/if}
<div class:task-layout={!compact} class:has-detail={!compact && !!selected}>
  <section
    class:workspace-card={!compact}
    class="task-list"
    aria-busy={loading}
  >
    {#if loading}<p class="muted">Aufgaben werden geladen …</p>
    {:else if !visible.length}<div class="empty-panel small-empty">
        <Icon name="check" size={28} />
        <h3>
          {tab === "history"
            ? "Noch keine erledigten Aufgaben"
            : "Alles im Blick"}
        </h3>
        <p>
          {compact
            ? "Ergänze die Vorbereitung für diese Stunde."
            : "Erstelle eine Aufgabe für die Klasse oder für jedes Kind."}
        </p>
        {#if !compact}<button class="secondary" onclick={openNew}
            >Aufgabe erstellen</button
          >{/if}
      </div>
    {:else}{#each visible as task (task.id)}
        <div class="task-row" class:selected={selected?.id === task.id}>
          {#if task.completion_mode === "once"}<input
              type="checkbox"
              checked={taskDone(task)}
              disabled={saving}
              onchange={(e) => complete(task, e.currentTarget.checked)}
              aria-label={`${task.title} erledigen`}
            />
          {:else}<span class="subject-icon blue"><Icon name="users" /></span
            >{/if}
          <button class="task-content" onclick={() => select(task)}
            ><strong>{task.title}</strong><small
              class:overdue={task.due_date &&
                task.due_date < dateKey() &&
                !taskDone(task)}
            >
              {task.completion_mode === "per_pupil"
                ? `${task.completed_count} von ${task.assigned_count} erledigt`
                : "Einmal für die Klasse"}
              {#if task.due_date}
                · {dateLabel(task.due_date, {
                  day: "numeric",
                  month: "short",
                })}{/if}</small
            ></button
          >
          <button
            class="icon-button"
            onclick={() => edit(task)}
            disabled={saving}
            aria-label={`${task.title} bearbeiten`}><Icon name="edit" /></button
          >
        </div>
      {/each}{/if}
  </section>
  {#if selected}
    <aside class="workspace-card task-detail">
      <div class="panel-heading">
        <h2>{selected.title}</h2>
        <button
          class="icon-button"
          onclick={() => (selected = null)}
          aria-label="Aufgabe schließen"><Icon name="close" /></button
        >
      </div>
      {#if selected.description}<p class="preserve-lines">
          {selected.description}
        </p>{/if}
      {#if selected.lesson_id}<p class="muted">
          Mit einer Unterrichtsstunde verknüpft
        </p>{/if}
      {#if selected.completion_mode === "per_pupil"}<p class="muted">
          {selected.completed_count} von {selected.assigned_count} erledigt
        </p>
        {#each pupils as pupil (pupil.student_id)}<label class="pupil-check"
            ><input
              type="checkbox"
              checked={!!pupil.completed_at}
              onchange={(e) =>
                complete(selected, e.currentTarget.checked, pupil.student_id)}
              disabled={saving}
            /><span>{pupil.name}</span></label
          >{/each}
      {:else}<label class="pupil-check"
          ><input
            type="checkbox"
            checked={taskDone(selected)}
            onchange={(e) => complete(selected, e.currentTarget.checked)}
            disabled={saving}
          /><span>Aufgabe erledigt</span></label
        >{/if}
    </aside>
  {/if}
</div>
{#if modal}
  <dialog
    bind:this={dialog}
    aria-labelledby="task-title"
    oncancel={(e) => {
      e.preventDefault();
      close();
    }}
  >
    <form class="modal-content" onsubmit={save}>
      <div class="panel-heading">
        <h2 id="task-title">
          {editing ? "Aufgabe bearbeiten" : "Neue Aufgabe"}
        </h2>
        <button
          type="button"
          class="icon-button"
          onclick={close}
          disabled={saving}
          aria-label="Dialog schließen"><Icon name="close" /></button
        >
      </div>
      {#if error}<div class="alert" role="alert">{error}</div>{/if}
      <label
        >Titel<input
          bind:value={title}
          required
          maxlength="200"
          placeholder="Zum Beispiel: Einverständniserklärungen einsammeln"
        /></label
      >
      <label
        >Beschreibung <span class="muted">Optional</span><textarea
          bind:value={description}
          rows="3"
          maxlength="5000"
        ></textarea></label
      >
      <div class="form-grid">
        <label
          >Erledigung<select bind:value={mode} disabled={!!editing}
            ><option value="once">Einmal für die Klasse</option><option
              value="per_pupil">Für jedes Kind</option
            ></select
          ></label
        ><label
          >Fällig am <span class="muted">Optional</span><input
            type="date"
            bind:value={due}
          /></label
        >
      </div>
      <label
        >Unterrichtsstunde <span class="muted">Optional</span><select
          bind:value={linkedLesson}
          disabled={!!editing || compact}
          ><option value="">Allgemeine Klassenaufgabe</option
          >{#each lessons as lesson}<option value={lesson.id}
              >{dateLabel(lesson.lesson_date, {
                day: "numeric",
                month: "short",
              })} · {lesson.starts_at} · {lesson.subject_name}</option
            >{/each}</select
        ></label
      >
      {#if mode === "per_pupil" && !editing}<p class="muted">
          Die aktuelle Kinderliste wird beim Erstellen übernommen.
        </p>{/if}
      {#if deleting}<div class="info-box">
          <p>Aufgabe und alle zugehörigen Erledigungen löschen?</p>
          <button
            type="button"
            class="primary danger"
            onclick={remove}
            disabled={saving}>Endgültig löschen</button
          >
        </div>{/if}
      <div class="modal-actions">
        {#if editing}<button
            type="button"
            class="text-button danger-text"
            onclick={() => (deleting = !deleting)}
            disabled={saving}>Löschen</button
          >{/if}<button
          type="button"
          class="secondary"
          onclick={close}
          disabled={saving}>Abbrechen</button
        ><button class="primary" disabled={saving || !title.trim()}
          >{saving ? "Speichert …" : "Speichern"}</button
        >
      </div>
    </form>
  </dialog>
{/if}
