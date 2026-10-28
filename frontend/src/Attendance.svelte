<script>
  import { onDestroy } from "svelte";
  import { api } from "./api.js";

  let {
    classId,
    className,
    students,
    saving = $bindable(false),
    disabled = false,
    onerror,
  } = $props();
  let mode = $state("roll"),
    date = $state(today()),
    month = $state(today().slice(0, 7));
  let records = $state({}),
    loading = $state(true),
    loadedDate = $state("");
  let error = $state(""),
    notice = $state(""),
    index = $state(0);
  let version = 0;
  const locked = $derived(disabled || saving || loading || loadedDate !== date);
  const present = $derived(
    students.filter((s) => records[s.id] === "present").length,
  );
  const absent = $derived(
    students.filter((s) => records[s.id] === "absent").length,
  );
  const open = $derived(students.length - present - absent);
  const pupil = $derived(students[index]);
  const dayLabel = $derived(
    new Date(date + "T12:00:00").toLocaleDateString("de-DE", {
      weekday: "long",
      day: "numeric",
      month: "long",
      year: "numeric",
    }),
  );
  const monthLabel = $derived(
    new Date(month + "-15T12:00:00").toLocaleDateString("de-DE", {
      month: "long",
      year: "numeric",
    }),
  );
  const days = $derived.by(() => {
    const first = new Date(month + "-01T12:00:00");
    const count = new Date(
      first.getFullYear(),
      first.getMonth() + 1,
      0,
    ).getDate();
    return [
      ...Array((first.getDay() + 6) % 7).fill(null),
      ...Array.from(
        { length: count },
        (_, i) => `${month}-${String(i + 1).padStart(2, "0")}`,
      ),
    ];
  });

  function dateKey(d) {
    return `${String(d.getFullYear()).padStart(4, "0")}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  }
  function today() {
    return dateKey(new Date());
  }
  function firstOpen(values = records) {
    const next = students.findIndex((s) => !values[s.id]);
    return next === -1 ? students.length : next;
  }
  function selectDate(value) {
    if (
      saving ||
      disabled ||
      !/^\d{4}-\d{2}-\d{2}$/.test(value) ||
      value < "0001-01-01" ||
      value > "9999-12-31"
    )
      return;
    date = value;
    month = value.slice(0, 7);
  }
  function moveMonth(offset) {
    const d = new Date(month + "-15T12:00:00");
    d.setMonth(d.getMonth() + offset);
    month = dateKey(d).slice(0, 7);
  }
  function chooseMode(value) {
    mode = value;
    if (value === "roll") index = firstOpen();
  }
  $effect(() => {
    loadDay(classId, date);
  });
  onDestroy(() => {
    version += 1;
  });

  async function loadDay(classroom, day) {
    const request = ++version;
    loading = true;
    loadedDate = "";
    error = "";
    notice = "";
    try {
      const result = await api(`/classrooms/${classroom}/attendance/${day}`);
      if (request !== version) return;
      const values = Object.fromEntries(
        result.map((r) => [r.student_id, r.status]),
      );
      records = values;
      index = firstOpen(values);
      loadedDate = day;
    } catch (e) {
      if (request !== version) return;
      error = e.message;
      if (e.status === 401) onerror(e);
    } finally {
      if (request === version) loading = false;
    }
  }
  async function save(student, status, advance = false) {
    if (locked) return;
    saving = true;
    error = "";
    notice = "";
    try {
      await api(`/classrooms/${classId}/attendance/${date}`, {
        method: "PATCH",
        body: JSON.stringify({ changes: [{ student_id: student, status }] }),
      });
      records = { ...records, [student]: status };
      notice = "Alle Änderungen gespeichert";
      if (advance) index += 1;
    } catch (e) {
      error = e.message;
      if (e.status === 401) onerror(e);
    } finally {
      saving = false;
    }
  }
</script>

<section
  class="workspace-card attendance"
  aria-label="Anwesenheit"
  aria-busy={loading}
>
  <div class="workspace-header">
    <div class="workspace-title">
      <span class="book-icon" aria-hidden="true">✓</span>
      <div>
        <h2>Anwesenheit · {className}</h2>
        <p>Ein Eintrag pro Kind und Tag</p>
      </div>
    </div>
    <div class="attendance-date-controls">
      <label
        >Datum<input
          type="date"
          value={date}
          min="0001-01-01"
          max="9999-12-31"
          disabled={saving || disabled}
          onchange={(e) => {
            selectDate(e.currentTarget.value);
            e.currentTarget.value = date;
          }}
        /></label
      >
      <button
        class="secondary"
        disabled={saving || disabled}
        onclick={() => selectDate(today())}>Heute</button
      >
    </div>
  </div>
  <div class="workspace-subheader">
    <div class="view-tabs" role="tablist" aria-label="Anwesenheitsansicht">
      <button
        role="tab"
        aria-selected={mode === "roll"}
        class:active={mode === "roll"}
        disabled={saving || disabled}
        onclick={() => chooseMode("roll")}>◉ Kind für Kind</button
      >
      <button
        role="tab"
        aria-selected={mode === "calendar"}
        class:active={mode === "calendar"}
        disabled={saving || disabled}
        onclick={() => chooseMode("calendar")}>▦ Kalender & Liste</button
      >
    </div>
    <div class="save-status" role="status">
      {saving
        ? "Wird gespeichert …"
        : loading
          ? "Wird geladen …"
          : notice || "Änderungen werden automatisch gespeichert"}
    </div>
  </div>
  {#if error}<div class="alert attendance-error" role="alert">
      {error}
      {#if loadedDate !== date}<button
          class="secondary"
          disabled={loading || disabled}
          onclick={() => loadDay(classId, date)}>Erneut laden</button
        >{/if}
    </div>{/if}
  <div class="attendance-layout" class:with-calendar={mode === "calendar"}>
    {#if mode === "calendar"}
      <aside class="attendance-calendar" aria-label="Kalender">
        <div class="calendar-heading">
          <button
            class="icon-button"
            aria-label="Vorheriger Monat"
            disabled={saving || disabled || month === "0001-01"}
            onclick={() => moveMonth(-1)}>‹</button
          >
          <strong aria-live="polite">{monthLabel}</strong>
          <button
            class="icon-button"
            aria-label="Nächster Monat"
            disabled={saving || disabled || month === "9999-12"}
            onclick={() => moveMonth(1)}>›</button
          >
        </div>
        <div class="calendar-grid">
          {#each ["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"] as weekday}<span
              class="weekday">{weekday}</span
            >{/each}
          {#each days as day}
            {#if day}<button
                class:chosen={day === date}
                class:is-today={day === today()}
                aria-label={new Date(day + "T12:00:00").toLocaleDateString(
                  "de-DE",
                  { day: "numeric", month: "long", year: "numeric" },
                )}
                aria-pressed={day === date}
                disabled={saving || disabled}
                onclick={() => selectDate(day)}>{Number(day.slice(8))}</button
              >
            {:else}<span></span>{/if}
          {/each}
        </div>
        <p class="muted">
          Wähle einen Tag, um Anwesenheiten nachzutragen oder zu korrigieren.
        </p>
      </aside>
    {/if}
    <div class="attendance-content">
      <div class="attendance-day">
        <h3>{dayLabel}</h3>
        {#if loadedDate === date}<p
            class="attendance-counts"
            aria-live="polite"
          >
            <span>{present} anwesend</span><span>{absent} abwesend</span><span
              >{open} offen</span
            >
          </p>{/if}
      </div>
      {#if loading}<div class="empty-panel">
          <p>Anwesenheit wird geladen …</p>
        </div>
      {:else if loadedDate !== date}<div class="empty-panel">
          <p>Für diesen Tag konnten die Einträge nicht geladen werden.</p>
        </div>
      {:else if mode === "roll"}
        {#if pupil}
          <div class="roll-call" aria-live="polite">
            <span class="eyebrow">KIND {index + 1} VON {students.length}</span>
            <progress
              value={present + absent}
              max={students.length}
              aria-label="Erfasste Kinder"
            ></progress>
            <span class="avatar big"
              >{pupil.name
                .split(/\s+/)
                .map((p) => p[0])
                .slice(0, 2)
                .join("")}</span
            >
            <h3>{pupil.name}</h3>
            <p class="muted">
              {records[pupil.id] === "present"
                ? "Als anwesend erfasst"
                : records[pupil.id] === "absent"
                  ? "Als abwesend erfasst"
                  : "Noch nicht erfasst"}
            </p>
            <div class="roll-actions">
              <button
                class="primary"
                disabled={locked}
                onclick={() => save(pupil.id, "present", true)}
                >✓ Anwesend</button
              >
              <button
                class="primary absent-button"
                disabled={locked}
                onclick={() => save(pupil.id, "absent", true)}
                >✕ Abwesend</button
              >
            </div>
            <div class="roll-navigation">
              <button
                class="secondary"
                disabled={locked || index === 0}
                onclick={() => (index -= 1)}>‹ Zurück</button
              >
              <button
                class="secondary"
                disabled={locked}
                onclick={() => (index += 1)}
                >{records[pupil.id] ? "Weiter ›" : "Überspringen ›"}</button
              >
            </div>
          </div>
        {:else}
          <div class="empty-panel roll-complete" role="status">
            <span class="empty-icon">{open ? "◷" : "✓"}</span>
            <h3>
              {open ? "Noch nicht ganz fertig." : "Alle Kinder sind erfasst."}
            </h3>
            <p>{present} anwesend · {absent} abwesend · {open} offen</p>
            {#if open}<button
                class="primary"
                disabled={locked}
                onclick={() => (index = firstOpen())}
                >Offene Kinder erfassen</button
              >{/if}
            <button
              class="secondary"
              disabled={locked}
              onclick={() => (index = 0)}>Klasse erneut durchgehen</button
            >
            <button
              class="text-button"
              disabled={locked}
              onclick={() => chooseMode("calendar")}
              >Einträge in der Liste prüfen</button
            >
          </div>
        {/if}
      {:else}
        <p class="muted attendance-help">
          Setze ein Häkchen für anwesend oder abwesend. Entferne es, um den
          Eintrag zurückzusetzen.
        </p>
        <div class="attendance-table-scroll">
          <table class="attendance-table">
            <thead
              ><tr
                ><th scope="col">Kind</th><th scope="col">Anwesend</th><th
                  scope="col">Abwesend</th
                ><th scope="col">Status</th></tr
              ></thead
            >
            <tbody
              >{#each students as student, i (student.id)}
                <tr
                  ><th scope="row"
                    ><span class="roster-number">{i + 1}.</span>
                    {student.name}</th
                  >
                  <td
                    ><input
                      type="checkbox"
                      aria-label={`${student.name} (${i + 1}): Anwesend`}
                      checked={records[student.id] === "present"}
                      disabled={locked}
                      onchange={(e) => {
                        e.currentTarget.checked =
                          records[student.id] === "present";
                        save(
                          student.id,
                          records[student.id] === "present" ? null : "present",
                        );
                      }}
                    /></td
                  >
                  <td
                    ><input
                      type="checkbox"
                      aria-label={`${student.name} (${i + 1}): Abwesend`}
                      checked={records[student.id] === "absent"}
                      disabled={locked}
                      onchange={(e) => {
                        e.currentTarget.checked =
                          records[student.id] === "absent";
                        save(
                          student.id,
                          records[student.id] === "absent" ? null : "absent",
                        );
                      }}
                    /></td
                  >
                  <td class="attendance-row-status"
                    >{records[student.id] === "present"
                      ? "Anwesend"
                      : records[student.id] === "absent"
                        ? "Abwesend"
                        : "Offen"}</td
                  >
                </tr>
              {/each}</tbody
            >
          </table>
        </div>
      {/if}
    </div>
  </div>
</section>
