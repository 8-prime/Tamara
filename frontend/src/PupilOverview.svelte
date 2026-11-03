<script lang="ts">
  import DevelopmentChart from "./DevelopmentChart.svelte";
  import ClassDevelopment from "./ClassDevelopment.svelte";
  import { api } from "./api.ts";
  import { dateLabel, initials } from "./teaching.ts";
  import { recentMissing, performancePoints } from "./pupil-history.ts";
  let {
    classId,
    students,
    pupilId = "",
    onselect,
    onrename,
    ondelete,
    owner,
    disabled,
    onerror,
    search = $bindable(""),
  } = $props();
  let data = $state({ performances: [], homework: [] });
  let loading = $state(true),
    error = $state(""),
    subject = $state("");
  const today = new Date().toLocaleDateString("sv-SE");
  const pupil = $derived(students.find((s) => s.id === pupilId));
  const subjects = $derived(
    [
      ...new Map(
        [...data.performances, ...data.homework]
          .filter((r) => r.student_id === pupilId)
          .map((r) => [
            r.subject_id,
            { id: r.subject_id, name: r.subject_name },
          ]),
      ).values(),
    ].sort((a, b) => a.name.localeCompare(b.name, "de")),
  );
  const performances = $derived(
    data.performances.filter(
      (r) => r.student_id === pupilId && (!subject || r.subject_id === subject),
    ),
  );
  const homework = $derived(
    data.homework.filter(
      (r) => r.student_id === pupilId && (!subject || r.subject_id === subject),
    ),
  );
  const missing = $derived(recentMissing(homework, today));
  const points = $derived(performancePoints(performances));
  const qualityLabels = {
    all_correct: "Alles richtig",
    partially_correct: "Teilweise korrekt",
    missing: "Fehlt",
    all_incorrect: "Alles falsch",
  };
  $effect(() => {
    const id = classId;
    let active = true;
    loading = true;
    error = "";
    api(`/classrooms/${id}/pupil-history`)
      .then((result) => {
        if (active) data = result;
      })
      .catch((e) => {
        if (active) {
          error = e.message;
          onerror(e);
        }
      })
      .finally(() => {
        if (active) loading = false;
      });
    return () => {
      active = false;
    };
  });
  $effect(() => {
    const id = pupilId;
    subject = "";
  });
</script>

{#if error}<p class="alert" role="alert">{error}</p>{/if}
{#if pupilId}
  <section class="workspace-card pupil-detail" aria-busy={loading}>
    <button class="text-button" onclick={() => onselect("")}
      >← Alle Kinder</button
    >
    {#if !pupil}<p class="muted">Dieses Kind ist nicht mehr in der Klasse.</p>
    {:else}
      <div class="detail-heading">
        <span class="avatar small-avatar">{initials(pupil.name)}</span>
        <h2>{pupil.name}</h2>
      </div>
      {#if loading}<p role="status">Verlauf wird geladen …</p>
      {:else if !error}
        <label class="subject-filter"
          >Fach<select bind:value={subject}
            ><option value="">Alle Fächer</option
            >{#each subjects as item}<option value={item.id}>{item.name}</option
              >{/each}</select
          ></label
        >
        <div class="summary">
          <span
            ><strong>{performances.length}</strong> Unterrichtsbewertungen</span
          ><span
            ><strong
              >{homework.filter((h) => h.handed_in_at != null).length} / {homework.length}</strong
            > Hausaufgaben abgegeben</span
          >
        </div>
        {#if missing.length}<p class="homework-warning" role="status">
            {missing.length} Hausaufgabe{missing.length === 1 ? "" : "n"} zuletzt
            nicht abgegeben
          </p>{/if}
        <p class="muted">
          Hinweis: Gezählt werden zugewiesene, noch nicht abgegebene
          Hausaufgaben mit Fälligkeit in den letzten 14 Tagen vor heute.
        </p>
        <h3>Leistungsentwicklung</h3>
        <p class="muted">
          Bewertung von 1–4 je Unterrichtsstunde. Jedes Fach hat eine eigene
          Linie.
        </p>
        {#if points.length}
          <DevelopmentChart
            records={performances}
            label={`Leistungsentwicklung für ${pupil.name}, Bewertungen von 1 bis 4`}
          />
          <div class="history-scroll">
            <table>
              <caption>Bewertungen und Beobachtungen</caption><thead
                ><tr
                  ><th>Datum</th><th>Fach</th><th>Bewertung</th><th
                    >Beobachtung</th
                  ></tr
                ></thead
              ><tbody
                >{#each performances.toReversed() as record}<tr
                    ><td>{dateLabel(record.date)} · {record.time}</td><td
                      >{record.subject_name}</td
                    ><td>{record.rating} / 4</td><td class="observation"
                      >{record.observation || "–"}</td
                    ></tr
                  >{/each}</tbody
              >
            </table>
          </div>
        {:else}<p class="empty-panel">
            Noch keine Unterrichtsbewertungen für diese Auswahl.
          </p>{/if}
        <h3>Hausaufgabenverlauf</h3>
        {#if homework.length}<div class="history-scroll">
            <table>
              <thead
                ><tr
                  ><th>Fällig am</th><th>Hausaufgabe / Fach</th><th>Abgabe</th
                  ><th>Qualität / Notiz</th></tr
                ></thead
              ><tbody
                >{#each homework as item}<tr
                    ><td>{dateLabel(item.due_date)}</td><td
                      >{item.title}<small>{item.subject_name}</small></td
                    ><td
                      >{#if item.handed_in_at != null}Abgegeben<small
                          >{new Date(
                            item.handed_in_at * 1000,
                          ).toLocaleDateString("de-DE")}</small
                        >{:else if item.due_date < today}<span
                          class="missing-text">Nicht abgegeben</span
                        >{:else}Noch offen{/if}</td
                    ><td class="observation"
                      >{qualityLabels[item.quality] ||
                        "Nicht bewertet"}{#if item.note}<small
                          >{item.note}</small
                        >{/if}</td
                    ></tr
                  >{/each}</tbody
              >
            </table>
          </div>{:else}<p class="empty-panel">
            Noch keine zugewiesenen Hausaufgaben für diese Auswahl.
          </p>{/if}
      {/if}
    {/if}
  </section>
{:else}
  {#if !loading && !error}
    <ClassDevelopment {students} {data} {today} {onselect} />
  {/if}
  <section
    class="workspace-card settings-panel"
    aria-label="Kinderliste"
    aria-busy={loading}
  >
    <label class="pupil-search"
      >Kind suchen<input bind:value={search} placeholder="Name …" /></label
    >
    {#if loading}<p class="muted" role="status">
        Hausaufgabenhinweise werden geladen …
      </p>{/if}
    <div class="class-roster">
      {#each students.filter((s) => s.name
          .toLocaleLowerCase("de")
          .includes(search.toLocaleLowerCase("de"))) as s (s.id)}
        {@const count = recentMissing(
          data.homework.filter((h) => h.student_id === s.id),
          today,
        ).length}
        <div class="collaborator">
          <div class="pupil-identity">
            <span class="avatar small-avatar">{initials(s.name)}</span>
            <div>
              <button
                class="text-button pupil-name"
                onclick={() => onselect(s.id)}>{s.name}</button
              >{#if !loading && !error && count}<small class="missing-text"
                  >{count} Hausaufgabe{count === 1 ? "" : "n"} zuletzt nicht abgegeben
                  · 14 Tage</small
                >{/if}
            </div>
          </div>
          <div class="inline-actions">
            <button class="text-button" onclick={() => onselect(s.id)}
              >Verlauf</button
            ><button class="text-button" {disabled} onclick={() => onrename(s)}
              >Bearbeiten</button
            >{#if owner}<button
                class="text-button danger-text"
                {disabled}
                onclick={() => ondelete(s)}>Entfernen</button
              >{/if}
          </div>
        </div>
      {:else}<p class="empty-panel">
          {students.length
            ? "Kein passendes Kind."
            : "Füge Kinder einzeln oder als Namensliste hinzu."}
        </p>{/each}
    </div>
  </section>
{/if}

<style>
  .detail-heading,
  .summary {
    display: flex;
    gap: 16px;
    align-items: center;
    flex-wrap: wrap;
    margin: 20px 0;
  }
  .subject-filter {
    display: block;
    max-width: 300px;
  }
  .summary > span {
    padding: 14px;
    background: var(--paper, #f3f7f5);
    border-radius: 10px;
  }
  .homework-warning {
    padding: 12px 16px;
    background: #fff1dc;
    color: #805014;
    border-radius: 8px;
  }
  .missing-text {
    color: #92571a;
  }
  .pupil-name {
    font-weight: 650;
    text-align: left;
  }
  small {
    display: block;
    margin-top: 5px;
    font-size: 12px;
  }
  h3 {
    margin-top: 28px;
  }
  .muted {
    font-size: 13px;
    line-height: 1.6;
  }
  .history-scroll {
    overflow-x: auto;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    text-align: left;
    min-width: 540px;
    font-size: 13px;
  }
  th,
  td {
    padding: 14px 10px;
    border-bottom: 1px solid var(--line);
    vertical-align: top;
  }
  caption {
    text-align: left;
    font-weight: 600;
    padding: 16px 10px;
  }
  .observation {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    max-width: 400px;
  }
  @media (max-width: 700px) {
    .collaborator {
      flex-wrap: wrap;
      gap: 12px;
    }
  }
</style>
