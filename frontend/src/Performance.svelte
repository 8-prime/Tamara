<script>
  import { onMount } from "svelte";
  import { api } from "./api.js";
  import Icon from "./Icon.svelte";
  import { initials, dateLabel } from "./teaching.js";
  let {
    lesson,
    saving = $bindable(false),
    onclose,
    onerror = () => {},
  } = $props();
  let dialog = $state(null),
    students = $state([]),
    records = $state({}),
    index = $state(0),
    loading = $state(true),
    error = $state("");
  let rating = $state(0),
    observation = $state("");
  const pupil = $derived(students[index]);
  const count = $derived(Object.keys(records).length);
  onMount(async () => {
    dialog.showModal();
    try {
      const [workspace, list] = await Promise.all([
        api(`/classrooms/${lesson.classroom_id}/workspace`),
        api(`/lessons/${lesson.id}/performances`),
      ]);
      students = workspace.students;
      records = Object.fromEntries(list.map((r) => [r.student_id, r]));
      const next = students.findIndex((s) => !records[s.id]);
      choose(next < 0 ? students.length : next);
    } catch (e) {
      error = e.message;
      onerror(e);
    } finally {
      loading = false;
    }
  });
  function choose(i) {
    index = i;
    const record = records[students[i]?.id];
    rating = record?.rating || 0;
    observation = record?.observation || "";
  }
  function close() {
    if (!saving) {
      dialog.close();
      onclose();
    }
  }
  function keyboard(e) {
    if (
      saving ||
      loading ||
      e.isComposing ||
      e.repeat ||
      e.ctrlKey ||
      e.metaKey ||
      e.altKey
    )
      return;
    const text =
      e.target instanceof Element && e.target.closest("textarea,input,select");
    if (!text && /^[1-4]$/.test(e.key)) {
      e.preventDefault();
      rating = Number(e.key);
    }
    if (
      e.key === "Enter" &&
      !e.shiftKey &&
      pupil &&
      rating &&
      !(e.target instanceof Element && e.target.closest("button"))
    ) {
      e.preventDefault();
      save();
    }
  }
  async function save() {
    if (saving || !pupil || !rating) return;
    saving = true;
    error = "";
    const record = { student_id: pupil.id, rating, observation };
    try {
      await api(`/lessons/${lesson.id}/performances`, {
        method: "PATCH",
        body: JSON.stringify(record),
      });
      records = { ...records, [pupil.id]: record };
      choose(index + 1);
    } catch (e) {
      error = e.message;
      onerror(e);
    } finally {
      saving = false;
    }
  }
</script>

<dialog
  bind:this={dialog}
  class="performance-dialog"
  aria-labelledby="performance-title"
  onkeydown={keyboard}
  oncancel={(e) => {
    e.preventDefault();
    close();
  }}
>
  <div class="modal-content">
    <div class="performance-top">
      <span>{pupil ? `${index + 1} von ${students.length}` : "Leistungen"}</span
      ><progress
        value={count}
        max={students.length || 1}
        aria-label="Erfasste Leistungen"
      ></progress><button
        class="icon-button"
        onclick={close}
        disabled={saving}
        aria-label="Dialog schließen"><Icon name="close" /></button
      >
    </div>
    {#if error}<div class="alert" role="alert">{error}</div>{/if}
    {#if loading}<div class="empty-panel"><p>Kinder werden geladen …</p></div>
    {:else if pupil}
      <div class="performance-pupil">
        <span class="pupil-avatar">{initials(pupil.name)}</span>
        <h2 id="performance-title">{pupil.name}</h2>
        <p>{lesson.class_name} · {lesson.subject_name}</p>
        <small
          >{dateLabel(lesson.lesson_date)} · {lesson.starts_at}–{lesson.ends_at}</small
        >
      </div>
      <p class="field-title">Leistung bewerten</p>
      <div class="rating-options" aria-label="Bewertung von 1 bis 4">
        {#each [1, 2, 3, 4] as value}<button
            class:selected={rating === value}
            class:filled={rating >= value}
            aria-pressed={rating === value}
            aria-label={`Bewertung ${value} von 4`}
            onclick={() => (rating = value)}
            disabled={saving}
            ><Icon name="star" size={32} /><span>{value}</span></button
          >{/each}
      </div>
      <p class="muted note-help">
        Tasten 1–4 verwenden oder eine Bewertung anklicken.
      </p>
      <label
        >Optionale Notiz<textarea
          rows="4"
          maxlength="500"
          bind:value={observation}
          placeholder="Kurze Beobachtung zur Leistung …"
          disabled={saving}
        ></textarea></label
      >
      <p class="character-count">{observation.length} / 500</p>
      <div class="performance-footer">
        <button
          class="secondary"
          onclick={() => choose(index - 1)}
          disabled={saving || index === 0}>← Vorheriges</button
        ><button
          class="text-button"
          onclick={() => choose(index + 1)}
          disabled={saving}>Überspringen</button
        ><button class="primary" onclick={save} disabled={saving || !rating}
          >{saving ? "Speichert …" : "Speichern & nächstes"}<Icon
            name="arrow"
          /></button
        >
      </div>
      <p class="note-help muted">
        Enter speichert und öffnet das nächste Kind. Shift+Enter für eine neue
        Zeile.
      </p>
    {:else}
      <div class="empty-panel">
        <span class="completion-icon"><Icon name="check" size={36} /></span>
        <h2 id="performance-title">
          {students.length
            ? "Erfassung abgeschlossen"
            : "Noch keine Kinder in dieser Klasse"}
        </h2>
        <p>
          {count} von {students.length} Leistungen erfasst. {students.length -
          count
            ? `${students.length - count} noch offen.`
            : ""}
        </p>
        {#if students.length}<button
            class="secondary"
            onclick={() =>
              choose(
                Math.max(
                  0,
                  students.findIndex((s) => !records[s.id]),
                ),
              )}>Bewertungen prüfen / fortsetzen</button
          >{/if}<button class="primary" onclick={close}>Fertig</button>
      </div>
    {/if}
  </div>
</dialog>
