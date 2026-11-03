<script>
  import DevelopmentChart from "./DevelopmentChart.svelte";
  import { dateLabel } from "./teaching.js";
  import {
    filterPupilHistory,
    classLessonAverages,
    classPupilSummaries,
  } from "./pupil-history.js";
  let { students, data, today, onselect } = $props();
  let subject = $state(""),
    from = $state(""),
    until = $state("");
  const subjects = $derived(
    [
      ...new Map(
        [...data.performances, ...data.homework].map((r) => [
          r.subject_id,
          { id: r.subject_id, name: r.subject_name },
        ]),
      ).values(),
    ].sort((a, b) => a.name.localeCompare(b.name, "de")),
  );
  const performances = $derived(
    filterPupilHistory(data.performances, { subject, from, until }),
  );
  const homework = $derived(
    filterPupilHistory(data.homework, { subject, from, until }),
  );
  const lessons = $derived(classLessonAverages(performances));
  const pupils = $derived(
    classPupilSummaries(students, performances, homework, today),
  );
  const format = (value) =>
    value.toLocaleString("de-DE", { maximumFractionDigits: 2 });
</script>

<section
  class="workspace-card class-development"
  aria-label="Klassenentwicklung"
>
  <div class="panel-heading">
    <div>
      <h2>Klassenentwicklung</h2>
      <p class="muted">
        Unterrichtsleistungen und Hausaufgaben der ganzen Klasse im Überblick.
      </p>
    </div>
  </div>
  <div class="filters">
    <label
      >Fach<select bind:value={subject}
        ><option value="">Alle Fächer</option>{#each subjects as item}<option
            value={item.id}>{item.name}</option
          >{/each}</select
      ></label
    ><label>Von<input type="date" bind:value={from} /></label><label
      >Bis<input
        type="date"
        bind:value={until}
        min={from || undefined}
      /></label
    >
  </div>
  {#if from && until && from > until}<p class="alert" role="alert">
      Das Enddatum muss am oder nach dem Startdatum liegen.
    </p>
  {:else}
    <p class="muted">
      Jeder Punkt zeigt den Durchschnitt der erfassten Bewertungen einer Stunde
      (1–4), getrennt nach Fach. Nicht bewertete Kinder zählen nicht mit. Die
      Zahl der bewerteten Kinder steht in der Tabelle; wechselnde Gruppen können
      den Durchschnitt beeinflussen.
    </p>
    {#if lessons.length}
      <DevelopmentChart
        records={lessons}
        label="Klassenentwicklung: durchschnittliche Unterrichtsbewertungen von 1 bis 4, getrennt nach Fach"
      />
      <details>
        <summary>Durchschnitte je Unterrichtsstunde ({lessons.length})</summary>
        <div class="table-scroll">
          <table>
            <thead
              ><tr
                ><th>Datum</th><th>Fach</th><th>Durchschnitt</th><th
                  >Bewertete Kinder</th
                ></tr
              ></thead
            ><tbody
              >{#each lessons.toSorted( (a, b) => (b.date + b.time).localeCompare(a.date + a.time) ) as lesson}<tr
                  ><td>{dateLabel(lesson.date)} · {lesson.time}</td><td
                    >{lesson.subject_name}</td
                  ><td>{format(lesson.rating)} / 4</td><td>{lesson.count}</td
                  ></tr
                >{/each}</tbody
            >
          </table>
        </div>
      </details>
    {:else}<p class="empty-panel">
        Noch keine Unterrichtsbewertungen für diese Auswahl.
      </p>{/if}
    <h3>Kinder im Überblick</h3>
    <p class="muted">
      Alle Werte folgen dem Fach- und Datumsfilter. Hausaufgaben zählen nach
      Fälligkeit: Abgaben nur für Termine vor heute, fehlende Abgaben nur aus
      den letzten 14 Tagen. Der Bewertungsdurchschnitt bezieht sich auf alle
      ausgewählten Bewertungen des Kindes.
    </p>
    {#if pupils.length}<div class="table-scroll">
        <table>
          <thead
            ><tr
              ><th>Kind</th><th>Bewertungen</th><th>Durchschnitt</th><th
                >Zuletzt bewertet</th
              ><th>Hausaufgaben abgegeben</th><th>Zuletzt nicht abgegeben</th
              ></tr
            ></thead
          ><tbody
            >{#each pupils as pupil (pupil.id)}<tr
                ><th scope="row"
                  ><button
                    class="text-button"
                    onclick={() => onselect(pupil.id)}
                    aria-label={`${pupil.name}: Verlauf öffnen`}
                    >{pupil.name}</button
                  ></th
                ><td>{pupil.count}</td><td
                  >{pupil.average == null
                    ? "–"
                    : `${format(pupil.average)} / 4`}</td
                ><td>{pupil.latest ? dateLabel(pupil.latest) : "–"}</td><td
                  >{pupil.due ? `${pupil.submitted} / ${pupil.due}` : "–"}</td
                ><td
                  ><span class:missing={pupil.missing > 0}
                    >{pupil.missing || "–"}</span
                  ></td
                ></tr
              >{/each}</tbody
          >
        </table>
      </div>{:else}<p class="empty-panel">
        Noch keine Kinder in dieser Klasse.
      </p>{/if}
  {/if}
</section>

<style>
  .class-development {
    margin-bottom: 20px;
  }
  .filters {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    margin: 20px 0;
  }
  .filters label {
    flex: 1 1 150px;
  }
  .muted {
    font-size: 13px;
    line-height: 1.6;
  }
  h3 {
    margin-top: 24px;
  }
  .table-scroll {
    overflow-x: auto;
  }
  table {
    width: 100%;
    min-width: 540px;
    border-collapse: collapse;
    text-align: left;
    font-size: 13px;
  }
  td,
  th {
    padding: 14px 10px;
    border-bottom: 1px solid var(--line);
    vertical-align: top;
  }
  .missing {
    color: #92571a;
    font-weight: 650;
  }
  summary {
    cursor: pointer;
    padding: 12px 0;
  }
</style>
