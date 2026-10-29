<script>
  import { onMount, untrack } from "svelte";
  import { api } from "./api.js";
  import Icon from "./Icon.svelte";
  import { dateKey } from "./teaching.js";
  let {
    classrooms,
    lesson = null,
    initialDate = dateKey(),
    initialClassId = "",
    initialStart = "08:00",
    initialEnd = "08:45",
    saving = $bindable(false),
    onsaved,
    onclose,
    onclasses,
  } = $props();
  let dialog = $state(null),
    subjects = $state([]),
    error = $state(""),
    subjectLoading = $state(false);
  let classId = $state(
    untrack(
      () => lesson?.classroom_id || initialClassId || classrooms[0]?.id || "",
    ),
  );
  let subjectId = $state(untrack(() => lesson?.subject_id || "")),
    date = $state(untrack(() => lesson?.lesson_date || initialDate));
  let start = $state(untrack(() => lesson?.starts_at || initialStart)),
    end = $state(untrack(() => lesson?.ends_at || initialEnd));
  let room = $state(untrack(() => lesson?.room || "")),
    topic = $state(untrack(() => lesson?.topic || "")),
    goals = $state(untrack(() => lesson?.learning_goals || ""));
  let repeat = $state(false),
    repeatUntil = $state(`${new Date().getFullYear() + 1}-07-31`);
  let status = $state(untrack(() => lesson?.status || "scheduled")),
    newClass = $state(false),
    className = $state(""),
    newSubject = $state(false),
    subjectName = $state("");
  const year = new Date().getFullYear() - (new Date().getMonth() < 7 ? 1 : 0);
  let schoolYear = $state(`${year}/${String(year + 1).slice(-2)}`);
  onMount(() => dialog.showModal());
  $effect(() => {
    const id = classId;
    let active = true;
    subjects = [];
    subjectLoading = true;
    if (id)
      api(`/classrooms/${id}/subjects`)
        .then((data) => {
          if (active) {
            subjects = data;
            if (!data.some((s) => s.id === subjectId)) subjectId = "";
          }
        })
        .catch((e) => {
          if (active) error = e.message;
        })
        .finally(() => {
          if (active) subjectLoading = false;
        });
    else subjectLoading = false;
    return () => (active = false);
  });
  function close() {
    if (!saving) {
      dialog.close();
      onclose();
    }
  }
  async function addClass() {
    saving = true;
    error = "";
    try {
      const result = await api("/classrooms", {
        method: "POST",
        body: JSON.stringify({ name: className, school_year: schoolYear }),
      });
      await onclasses();
      classId = result.id;
      subjectId = "";
      newClass = false;
    } catch (e) {
      error = e.message;
    } finally {
      saving = false;
    }
  }
  async function addSubject() {
    saving = true;
    error = "";
    try {
      const result = await api(`/classrooms/${classId}/subjects`, {
        method: "POST",
        body: JSON.stringify({ name: subjectName }),
      });
      subjects = await api(`/classrooms/${classId}/subjects`);
      subjectId = result.id;
      newSubject = false;
    } catch (e) {
      error = e.message;
    } finally {
      saving = false;
    }
  }
  async function save(event) {
    event.preventDefault();
    saving = true;
    error = "";
    try {
      const data = {
        lesson_date: date,
        starts_at: start,
        ends_at: end,
        room,
        topic,
        learning_goals: goals,
        status,
      };
      if (!lesson)
        Object.assign(data, {
          classroom_id: classId,
          subject_id: subjectId,
          repeat_until: repeat ? repeatUntil : null,
        });
      const result = await api(`/lessons${lesson ? "/" + lesson.id : ""}`, {
        method: lesson ? "PATCH" : "POST",
        body: JSON.stringify(data),
      });
      await onsaved(lesson?.id || result.id, date);
      saving = false;
      close();
    } catch (e) {
      error = e.message;
    } finally {
      saving = false;
    }
  }
</script>

<dialog
  bind:this={dialog}
  class="lesson-editor"
  aria-labelledby="lesson-form-title"
  oncancel={(e) => {
    e.preventDefault();
    close();
  }}
>
  <form class="modal-content" onsubmit={save}>
    <div class="panel-heading">
      <div>
        <h2 id="lesson-form-title">
          {lesson ? "Stunde bearbeiten" : "Stunde hinzufügen"}
        </h2>
        <p class="muted">
          {lesson
            ? "Änderungen gelten nur für diese Stunde."
            : "Klasse und Fach geben deiner Stunde den Kontext."}
        </p>
      </div>
      <button
        type="button"
        class="icon-button"
        onclick={close}
        disabled={saving}
        aria-label="Dialog schließen"><Icon name="close" /></button
      >
    </div>
    {#if error}<div class="alert" role="alert">{error}</div>{/if}
    <div class="form-grid">
      <label
        >Klasse<select
          bind:value={classId}
          required
          disabled={!!lesson || saving}
          ><option value="">Klasse auswählen</option
          >{#each classrooms as classroom}<option value={classroom.id}
              >{classroom.name} · {classroom.school_year}</option
            >{/each}</select
        ></label
      >
      <label
        >Fach<select
          bind:value={subjectId}
          required
          disabled={!!lesson || saving || subjectLoading}
          ><option value=""
            >{subjectLoading
              ? "Fächer werden geladen …"
              : "Fach auswählen"}</option
          >{#each subjects as subject}<option value={subject.id}
              >{subject.name}</option
            >{/each}</select
        ></label
      >
    </div>
    {#if !lesson}<div class="form-grid">
        <button
          type="button"
          class="text-button"
          onclick={() => (newClass = !newClass)}
          disabled={saving}><Icon name="plus" /> Klasse schnell anlegen</button
        ><button
          type="button"
          class="text-button"
          onclick={() => (newSubject = !newSubject)}
          disabled={saving || !classId}
          ><Icon name="plus" /> Fach schnell anlegen</button
        >
      </div>{/if}
    {#if newClass}<div class="inline-create">
        <div class="form-grid">
          <label
            >Klassenname<input
              bind:value={className}
              maxlength="80"
              placeholder="2a"
            /></label
          ><label
            >Schuljahr<input bind:value={schoolYear} maxlength="40" /></label
          >
        </div>
        <button
          type="button"
          class="secondary"
          onclick={addClass}
          disabled={saving || !className.trim() || !schoolYear.trim()}
          >Klasse anlegen & auswählen</button
        >
      </div>{/if}
    {#if newSubject}<div class="inline-create">
        <label
          >Neues Fach<input
            bind:value={subjectName}
            maxlength="80"
            placeholder="Mathematik"
          /></label
        ><button
          type="button"
          class="secondary"
          onclick={addSubject}
          disabled={saving || !subjectName.trim()}
          >Fach anlegen & auswählen</button
        >
      </div>{/if}
    <div class="form-grid">
      <label>Datum<input type="date" bind:value={date} required /></label><label
        >Raum <span class="muted">Optional</span><input
          bind:value={room}
          maxlength="80"
          placeholder="1.12"
        /></label
      >
    </div>
    <div class="form-grid">
      <label>Beginn<input type="time" bind:value={start} required /></label
      ><label>Ende<input type="time" bind:value={end} required /></label>
    </div>
    {#if !lesson}<label class="pupil-check"
        ><input type="checkbox" bind:checked={repeat} /><span
          >Wöchentlich wiederholen</span
        ></label
      >{#if repeat}<label
          >Wiederholen bis<input
            type="date"
            bind:value={repeatUntil}
            min={date}
            max={`${Number(date.slice(0, 4)) + 1}${date.slice(4)}`}
            required
          /></label
        >
        <p class="muted">
          Jede Woche am gleichen Wochentag. Bestehende Stunden werden auf
          Überschneidungen geprüft.
        </p>{/if}{/if}
    <label
      >Thema der Stunde <span class="muted">Optional</span><input
        bind:value={topic}
        maxlength="500"
        placeholder="Was steht heute im Mittelpunkt?"
      /></label
    >
    <label
      >Lernziele <span class="muted">Optional · ein Ziel pro Zeile</span
      ><textarea bind:value={goals} rows="3" maxlength="5000"></textarea></label
    >
    {#if lesson}<label
        >Status<select bind:value={status}
          ><option value="scheduled">Findet statt</option><option
            value="cancelled">Fällt aus</option
          ></select
        ></label
      >{/if}
    <div class="modal-actions">
      <button type="button" class="secondary" onclick={close} disabled={saving}
        >Abbrechen</button
      ><button
        class="primary"
        disabled={saving || !classId || !subjectId || newClass || newSubject}
        >{saving
          ? "Speichert …"
          : lesson
            ? "Änderungen speichern"
            : "Stunde hinzufügen"}</button
      >
    </div>
  </form>
</dialog>
