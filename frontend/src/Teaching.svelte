<script>
  import { onMount, untrack } from "svelte";
  import { api } from "./api.js";
  import Icon from "./Icon.svelte";
  import LessonEditor from "./LessonEditor.svelte";
  import Performance from "./Performance.svelte";
  import Tasks from "./Tasks.svelte";
  import {
    dateKey,
    addDays,
    monday,
    dateLabel,
    subjectStyle,
    currentOrNext,
    lessonAction,
    taskDone,
  } from "./teaching.js";
  let {
    view,
    teacher,
    classrooms,
    classId,
    lessonId = "",
    routeDate = "",
    saving = $bindable(false),
    onaction,
    onnavigate,
    onroute,
    onclasses,
    onerror,
  } = $props();
  let lessons = $state([]),
    tasks = $state([]),
    worksheets = $state([]),
    homework = $state([]);
  let loading = $state(true),
    error = $state(""),
    notice = $state(""),
    now = $state(new Date());
  let date = $state(untrack(() => routeDate || dateKey())),
    calendarMode = $state("week"),
    selectedId = $state(untrack(() => lessonId));
  let editor = $state(false),
    editing = $state(null),
    performance = $state(null),
    materialSearch = $state("");
  let newStart = $state("08:00"),
    newEnd = $state("08:45");
  let materialLoading = $state(false),
    materialError = $state("");
  const today = $derived(dateKey(now));
  const current = $derived(currentOrNext(lessons, now));
  const action = $derived(lessonAction(current, now));
  const currentLive = $derived(
    current && new Date(`${current.lesson_date}T${current.starts_at}`) <= now,
  );
  const todayLessons = $derived(lessons.filter((l) => l.lesson_date === today));
  const selected = $derived(
    lessons.find((l) => l.id === selectedId) ||
      (view === "dashboard"
        ? current
        : lessons.find((l) => l.lesson_date === date)),
  );
  const startOfWeek = $derived(monday(date));
  const weekLessons = $derived(
    lessons.filter(
      (l) =>
        l.lesson_date >= startOfWeek &&
        l.lesson_date <= addDays(startOfWeek, 6),
    ),
  );
  const weekDays = $derived(
    Array.from(
      {
        length: weekLessons.some(
          (l) => new Date(l.lesson_date + "T12:00:00").getDay() % 6 === 0,
        )
          ? 7
          : 5,
      },
      (_, i) => addDays(startOfWeek, i),
    ),
  );
  const times = $derived(
    [...new Set(weekLessons.map((l) => l.starts_at))].sort(),
  );
  const dayLessons = $derived(lessons.filter((l) => l.lesson_date === date));
  const openTasks = $derived(tasks.filter((t) => !taskDone(t)));
  const context = $derived(
    view === "dashboard"
      ? current
      : view === "materials"
        ? lessons.find((l) => l.id === lessonId) || null
        : selected,
  );
  const contextClass = $derived(
    classrooms.find((c) => c.id === (context?.classroom_id || classId)),
  );
  const activeHomework = $derived(homework.filter((h) => h.status === "open"));
  const assigned = $derived(
    activeHomework.reduce((s, h) => s + h.assigned_count, 0),
  );
  const handed = $derived(
    activeHomework.reduce((s, h) => s + h.handed_in_count, 0),
  );
  const homeworkPercent = $derived(
    assigned ? Math.round((handed / assigned) * 100) : 0,
  );
  const visibleMaterials = $derived(
    worksheets.filter(
      (h) =>
        (!context ||
          view === "materials" ||
          h.subject_id === context.subject_id) &&
        `${h.title} ${h.subject_name} ${h.worksheet_name}`
          .toLocaleLowerCase("de")
          .includes(materialSearch.toLocaleLowerCase("de")),
    ),
  );
  let request = 0;
  export function noteLesson() {
    return context || null;
  }
  $effect(() => {
    if (routeDate) date = routeDate;
    selectedId = lessonId;
  });
  $effect(() => {
    const area = view,
      id = classId;
    untrack(() => reload(area, id));
  });
  $effect(() => {
    const id = context?.classroom_id || classId;
    let active = true;
    materialLoading = true;
    materialError = "";
    homework = [];
    worksheets = [];
    if (id)
      api(`/classrooms/${id}/homework`)
        .then((data) => {
          if (active) {
            homework = data;
            worksheets = data.filter((h) => h.worksheet_name);
          }
        })
        .catch((e) => {
          if (active) materialError = e.message;
        })
        .finally(() => {
          if (active) materialLoading = false;
        });
    else materialLoading = false;
    return () => (active = false);
  });
  onMount(() => {
    const timer = setInterval(() => {
      now = new Date();
    }, 15000);
    const refresh = () => {
      now = new Date();
      if (!saving) reload();
    };
    window.addEventListener("focus", refresh);
    return () => {
      clearInterval(timer);
      window.removeEventListener("focus", refresh);
      request++;
    };
  });
  function fail(e) {
    error = e.message;
    if (e.status === 401) onerror(e);
  }
  async function reload(area = view, id = classId) {
    const version = ++request;
    loading = true;
    error = "";
    try {
      const from =
        area === "dashboard"
          ? dateKey()
          : [addDays(dateKey(), -365), startOfWeek].sort()[0];
      const until =
        area === "dashboard"
          ? "9999-12-31"
          : [addDays(dateKey(), 366), addDays(startOfWeek, 6)].sort().at(-1);
      const query = area === "performances" && id ? `&classroom_id=${id}` : "";
      const [list, allTasks] = await Promise.all([
        api(`/lessons?from=${from}&until=${until}${query}`),
        api("/tasks"),
      ]);
      if (version !== request) return;
      lessons = list;
      tasks = allTasks;
      if (lessonId && !list.some((l) => l.id === lessonId)) {
        const extra = await api(`/lessons/${lessonId}`);
        if (version === request)
          lessons = [...list, extra].sort((a, b) =>
            (a.lesson_date + a.starts_at).localeCompare(
              b.lesson_date + b.starts_at,
            ),
          );
      }
    } catch (e) {
      if (version === request) fail(e);
    } finally {
      if (version === request) loading = false;
    }
  }
  async function selectLesson(lesson) {
    if (saving) return;
    selectedId = lesson.id;
    date = lesson.lesson_date;
    onroute(lesson.id, lesson.lesson_date);
  }
  async function changeDate(value) {
    if (saving) return;
    date = value;
    selectedId = "";
    onroute("", date);
    await reload();
  }
  function newLesson(day = date, time = "08:00") {
    if (saving) return;
    date = day;
    newStart = time;
    const minutes = Math.min(
      1439,
      Number(time.slice(0, 2)) * 60 + Number(time.slice(3)) + 45,
    );
    newEnd = `${String(Math.floor(minutes / 60)).padStart(2, "0")}:${String(minutes % 60).padStart(2, "0")}`;
    editing = null;
    editor = true;
  }
  function act(kind, lesson = current) {
    if (kind === "create") {
      newLesson(today);
      return;
    }
    if (!lesson) {
      if (classId && kind === "materials") {
        onnavigate("materials", { classroom_id: classId });
        return;
      }
      if (classId && ["attendance", "note"].includes(kind)) {
        onaction(kind, { classroom_id: classId, lesson_date: today });
        return;
      }
      notice = "Wähle zuerst eine Stunde im Kalender aus.";
      onnavigate("calendar");
      return;
    }
    if (kind === "prepare") {
      onnavigate("calendar", lesson);
      return;
    }
    if (kind === "performance") {
      performance = lesson;
      return;
    }
    if (kind === "materials") {
      onnavigate("materials", lesson);
      return;
    }
    onaction(kind, lesson);
  }
  async function saved(id, day) {
    date = day;
    selectedId = id;
    onroute(id, day);
    await reload();
  }
  async function closePerformance() {
    performance = null;
    await reload();
  }
  async function cancelSeries(lesson) {
    const from =
      lesson.lesson_date > today ? lesson.lesson_date : addDays(today, 1);
    if (
      !window.confirm(
        `Alle künftigen Stunden dieser Serie ab ${dateLabel(from)} absagen? Stunden mit Notizen oder Leistungen bleiben erhalten.`,
      )
    )
      return;
    saving = true;
    try {
      await api(`/timetable/${lesson.timetable_entry_id}/cancel`, {
        method: "POST",
        body: JSON.stringify({ from }),
      });
      await reload();
    } catch (e) {
      fail(e);
    } finally {
      saving = false;
    }
  }
  async function copyMaterial(h) {
    try {
      await navigator.clipboard.writeText(
        `${window.location.origin}/api/classrooms/${contextClass.id}/homework/${h.id}/worksheet`,
      );
      notice =
        "Materiallink kopiert. Zum Öffnen ist eine Klassenfreigabe erforderlich.";
    } catch {
      notice =
        "Der Link konnte nicht kopiert werden. Öffne das PDF und kopiere die Adresse.";
    }
  }
</script>

{#snippet schedule(list)}
  <div class="day-schedule">
    {#if !list.length}<div class="empty-panel small-empty">
        <Icon name="calendar" size={28} />
        <h3>Hier ist noch Platz für Unterricht</h3>
        <p>Ergänze eine Stunde mit Klasse und Fach.</p>
        <button
          class="secondary"
          onclick={() => newLesson(view === "dashboard" ? today : date)}
          >Stunde hinzufügen</button
        >
      </div>
    {:else}{#each list as lesson (lesson.id)}
        <button
          class="schedule-row"
          class:current={lesson.id ===
            (view === "dashboard" ? current?.id : selected?.id)}
          class:cancelled={lesson.status === "cancelled"}
          onclick={() =>
            view === "dashboard"
              ? onnavigate("calendar", lesson)
              : selectLesson(lesson)}
        >
          <time>{lesson.starts_at}–{lesson.ends_at}</time><span
            class={`subject-icon ${subjectStyle(lesson.subject_name)}`}
            ><Icon
              name={/deutsch|englisch/i.test(lesson.subject_name)
                ? "book"
                : "chart"}
            /></span
          ><strong
            >{lesson.subject_name}{#if lesson.status === "cancelled"}<small
                >Fällt aus</small
              >{/if}</strong
          ><span>{lesson.class_name}</span><span class="schedule-room"
            >{lesson.room ? `Raum ${lesson.room}` : ""}</span
          ><Icon name="chevron" />
        </button>
      {/each}{/if}
  </div>
{/snippet}

{#snippet materialList(items)}
  {#if materialLoading}<p class="muted">
      Materialien werden geladen …
    </p>{:else if materialError}<p class="alert" role="alert">
      {materialError}
    </p>{:else if !items.length}<p class="muted">
      Noch keine PDF-Arbeitsblätter {context
        ? "für dieses Fach"
        : "in dieser Klasse"}. Du kannst sie bei einer Hausaufgabe ergänzen.
    </p>{:else}<ul class="lesson-materials">
      {#each items as h (h.id)}<li>
          <Icon name="note" /><a
            href={`/api/classrooms/${contextClass.id}/homework/${h.id}/worksheet`}
            target="_blank"
            rel="noreferrer"
            >{h.worksheet_name}<small>{h.subject_name} · {h.title}</small></a
          >{#if view === "materials"}<button
              class="text-button"
              onclick={() => copyMaterial(h)}>Link kopieren</button
            >{/if}
        </li>{/each}
    </ul>{/if}
{/snippet}

{#if error}<div class="alert" role="alert">
    {error}<button class="text-button" onclick={() => reload()}
      >Erneut versuchen</button
    >
  </div>{/if}
{#if notice}<p class="workspace-notice" role="status">{notice}</p>{/if}

{#if view === "dashboard"}
  <section class="dashboard-greeting">
    <p>
      {dateLabel(today, {
        weekday: "long",
        day: "numeric",
        month: "long",
        year: "numeric",
      })}
    </p>
    <h1>
      {now.getHours() < 11
        ? "Guten Morgen"
        : now.getHours() < 18
          ? "Guten Tag"
          : "Guten Abend"}, {teacher.name}!
    </h1>
  </section>
  {#if loading}<p class="muted" role="status">
      Dein Schultag wird geladen …
    </p>{/if}
  <div class="dashboard-layout">
    <div class="dashboard-top">
      <section class="lesson-hero">
        <div class="hero-content">
          <div class="hero-meta">
            <span class="live-pill"
              ><Icon name="clock" />{current
                ? currentLive
                  ? "Aktuelle Stunde"
                  : "Als Nächstes"
                : "Dein Unterricht"}</span
            >{#if current}<span>{current.starts_at}–{current.ends_at}</span
              >{/if}
          </div>
          <h2>{current?.subject_name || "Dein Schultag beginnt hier"}</h2>
          {#if current}<p class="hero-class">
              Klasse {current.class_name}{#if current.room}
                · Raum {current.room}{/if}
            </p>
            {#if current.lesson_date !== today}<p>
                {dateLabel(current.lesson_date, {
                  weekday: "long",
                  day: "numeric",
                  month: "long",
                })}
              </p>{/if}
            <div class="hero-topic">
              <span>Thema der Stunde</span>
              <p>{current.topic || "Was steht heute im Mittelpunkt?"}</p>
            </div>
          {:else}<p class="hero-topic">
              Plane deine erste Stunde. Anwesenheit, Notizen und Leistungen sind
              dann direkt zur Hand.
            </p>{/if}
          <button
            class="primary predictive-action"
            onclick={() => act(action.kind)}
            disabled={saving || loading}
            ><Icon name={action.icon} size={23} /><span>{action.label}</span
            ><Icon name="arrow" /></button
          >
          {#if currentLive && current?.attendance_count < current?.pupil_count && action.kind !== "attendance"}<button
              class="text-button attendance-reminder"
              onclick={() => act("attendance")}
              >Anwesenheit: {current.attendance_count} von {current.pupil_count}
              erfasst</button
            >{/if}
        </div>
        <div class="classroom-scene" aria-hidden="true">
          <div class="scene-board">
            Aa + Bb<span>Jeder Schritt zählt.</span>
          </div>
          <div class="scene-desk"></div>
          <div class="scene-books"></div>
        </div>
      </section>
      <section class="workspace-card today-panel">
        <div class="panel-heading">
          <h2>Heutiger Stundenplan</h2>
          <button class="text-button" onclick={() => onnavigate("calendar")}
            >{dateLabel(today, { day: "numeric", month: "short" })}<Icon
              name="chevron"
            /></button
          >
        </div>
        {@render schedule(todayLessons)}
      </section>
    </div>
    <section class="workspace-card quick-actions-panel">
      <div class="panel-heading">
        <h2>
          <span class="gold"><Icon name="bolt" size={25} /></span> Schnellaktionen
        </h2>
      </div>
      <div class="quick-actions">
        {#each [{ kind: "attendance", title: "Anwesenheit erfassen", icon: "users", color: "blue" }, { kind: "note", title: "Notiz erfassen", icon: "note", color: "orange" }, { kind: "performance", title: "Leistungen dokumentieren", icon: "chart", color: "green" }, { kind: "materials", title: "Material finden & teilen", icon: "folder", color: "green" }] as item}
          <button onclick={() => act(item.kind)} disabled={saving || loading}
            ><span class={item.color}><Icon name={item.icon} size={28} /></span
            ><strong>{item.title}</strong><Icon name="chevron" /></button
          >
        {/each}
      </div>
    </section>
    <div class="dashboard-bottom">
      <section class="workspace-card">
        <div class="panel-heading">
          <h2>
            <span class="orange"><Icon name="check" size={24} /></span> Wichtige
            Aufgaben
          </h2>
          <button class="text-button" onclick={() => onnavigate("tasks")}
            >Alle Aufgaben <Icon name="arrow" /></button
          >
        </div>
        {#if !openTasks.length}<div class="empty-panel small-empty">
            <h3>Raum für das Wesentliche</h3>
            <p>Hier erscheinen offene Aufgaben deiner Klassen.</p>
            <button class="secondary" onclick={() => onnavigate("tasks")}
              >Aufgabe erstellen</button
            >
          </div>{:else}{#each openTasks.slice(0, 5) as task}<button
              class="dashboard-task"
              onclick={() =>
                onnavigate("tasks", { classroom_id: task.classroom_id })}
              ><span class="task-box"></span><span
                ><strong>{task.title}</strong><small
                  class:overdue={task.due_date && task.due_date < today}
                  >{task.class_name} · {task.completion_mode === "per_pupil"
                    ? `${task.assigned_count - task.completed_count} noch offen`
                    : task.due_date
                      ? dateLabel(task.due_date, {
                          day: "numeric",
                          month: "long",
                        })
                      : "Einmal für die Klasse"}</small
                ></span
              ><Icon name="chevron" /></button
            >{/each}{/if}
      </section>
      <section class="workspace-card">
        <div class="panel-heading">
          <h2>
            <span class="green"><Icon name="users" size={24} /></span>
            {contextClass ? `Klasse ${contextClass.name}` : "Deine Klassen"} – Überblick
          </h2>
        </div>
        <div class="class-overview">
          <div>
            <h3>Hausaufgaben</h3>
            <div
              class="homework-ring"
              style={`--completion:${homeworkPercent}%`}
            >
              <span
                ><strong>{assigned ? homeworkPercent + " %" : "–"}</strong
                ><small>abgegeben</small></span
              >
            </div>
            <p class="muted">
              {handed} von {assigned} Abgaben · offene Hausaufgaben
            </p>
            <button
              class="text-button"
              onclick={() => onnavigate("homework", context)}
              >Details ansehen <Icon name="arrow" /></button
            >
          </div>
          <div>
            <h3>Anwesenheit</h3>
            <div class="attendance-total">
              {context?.present_count ?? "–"}<span>
                / {context?.pupil_count ??
                  contextClass?.student_count ??
                  "–"}</span
              >
            </div>
            <p class="muted">
              {context
                ? dateLabel(context.lesson_date, {
                    day: "numeric",
                    month: "long",
                  })
                : "Wähle eine Klasse"}
            </p>
            <p>
              {context
                ? `${context.attendance_count} Kinder erfasst`
                : `${classrooms.length} Klassen in deinem Arbeitsbereich`}
            </p>
            <button
              class="text-button"
              onclick={() =>
                context ? act("attendance", context) : onnavigate("students")}
              >Anwesenheit öffnen <Icon name="arrow" /></button
            >
          </div>
        </div>
      </section>
    </div>
  </div>
{:else if view === "materials"}
  <section class="page-heading with-actions">
    <div>
      <h1>Materialien</h1>
      <p>
        Arbeitsblätter für Klasse {contextClass?.name}. Finden, öffnen und
        teilen.
      </p>
    </div>
    <button class="primary" onclick={() => onnavigate("homework", context)}
      ><Icon name="plus" /> Arbeitsblatt ergänzen</button
    >
  </section>
  <section class="workspace-card">
    <label
      >Material suchen<input
        type="search"
        bind:value={materialSearch}
        placeholder="Dateiname, Fach oder Hausaufgabe …"
      /></label
    >{@render materialList(visibleMaterials)}
    <p class="note-help muted">
      Materiallinks können nur Lehrkräfte mit Zugriff auf die Klasse öffnen.
    </p>
  </section>
{:else}
  <section class="page-heading with-actions">
    <div>
      <h1>
        {view === "calendar"
          ? "Stundenplan & Unterrichtsplanung"
          : "Unterrichtsleistungen"}
      </h1>
      <p>
        {view === "calendar"
          ? "Deine Woche planen. Jede Stunde im Blick."
          : "Leistungen für eine konkrete Stunde dokumentieren und wiederfinden."}
      </p>
    </div>
    <button class="primary" onclick={() => newLesson()} disabled={saving}
      ><Icon name="plus" /> Stunde hinzufügen</button
    >
  </section>
  <div class="planning-layout">
    <div class="planning-main">
      <div class="calendar-toolbar">
        <div class="date-navigation">
          <button
            class="icon-button"
            onclick={() =>
              changeDate(
                addDays(
                  date,
                  calendarMode === "week" && view === "calendar" ? -7 : -1,
                ),
              )}
            aria-label="Vorheriger Zeitraum">‹</button
          ><label
            ><span class="sr-only">Datum auswählen</span><input
              type="date"
              value={date}
              onchange={(e) => changeDate(e.currentTarget.value || today)}
            /></label
          ><button
            class="icon-button"
            onclick={() =>
              changeDate(
                addDays(
                  date,
                  calendarMode === "week" && view === "calendar" ? 7 : 1,
                ),
              )}
            aria-label="Nächster Zeitraum">›</button
          >
        </div>
        {#if view === "calendar"}<div class="segmented">
            <button
              class:active={calendarMode === "week"}
              onclick={() => (calendarMode = "week")}>Woche</button
            ><button
              class:active={calendarMode === "day"}
              onclick={() => (calendarMode = "day")}>Tag</button
            >
          </div>{/if}<button class="secondary" onclick={() => changeDate(today)}
          ><Icon name="calendar" /> Heute</button
        >
      </div>
      {#if loading}<p class="muted" role="status">
          Stunden werden geladen …
        </p>{/if}
      {#if view === "calendar" && calendarMode === "week"}
        <section class="workspace-card timetable-panel">
          <div class="week-caption">
            {dateLabel(startOfWeek, { day: "numeric", month: "short" })} – {dateLabel(
              addDays(startOfWeek, weekDays.length - 1),
            )}
          </div>
          {#if !weekLessons.length}<div class="empty-panel">
              <Icon name="calendar" size={38} />
              <h2>Deine Woche ist noch frei</h2>
              <p>
                Füge eine einzelne oder wöchentlich wiederkehrende Stunde hinzu.
              </p>
              <button class="primary" onclick={() => newLesson()}
                >Erste Stunde hinzufügen</button
              >
            </div>
          {:else}<div class="timetable-scroll">
              <table class="timetable-grid">
                <thead
                  ><tr
                    ><th scope="col">Zeit</th>{#each weekDays as day}<th
                        scope="col"
                        class:today={day === today}
                        ><button
                          onclick={() => {
                            changeDate(day);
                            calendarMode = "day";
                          }}
                          >{dateLabel(day, { weekday: "short" })}<small
                            >{dateLabel(day, {
                              day: "numeric",
                              month: "short",
                            })}</small
                          ></button
                        ></th
                      >{/each}</tr
                  ></thead
                ><tbody
                  >{#each times as time}<tr
                      ><th scope="row">{time}</th>{#each weekDays as day}<td
                          >{#each weekLessons.filter((l) => l.lesson_date === day && l.starts_at === time) as lesson}<button
                              class={`timetable-lesson ${subjectStyle(lesson.subject_name)}`}
                              class:selected={lesson.id === selected?.id}
                              class:cancelled={lesson.status === "cancelled"}
                              onclick={() => selectLesson(lesson)}
                              ><strong>{lesson.subject_name}</strong><span
                                >{lesson.class_name}</span
                              ><small
                                >{lesson.starts_at}–{lesson.ends_at}{lesson.status ===
                                "cancelled"
                                  ? " · Fällt aus"
                                  : ""}</small
                              ></button
                            >{:else}<button
                              class="empty-slot"
                              onclick={() => newLesson(day, time)}
                              aria-label={`Stunde am ${dateLabel(day)} hinzufügen`}
                              >+</button
                            >{/each}</td
                        >{/each}</tr
                    >{/each}</tbody
                >
              </table>
            </div>{/if}
        </section>
      {/if}
      <section class="workspace-card day-panel">
        <div class="panel-heading">
          <h2>
            {view === "performances" ? "Stunden & Leistungen" : "Stundenplan"} ·
            {dateLabel(date, {
              weekday: "long",
              day: "numeric",
              month: "long",
            })}
          </h2>
        </div>
        {@render schedule(dayLessons)}
        {#if view === "performances" && dayLessons.length}<p
            class="muted note-help"
          >
            Wähle eine Stunde. Bereits erfasste Bewertungen lassen sich prüfen
            und korrigieren.
          </p>{/if}
      </section>
    </div>
    <aside class="lesson-detail-column">
      {#if selected}
        <section class="workspace-card lesson-detail">
          <div class="lesson-detail-title">
            <span
              class={`subject-icon large ${subjectStyle(selected.subject_name)}`}
              ><Icon name="chart" size={29} /></span
            >
            <div>
              <h2>{selected.subject_name}</h2>
              <p>
                Klasse {selected.class_name} · {dateLabel(
                  selected.lesson_date,
                  { weekday: "short", day: "numeric", month: "short" },
                )}
              </p>
              <p class="muted">
                {selected.starts_at}–{selected.ends_at}{selected.room
                  ? ` · Raum ${selected.room}`
                  : ""}
              </p>
            </div>
            <button
              class="icon-button"
              onclick={() => {
                editing = selected;
                editor = true;
              }}
              disabled={saving}
              aria-label="Stunde bearbeiten"><Icon name="edit" /></button
            >
          </div>
          {#if selected.status === "cancelled"}<p class="alert">
              Diese Stunde fällt aus. Vorhandene Einträge bleiben erhalten.
            </p>{/if}
          <div class="lesson-detail-section">
            <div class="panel-heading">
              <h3><Icon name="note" /> Thema der Stunde</h3>
              <button
                class="text-button"
                onclick={() => {
                  editing = selected;
                  editor = true;
                }}>Bearbeiten</button
              >
            </div>
            <p>{selected.topic || "Noch kein Thema festgelegt."}</p>
          </div>
          <div class="lesson-detail-section">
            <div class="panel-heading">
              <h3><Icon name="check" /> Lernziele</h3>
              <button
                class="text-button"
                onclick={() => {
                  editing = selected;
                  editor = true;
                }}>Bearbeiten</button
              >
            </div>
            {#if selected.learning_goals}<ul class="learning-goals">
                {#each selected.learning_goals
                  .split("\n")
                  .filter((g) => g.trim()) as goal}<li>{goal}</li>{/each}
              </ul>{:else}<p class="muted">
                Was sollen die Kinder heute mitnehmen?
              </p>{/if}
          </div>
          <div class="lesson-detail-section">
            <div class="panel-heading">
              <h3><Icon name="folder" /> Materialien</h3>
              <button
                class="text-button"
                onclick={() => act("materials", selected)}>Alle</button
              >
            </div>
            {@render materialList(visibleMaterials)}
          </div>
          <div class="lesson-detail-section">
            {#key selected.id}<Tasks
                classId={selected.classroom_id}
                lessonId={selected.id}
                compact
                bind:saving
                {onerror}
              />{/key}
          </div>
          {#if selected.timetable_entry_id && selected.teacher_id === teacher.id}<button
              class="text-button danger-text series-action"
              onclick={() => cancelSeries(selected)}
              disabled={saving}>Künftige Stunden dieser Serie absagen</button
            >{/if}
        </section>
        <section class="workspace-card lesson-tools">
          <h3>
            <span class="gold"><Icon name="bolt" /></span> Werkzeuge zur Stunde
          </h3>
          <div class="lesson-tool-buttons">
            <button
              class="secondary"
              onclick={() => act("attendance", selected)}
              ><span class="blue"><Icon name="users" /></span
              >Anwesenheit</button
            ><button class="secondary" onclick={() => act("note", selected)}
              ><span class="orange"><Icon name="note" /></span>Notiz zur Stunde</button
            ><button
              class="secondary"
              onclick={() => act("performance", selected)}
              disabled={selected.status === "cancelled"}
              ><span class="green"><Icon name="chart" /></span>Leistung erfassen</button
            >
          </div>
          <p class="muted note-help">
            {selected.attendance_count}/{selected.pupil_count} Anwesenheiten · {selected.performance_count}/{selected.pupil_count}
            Leistungen
          </p>
        </section>
      {:else}<section class="workspace-card empty-panel">
          <Icon name="book" size={36} />
          <h2>Deine Stunde im Detail</h2>
          <p>
            Wähle einen Eintrag im Stundenplan oder füge eine neue Stunde hinzu.
          </p>
        </section>{/if}
    </aside>
  </div>
{/if}
{#if editor}<LessonEditor
    {classrooms}
    lesson={editing}
    initialDate={date}
    initialClassId={classId}
    initialStart={newStart}
    initialEnd={newEnd}
    bind:saving
    onsaved={saved}
    onclose={() => (editor = false)}
    {onclasses}
  />{/if}
{#if performance}<Performance
    lesson={performance}
    bind:saving
    onclose={closePerformance}
    {onerror}
  />{/if}
