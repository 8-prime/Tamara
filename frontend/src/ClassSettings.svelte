<script>
  import { onMount } from "svelte";
  import { api } from "./api.js";

  let {
    classId,
    classroom,
    teachers = [],
    saving = $bindable(false),
    disabled = false,
    onshare,
    onerror,
  } = $props();
  let subjects = $state([]),
    loading = $state(true),
    error = $state(""),
    notice = $state("");
  let name = $state(""),
    editingId = $state("");
  const locked = $derived(saving || disabled || loading);

  function fail(e) {
    error = e.message;
    if (e.status === 401) onerror(e);
  }
  onMount(() => {
    let active = true;
    api(`/classrooms/${classId}/subjects`)
      .then((data) => {
        if (active) subjects = data;
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
  async function save(e) {
    e.preventDefault();
    if (locked) return;
    saving = true;
    error = "";
    notice = "";
    try {
      await api(
        `/classrooms/${classId}/subjects${editingId ? "/" + editingId : ""}`,
        {
          method: editingId ? "PATCH" : "POST",
          body: JSON.stringify({ name }),
        },
      );
      subjects = await api(`/classrooms/${classId}/subjects`);
      editingId = "";
      name = "";
      notice = "Fach gespeichert";
    } catch (e) {
      fail(e);
    } finally {
      saving = false;
    }
  }
  async function remove(subject) {
    if (locked) return;
    saving = true;
    error = "";
    notice = "";
    try {
      await api(`/classrooms/${classId}/subjects/${subject.id}`, {
        method: "DELETE",
      });
      subjects = subjects.filter((s) => s.id !== subject.id);
      if (editingId === subject.id) {
        editingId = "";
        name = "";
      }
      notice = "Fach entfernt";
    } catch (e) {
      fail(e);
    } finally {
      saving = false;
    }
  }
</script>

<section class="page-heading">
  <div>
    <span class="eyebrow">DEINE KLASSE · GUT ORGANISIERT</span>
    <h1>Klasseneinstellungen<span>.</span></h1>
    <p>Fächer und gemeinsame Arbeit an einem Ort.</p>
  </div>
</section>
{#if error}<div class="alert" role="alert">{error}</div>{/if}
{#if notice}<p class="workspace-notice" role="status">✓ {notice}</p>{/if}
<div class="settings-grid">
  <section
    class="workspace-card settings-panel"
    aria-labelledby="subjects-title"
    aria-busy={loading}
  >
    <h2 id="subjects-title">Fächer</h2>
    <p>Diese Fächer stehen für die Hausaufgaben deiner Klasse zur Verfügung.</p>
    <form class="subject-form" onsubmit={save}>
      <label for="subject-name"
        >{editingId ? "Fach umbenennen" : "Fach hinzufügen"}</label
      >
      <div class="inline-actions">
        <input
          id="subject-name"
          bind:value={name}
          maxlength="80"
          required
          placeholder="z. B. Mathematik"
          disabled={locked}
        /><button class="primary" disabled={locked}
          >{editingId ? "Speichern" : "Hinzufügen"}</button
        >
      </div>
      {#if editingId}<button
          class="text-button"
          type="button"
          disabled={locked}
          onclick={() => {
            editingId = "";
            name = "";
          }}>Bearbeitung abbrechen</button
        >{/if}
    </form>
    {#if loading}<p>Fächer werden geladen …</p>{:else if !subjects.length}<div
        class="info-box"
      >
        Noch keine Fächer. Lege das erste Fach an, um Hausaufgaben zu erstellen.
      </div>{/if}
    <ul class="subject-list">
      {#each subjects as subject (subject.id)}
        <li>
          <strong>{subject.name}</strong>
          <div class="inline-actions">
            <button
              class="text-button"
              disabled={locked}
              aria-label={`${subject.name} umbenennen`}
              onclick={() => {
                editingId = subject.id;
                name = subject.name;
              }}>Umbenennen</button
            ><button
              class="text-button danger-text"
              disabled={locked}
              aria-label={`${subject.name} entfernen`}
              onclick={() => remove(subject)}>Entfernen</button
            >
          </div>
        </li>
      {/each}
    </ul>
    <p class="muted">
      Fächer mit bestehenden Hausaufgaben bleiben für den Verlauf erhalten und
      können nur umbenannt werden.
    </p>
  </section>
  <section
    class="workspace-card settings-panel"
    aria-labelledby="class-details-title"
  >
    <h2 id="class-details-title">{classroom.name}</h2>
    <p>Schuljahr {classroom.school_year} · {classroom.student_count} Kinder</p>
    <h3>Gemeinsam arbeiten</h3>
    <p>Teile die Klasse mit anderen Lehrkräften.</p>
    <ul class="subject-list">
      {#each teachers as teacher (teacher.id)}<li>
          <div>
            <strong>{teacher.name}</strong><small
              >{teacher.role === "owner"
                ? "Klassenleitung"
                : "Lehrkraft"}</small
            >
          </div>
        </li>{/each}
    </ul>
    <button class="secondary" disabled={locked} onclick={onshare}
      >Freigaben verwalten</button
    >
  </section>
</div>
