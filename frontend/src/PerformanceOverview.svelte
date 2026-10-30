<script>
  import { untrack } from "svelte";
  import { api } from "./api.js";
  import Icon from "./Icon.svelte";
  import { dateLabel, initials, subjectStyle } from "./teaching.js";
  import {
    performanceLessons,
    pupilPerformances,
  } from "./performance-history.js";
  let {
    classId,
    className,
    lessons,
    loading,
    lessonId = "",
    saving = false,
    onselect,
    oncapture,
    onerror,
  } = $props();
  let subject = $state(""),
    from = $state(""),
    until = $state(""),
    recordedOnly = $state(true);
  let search = $state(""),
    pupilsOnlyRecorded = $state(false),
    students = $state([]),
    records = $state([]),
    recordsLoading = $state(false),
    error = $state("");
  let selectedId = $state(untrack(() => lessonId));
  const subjects = $derived(
    [
      ...new Map(
        lessons.map((l) => [
          l.subject_id,
          { id: l.subject_id, name: l.subject_name },
        ]),
      ).values(),
    ].sort((a, b) => a.name.localeCompare(b.name, "de")),
  );
  const visibleLessons = $derived(
    performanceLessons(lessons, { subject, from, until, recordedOnly }),
  );
  const selected = $derived(
    visibleLessons.find((l) => l.id === selectedId) ||
      visibleLessons[0] ||
      null,
  );
  const pupils = $derived(
    pupilPerformances(students, records, search, pupilsOnlyRecorded),
  );
  const savedCount = $derived(records.length);
  $effect(() => {
    selectedId = lessonId;
  });
  $effect(() => {
    const lesson = selected;
    let active = true;
    records = [];
    students = [];
    error = "";
    recordsLoading = !!lesson;
    if (lesson)
      Promise.all([
        api(`/classrooms/${lesson.classroom_id}/workspace`),
        api(`/lessons/${lesson.id}/performances`),
      ])
        .then(([workspace, list]) => {
          if (active) {
            students = workspace.students;
            records = list;
          }
        })
        .catch((e) => {
          if (active) {
            error = e.message;
            if (e.status === 401) onerror(e);
          }
        })
        .finally(() => {
          if (active) recordsLoading = false;
        });
    return () => (active = false);
  });
  $effect(() => {
    const id = classId;
    untrack(() => {
      subject = "";
      from = "";
      until = "";
      search = "";
      recordedOnly = true;
      pupilsOnlyRecorded = false;
    });
  });
  function select(lesson) {
    if (saving) return;
    selectedId = lesson.id;
    search = "";
    onselect(lesson);
  }
  function startCapture() {
    const lesson =
      (selected?.status === "scheduled" ? selected : null) ||
      performanceLessons(lessons, { recordedOnly: false }).find(
        (l) => l.status === "scheduled",
      );
    recordedOnly = false;
    subject = "";
    from = "";
    until = "";
    if (lesson) {
      select(lesson);
      oncapture(lesson);
    }
  }
</script>

<section class="page-heading with-actions">
  <div>
    <h1>Unterrichtsleistungen</h1>
    <p>Gespeicherte Bewertungen und Beobachtungen · Klasse {className}</p>
  </div>
  <button
    class="primary"
    onclick={startCapture}
    disabled={saving ||
      loading ||
      !lessons.some((l) => l.status === "scheduled")}
    ><Icon name="plus" /> Leistungen erfassen</button
  >
</section>
<section
  class="workspace-card performance-history-filters"
  aria-label="Leistungsverlauf filtern"
>
  <label
    >Fach<select bind:value={subject}
      ><option value="">Alle Fächer</option>{#each subjects as item}<option
          value={item.id}>{item.name}</option
        >{/each}</select
    ></label
  >
  <label>Von<input type="date" bind:value={from} /></label><label
    >Bis<input type="date" bind:value={until} min={from || undefined} /></label
  >
  <label class="pupil-check"
    ><input type="checkbox" bind:checked={recordedOnly} /><span
      >Nur Stunden mit Leistungen</span
    ></label
  >
</section>
{#if from && until && from > until}<p class="alert" role="alert">
    Das Enddatum muss am oder nach dem Startdatum liegen.
  </p>{/if}
<div class="performance-history-layout">
  <section
    class="workspace-card performance-lesson-list"
    aria-label="Gespeicherte Unterrichtsleistungen"
    aria-busy={loading}
  >
    <div class="panel-heading">
      <h2>{recordedOnly ? "Erfasste Stunden" : "Alle Stunden"}</h2>
      <span class="muted">{visibleLessons.length}</span>
    </div>
    {#if loading}<p class="muted" role="status">Leistungen werden geladen …</p>
    {:else if !visibleLessons.length}<div class="empty-panel">
        <Icon name="chart" size={30} />
        <h3>
          {recordedOnly
            ? "Noch keine passenden Leistungen"
            : "Keine passenden Stunden"}
        </h3>
        <p>
          {recordedOnly
            ? "Hier erscheinen gespeicherte Bewertungen, auch aus früheren Unterrichtsstunden."
            : "Ändere den Zeitraum oder das Fach, um weitere Stunden zu finden."}
        </p>
        {#if recordedOnly}<button
            class="secondary"
            onclick={() => (recordedOnly = false)}>Alle Stunden anzeigen</button
          >{/if}
      </div>
    {:else}{#each visibleLessons as lesson (lesson.id)}<button
          class="performance-lesson-row"
          class:active={lesson.id === selected?.id}
          aria-pressed={lesson.id === selected?.id}
          onclick={() => select(lesson)}
          disabled={saving}
          ><span class={`subject-icon ${subjectStyle(lesson.subject_name)}`}
            ><Icon name="chart" /></span
          ><span
            ><strong>{lesson.subject_name}</strong><small
              >{dateLabel(lesson.lesson_date, {
                weekday: "short",
                day: "numeric",
                month: "short",
                year: "numeric",
              })} · {lesson.starts_at}–{lesson.ends_at}</small
            ><small
              >{lesson.performance_count} von {lesson.pupil_count} erfasst{lesson.status ===
              "cancelled"
                ? " · Fällt aus"
                : ""}</small
            ></span
          ><Icon name="chevron" /></button
        >{/each}{/if}
  </section>
  <section
    class="workspace-card performance-results"
    aria-label="Bewertungen der ausgewählten Stunde"
    aria-busy={recordsLoading}
  >
    {#if selected}<div class="panel-heading">
        <div>
          <h2>{selected.subject_name} · Klasse {selected.class_name}</h2>
          <p class="muted">
            {dateLabel(selected.lesson_date, {
              weekday: "long",
              day: "numeric",
              month: "long",
              year: "numeric",
            })} · {selected.starts_at}–{selected.ends_at}
          </p>
        </div>
        <button
          class="secondary"
          onclick={() => oncapture(selected)}
          disabled={saving || selected.status === "cancelled"}
          ><Icon name="edit" /> Erfassen / bearbeiten</button
        >
      </div>
      {#if selected.topic}<p class="performance-lesson-topic">
          {selected.topic}
        </p>{/if}
      {#if selected.status === "cancelled"}<p class="info-box">
          Diese Stunde fällt aus. Gespeicherte Leistungen bleiben sichtbar.
        </p>{/if}
      {#if error}<div class="alert" role="alert">{error}</div>
      {:else if recordsLoading}<p class="muted" role="status">
          Bewertungen werden geladen …
        </p>
      {:else}<p class="performance-results-count">
          {savedCount} von {students.length} Kindern bewertet{#if students.length > savedCount}
            · {students.length - savedCount} noch offen{/if}
        </p>
        <div class="performance-pupil-filters">
          <label
            >Kind suchen<input
              type="search"
              bind:value={search}
              placeholder="Name …"
            /></label
          ><label class="pupil-check"
            ><input type="checkbox" bind:checked={pupilsOnlyRecorded} /><span
              >Nur bewertete Kinder</span
            ></label
          >
        </div>
        {#if !pupils.length}<div class="empty-panel">
            <h3>Keine passenden Kinder</h3>
            <p>Ändere den Suchtext oder den Filter.</p>
          </div>
        {:else}<div class="performance-results-scroll">
            <table class="performance-results-table">
              <thead
                ><tr
                  ><th scope="col">Kind</th><th scope="col">Bewertung</th><th
                    scope="col">Beobachtung</th
                  ><th scope="col">Aktion</th></tr
                ></thead
              ><tbody
                >{#each pupils as pupil (pupil.id)}<tr
                    ><th scope="row"
                      ><span class="performance-pupil-name"
                        ><span class="note-avatar">{initials(pupil.name)}</span
                        ><span>{pupil.name}</span></span
                      ></th
                    ><td
                      >{#if pupil.performance}<span
                          class="saved-rating"
                          aria-label={`${pupil.performance.rating} von 4`}
                          ><Icon name="star" /><strong
                            >{pupil.performance.rating}<small>
                              / 4</small
                            ></strong
                          ></span
                        >{:else}<span class="muted">Noch offen</span>{/if}</td
                    ><td class="performance-observation"
                      >{pupil.performance?.observation || "–"}</td
                    ><td
                      ><button
                        class="text-button"
                        onclick={() => oncapture(selected, pupil.id)}
                        disabled={saving || selected.status === "cancelled"}
                        aria-label={`${pupil.name}: ${pupil.performance ? "Bewertung bearbeiten" : "Leistung erfassen"}`}
                        >{pupil.performance ? "Bearbeiten" : "Erfassen"}</button
                      ></td
                    ></tr
                  >{/each}</tbody
              >
            </table>
          </div>{/if}
      {/if}
    {:else}<div class="empty-panel">
        <Icon name="chart" size={38} />
        <h2>Leistungen im Überblick</h2>
        <p>
          Wähle eine erfasste Stunde aus. Hier findest du die Bewertungen und
          optionalen Beobachtungen für jedes Kind.
        </p>
      </div>{/if}
  </section>
</div>

<style>
  .performance-history-filters {
    display: flex;
    flex-wrap: wrap;
    align-items: end;
    gap: 16px;
    margin-bottom: 20px;
  }
  .performance-history-filters > label:not(.pupil-check) {
    flex: 1 1 150px;
  }
  .performance-history-filters > .pupil-check {
    padding-bottom: 12px;
    flex: 1 1 190px;
  }
  .performance-history-layout {
    display: grid;
    grid-template-columns: minmax(240px, 0.8fr) minmax(0, 1.8fr);
    gap: 18px;
    align-items: start;
  }
  .performance-lesson-list {
    padding: 17px 12px;
  }
  .performance-lesson-list > .panel-heading {
    padding: 0 6px;
  }
  .performance-lesson-row {
    display: flex;
    align-items: center;
    gap: 11px;
    text-align: left;
    width: 100%;
    padding: 13px 9px;
    color: var(--ink);
    background: transparent;
    border: 1px solid transparent;
    border-radius: 8px;
  }
  .performance-lesson-row + .performance-lesson-row {
    margin-top: 4px;
  }
  .performance-lesson-row.active {
    background: #edf6f0;
    border-color: #c4dfce;
  }
  .performance-lesson-row:hover {
    background: #f0f6fa;
  }
  .performance-lesson-row > span:nth-child(2) {
    flex: 1;
    min-width: 0;
  }
  .performance-lesson-row strong,
  .performance-lesson-row small {
    display: block;
  }
  .performance-lesson-row strong {
    font-size: 14px;
    font-weight: 650;
  }
  .performance-lesson-row small {
    font-size: 11px;
    color: var(--muted);
    margin-top: 3px;
  }
  .performance-results > .panel-heading {
    align-items: flex-start;
    flex-wrap: wrap;
  }
  .performance-results .panel-heading p {
    font-size: 12px;
    margin-top: 7px;
  }
  .performance-results .panel-heading > .secondary {
    font-size: 12px;
    padding: 8px 12px;
  }
  .performance-lesson-topic {
    font-size: 14px;
    padding-bottom: 14px;
    border-bottom: 1px solid var(--line);
  }
  .performance-results-count {
    font-size: 13px;
    color: #366b50;
    margin: 17px 0;
  }
  .performance-pupil-filters {
    display: flex;
    align-items: end;
    gap: 16px;
    flex-wrap: wrap;
    margin-bottom: 18px;
  }
  .performance-pupil-filters > label:first-child {
    flex: 1 1 160px;
  }
  .performance-pupil-filters > .pupil-check {
    padding-bottom: 11px;
  }
  .performance-results-scroll {
    overflow-x: auto;
  }
  .performance-results-table {
    border-collapse: collapse;
    width: 100%;
    text-align: left;
    min-width: 470px;
  }
  .performance-results-table th,
  .performance-results-table td {
    padding: 14px 9px;
    border-bottom: 1px solid var(--line);
    vertical-align: top;
    font-size: 13px;
  }
  .performance-results-table thead th {
    font-size: 12px;
    color: var(--muted);
    font-weight: 550;
  }
  .performance-results-table tbody th {
    font-weight: 550;
  }
  .performance-pupil-name {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 150px;
  }
  .performance-pupil-name .note-avatar {
    width: 31px;
    height: 31px;
    font-size: 11px;
  }
  .saved-rating {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    white-space: nowrap;
  }
  .saved-rating :global(.ui-icon) {
    color: #e8a319;
    fill: #e8a319;
  }
  .saved-rating small {
    color: var(--muted);
    font-weight: 400;
  }
  .performance-observation {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    min-width: 120px;
  }
  @media (max-width: 1150px) {
    .performance-history-layout {
      grid-template-columns: 1fr;
    }
    .performance-lesson-list {
      max-height: 340px;
      overflow-y: auto;
    }
  }
  @media (max-width: 700px) {
    .performance-history-filters {
      gap: 12px;
    }
    .performance-history-filters > label:not(.pupil-check) {
      flex-basis: 120px;
    }
    .performance-history-filters > .pupil-check {
      padding: 0;
    }
    .performance-results {
      padding: 14px;
    }
    .performance-pupil-filters {
      gap: 10px;
    }
  }
</style>
