<script>
  import { onMount, tick } from "svelte";
  import { api } from "./api.js";
  import Status from "./Status.svelte";
  import Icon from "./Icon.svelte";
  import Attendance from "./Attendance.svelte";
  import Homework from "./Homework.svelte";
  import Notes from "./Notes.svelte";
  import ClassSettings from "./ClassSettings.svelte";
  import { readRoute, workspaceUrl, workAreas } from "./navigation.js";
  let feature = $state("learning");
  let homeworkTab = $state("open"),
    homeworkSubject = $state("");
  let teacher = $state(null),
    booting = $state(true),
    authMode = $state("login"),
    setupRequired = $state(false),
    registrationToken = $state("");
  let invitations = $state([]),
    invitationToken = $state("");
  let backupCreated = $state(false);
  let email = $state(""),
    password = $state(""),
    accountName = $state("");
  let error = $state(""),
    notice = $state(""),
    busy = $state(false),
    loading = $state(false),
    saving = $state(false);
  let classrooms = $state([]),
    classId = $state(""),
    catalog = $state([]),
    workspace = $state(null);
  let periodId = $state(""),
    studentId = $state(""),
    mode = $state("student");
  let section = $state(""),
    level = $state("all"),
    schoolGrade = $state("all"),
    search = $state(""),
    rosterSearch = $state("");
  let criterionId = $state(""),
    selected = $state(new Set()),
    matrixPage = $state(0),
    exportingZip = $state(false);
  let modal = $state(""),
    dialog = $state(null),
    fieldName = $state(""),
    fieldYear = $state("2026/27"),
    fieldNames = $state(""),
    fieldSource = $state(""),
    fieldEmail = $state(""),
    targetId = $state("");
  let loadVersion = 0;
  let notesView = $state(null);
  const modes = [
    { id: "student", label: "Einzelkind", icon: "◉" },
    { id: "class", label: "Klassenübersicht", icon: "▦" },
    { id: "criterion", label: "Nach Kompetenz", icon: "☷" },
  ];
  const levels = [
    "Basis",
    ...Array.from({ length: 10 }, (_, i) => String(i + 1).padStart(2, "0")),
  ];
  const gradeRanges = [
    { id: "1", first: 1, last: 4, target: "03" },
    { id: "2", first: 3, last: 6, target: "05" },
    { id: "3", first: 5, last: 8, target: "07" },
    { id: "4", first: 7, last: 10, target: "09" },
  ];
  const gradeRange = $derived(gradeRanges.find((g) => g.id === schoolGrade));
  const relevantLevels = $derived(
    gradeRange
      ? levels.filter(
          (l) =>
            l !== "Basis" &&
            Number(l) >= gradeRange.first &&
            Number(l) <= gradeRange.last,
        )
      : levels,
  );
  const currentClass = $derived(classrooms.find((c) => c.id === classId));
  const currentArea = $derived(workAreas.find((area) => area.id === feature));
  const students = $derived(workspace?.students || []),
    periods = $derived(workspace?.periods || []);
  const currentPeriod = $derived(periods.find((p) => p.id === periodId)),
    currentStudent = $derived(students.find((s) => s.id === studentId));
  const sections = $derived([...new Set(catalog.map((c) => c.section))]);
  const domains = $derived([...new Set(catalog.map((c) => c.domain))]);
  const sectionInfo = $derived(catalog.find((c) => c.section === section));
  const filtered = $derived(
    catalog.filter(
      (c) =>
        (!section || c.section === section) &&
        relevantLevels.includes(c.level) &&
        (level === "all" || c.level === level) &&
        (!search ||
          c.label
            .toLocaleLowerCase("de")
            .includes(search.toLocaleLowerCase("de"))),
    ),
  );
  const visibleStudents = $derived(
    students.filter((s) =>
      s.name
        .toLocaleLowerCase("de")
        .includes(rosterSearch.toLocaleLowerCase("de")),
    ),
  );
  const currentCriterion = $derived(
    filtered.find((c) => c.id === criterionId) || filtered[0],
  );
  const grades = $derived.by(() => {
    const m = {};
    for (const a of workspace?.assessments || [])
      m[a.student_id + ":" + a.criterion_id] = a;
    return m;
  });
  const assessedCount = $derived(workspace?.assessments?.length || 0),
    totalCount = $derived(students.length * catalog.length);
  const percentage = $derived(
    totalCount ? Math.round((assessedCount / totalCount) * 100) : 0,
  );
  const grouped = $derived(
    levels
      .map((l) => ({
        level: l,
        band: filtered.find((c) => c.level === l)?.band || "white",
        criteria: filtered.filter((c) => c.level === l),
      }))
      .filter((g) => g.criteria.length),
  );
  const matrixCriteria = $derived(
      filtered.slice(matrixPage * 8, matrixPage * 8 + 8),
    ),
    matrixPages = $derived(Math.max(1, Math.ceil(filtered.length / 8)));
  const editable = $derived(!!periodId && !saving && !loading && !busy);
  function value(s, c) {
    return grades[s + ":" + c]?.value || null;
  }
  function achievedOn(s, c) {
    return grades[s + ":" + c]?.achieved_on || null;
  }
  function today() {
    const d = new Date();
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  }
  function countFor(s, v = null) {
    return (workspace?.assessments || []).filter(
      (a) => a.student_id === s && (!v || a.value === v),
    ).length;
  }
  function initials(n) {
    return n
      .split(/\s+/)
      .map((p) => p[0])
      .slice(0, 2)
      .join("")
      .toUpperCase();
  }
  function fail(e) {
    error = e.message;
    if (e.status === 401) {
      teacher = null;
      workspace = null;
      classrooms = [];
    }
  }
  function resetFilters(resetGrade = true) {
    if (resetGrade) schoolGrade = "all";
    level = "all";
    search = "";
    matrixPage = 0;
    selected = new Set();
  }
  onMount(async () => {
    try {
      const status = await api("/auth/status");
      setupRequired = status.setup_required;
      if (setupRequired) authMode = "register";
      teacher = await api("/auth/me");
      await initialize();
    } catch (e) {
      if (e.status !== 401) error = e.message;
    } finally {
      booting = false;
    }
  });
  onMount(() => {
    const restore = async () => {
      if (!teacher) return;
      if (
        saving ||
        busy ||
        loading ||
        modal ||
        document.querySelector("dialog[open]")
      ) {
        writeRoute(true);
        return;
      }
      const route = readRoute();
      feature = route.feature;
      homeworkTab = route.tab;
      homeworkSubject = route.subject;
      const nextClass = classrooms.some((c) => c.id === route.classId)
        ? route.classId
        : classrooms[0]?.id || "";
      if (nextClass !== classId) await chooseClass(nextClass, false);
      writeRoute(true);
    };
    window.addEventListener("popstate", restore);
    return () => window.removeEventListener("popstate", restore);
  });
  function writeRoute(replace = false) {
    const url = workspaceUrl(classId, feature, homeworkTab, homeworkSubject);
    if (window.location.pathname + window.location.search !== url)
      window.history[replace ? "replaceState" : "pushState"]({}, "", url);
  }
  function navigate(area, event = null) {
    if (event) {
      if (
        event.button !== 0 ||
        event.metaKey ||
        event.ctrlKey ||
        event.shiftKey ||
        event.altKey
      )
        return;
      event.preventDefault();
    }
    if (saving || busy || loading || !classId) return;
    feature = area;
    error = "";
    notice = "";
    writeRoute();
  }
  async function quickNote(event) {
    if (
      event.key.toLowerCase() !== "n" ||
      event.repeat ||
      event.isComposing ||
      event.ctrlKey ||
      event.metaKey ||
      event.altKey ||
      event.shiftKey ||
      !teacher ||
      !classId ||
      !students.length ||
      saving ||
      busy ||
      loading ||
      modal ||
      document.querySelector("dialog[open]") ||
      (event.target instanceof Element &&
        event.target.closest(
          'input, textarea, select, [contenteditable]:not([contenteditable="false"]), [role="textbox"]',
        ))
    )
      return;
    event.preventDefault();
    navigate("notes");
    await tick();
    notesView?.openNew();
  }
  function homeworkFilters(tab, subject) {
    homeworkTab = tab;
    homeworkSubject = subject;
    writeRoute(true);
  }
  async function initialize(preferCurrent = false) {
    const [c, cat] = await Promise.all([api("/classrooms"), api("/criteria")]);
    classrooms = c;
    catalog = cat.criteria;
    section ||= cat.criteria[0]?.section || "";
    if (!preferCurrent) {
      const route = readRoute();
      classId = route.classId;
      feature = route.feature;
      homeworkTab = route.tab;
      homeworkSubject = route.subject;
    }
    if (!c.some((x) => x.id === classId)) classId = c[0]?.id || "";
    if (classId) await loadWorkspace();
    writeRoute(true);
  }
  async function authenticate(e) {
    e.preventDefault();
    busy = true;
    error = "";
    try {
      teacher = await api(
        "/auth/" + (authMode === "login" ? "login" : "register"),
        {
          method: "POST",
          body: JSON.stringify({
            email,
            password,
            name: accountName,
            registration_token: registrationToken,
          }),
        },
      );
      password = "";
      registrationToken = "";
      setupRequired = false;
      await initialize();
    } catch (e) {
      fail(e);
    } finally {
      busy = false;
    }
  }
  async function signout() {
    if (saving || busy) return;
    busy = true;
    try {
      await api("/auth/logout", { method: "POST" });
      teacher = null;
      workspace = null;
      classrooms = [];
      classId = "";
      periodId = "";
      email = "";
      notice = "";
      error = "";
      window.history.replaceState({}, "", "/");
    } catch (e) {
      fail(e);
    } finally {
      busy = false;
    }
  }
  async function loadWorkspace(preferred = periodId) {
    if (!classId) return;
    const version = ++loadVersion;
    loading = true;
    error = "";
    selected = new Set();
    try {
      const w = await api(
        `/classrooms/${classId}/workspace${preferred ? "?period_id=" + preferred : ""}`,
      );
      if (version !== loadVersion) return;
      workspace = w;
      periodId = w.period_id || "";
      if (!w.students.some((s) => s.id === studentId))
        studentId = w.students[0]?.id || "";
    } catch (e) {
      if (version === loadVersion) fail(e);
    } finally {
      if (version === loadVersion) loading = false;
    }
  }
  async function chooseClass(id, pushRoute = true) {
    if (saving || busy || loading) return;
    classId = id;
    periodId = "";
    workspace = null;
    studentId = "";
    rosterSearch = "";
    if (pushRoute) homeworkSubject = "";
    await loadWorkspace("");
    if (pushRoute) writeRoute();
  }
  async function choosePeriod(e) {
    periodId = e.currentTarget.value;
    await loadWorkspace();
  }
  async function save(changes) {
    if (!editable || !changes.length) return;
    changes = changes.map((c) => ({
      ...c,
      achieved_on:
        c.value === "yes"
          ? c.achieved_on || achievedOn(c.student_id, c.criterion_id) || today()
          : null,
    }));
    saving = true;
    error = "";
    notice = "";
    try {
      await api(`/classrooms/${classId}/periods/${periodId}/assessments`, {
        method: "PATCH",
        body: JSON.stringify({ changes }),
      });
      const keys = new Set(
        changes.map((c) => c.student_id + ":" + c.criterion_id),
      );
      workspace = {
        ...workspace,
        assessments: [
          ...workspace.assessments.filter(
            (a) => !keys.has(a.student_id + ":" + a.criterion_id),
          ),
          ...changes.filter((c) => c.value !== null),
        ],
      };
      notice = "Alle Änderungen gespeichert";
    } catch (e) {
      fail(e);
    } finally {
      saving = false;
    }
  }
  function grade(s, c, v, date = null) {
    return save([
      { student_id: s, criterion_id: c, value: v, achieved_on: date },
    ]);
  }
  function toggleSelected(id) {
    const next = new Set(selected);
    next.has(id) ? next.delete(id) : next.add(id);
    selected = next;
  }
  async function bulk(v) {
    if (currentCriterion)
      await save(
        [...selected].map((s) => ({
          student_id: s,
          criterion_id: currentCriterion.id,
          value: v,
        })),
      );
  }
  async function openModal(kind) {
    modal = kind;
    error = "";
    fieldName = "";
    fieldNames = "";
    fieldEmail = "";
    fieldSource = "";
    invitationToken = "";
    backupCreated = false;
    await tick();
    dialog.showModal();
    if (kind === "invites") {
      try {
        invitations = await api("/admin/invitations");
      } catch (e) {
        fail(e);
      }
    }
  }
  function closeModal() {
    if (!busy) {
      dialog.close();
      modal = "";
    }
  }
  async function submitModal(e) {
    e.preventDefault();
    busy = true;
    error = "";
    try {
      if (modal === "backup") {
        await api("/admin/backups", { method: "POST" });
        backupCreated = true;
        return;
      }
      if (modal === "invites") {
        const r = await api("/admin/invitations", {
          method: "POST",
          body: JSON.stringify({ label: fieldName }),
        });
        invitationToken = r.token;
        fieldName = "";
        invitations = await api("/admin/invitations");
        busy = false;
        return;
      }
      if (modal === "class") {
        const r = await api("/classrooms", {
          method: "POST",
          body: JSON.stringify({ name: fieldName, school_year: fieldYear }),
        });
        classId = r.id;
        periodId = "";
        studentId = "";
        workspace = null;
        await initialize(true);
      } else if (modal === "students") {
        await api(`/classrooms/${classId}/students`, {
          method: "POST",
          body: JSON.stringify({
            names: fieldNames
              .split("\n")
              .map((s) => s.trim())
              .filter(Boolean),
          }),
        });
        classrooms = await api("/classrooms");
        await loadWorkspace();
      } else if (modal === "period") {
        const r = await api(`/classrooms/${classId}/periods`, {
          method: "POST",
          body: JSON.stringify({
            name: fieldName,
            source_period_id: fieldSource || null,
          }),
        });
        periodId = r.id;
        await loadWorkspace();
      } else if (modal === "share") {
        await api(`/classrooms/${classId}/teachers`, {
          method: "POST",
          body: JSON.stringify({ email: fieldEmail }),
        });
        fieldEmail = "";
        await loadWorkspace();
        busy = false;
        return;
      } else if (modal === "rename") {
        await api(`/classrooms/${classId}/students/${targetId}`, {
          method: "PATCH",
          body: JSON.stringify({ name: fieldName }),
        });
        await loadWorkspace();
      } else if (modal === "delete") {
        await api(`/classrooms/${classId}/students/${targetId}`, {
          method: "DELETE",
        });
        classrooms = await api("/classrooms");
        await loadWorkspace();
      }
      dialog.close();
      modal = "";
    } catch (e) {
      fail(e);
    } finally {
      busy = false;
    }
  }
  async function removeTeacher(id) {
    busy = true;
    error = "";
    try {
      await api(`/classrooms/${classId}/teachers/${id}`, { method: "DELETE" });
      await loadWorkspace();
    } catch (e) {
      fail(e);
    } finally {
      busy = false;
    }
  }
  async function revokeInvitation(id) {
    busy = true;
    try {
      await api("/admin/invitations/" + id, { method: "DELETE" });
      invitations = await api("/admin/invitations");
    } catch (e) {
      fail(e);
    } finally {
      busy = false;
    }
  }
  function download(blob, filename) {
    const url = URL.createObjectURL(blob),
      a = document.createElement("a");
    a.href = url;
    a.download = filename;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 30000);
  }
  function filenamePart(name) {
    return name.replace(/[\\/:*?"<>|\x00-\x1f]/g, "-").trim() || "Mathematik";
  }
  async function exportZip() {
    if (!periodId || !students.length || !editable) return;
    busy = true;
    exportingZip = true;
    error = "";
    try {
      const r = await fetch(
        `/api/classrooms/${classId}/periods/${periodId}/zip`,
      );
      if (!r.ok) {
        const e = await r.json();
        throw new Error(e.error || "Der ZIP-Export ist fehlgeschlagen.");
      }
      download(
        await r.blob(),
        `Mathematik-${filenamePart(currentClass.name)}-${filenamePart(currentPeriod.name)}.zip`,
      );
    } catch (e) {
      fail(e);
    } finally {
      busy = false;
      exportingZip = false;
    }
  }
  async function exportPdf() {
    if (!currentStudent || !periodId || saving) return;
    busy = true;
    error = "";
    try {
      const r = await fetch(
        `/api/classrooms/${classId}/periods/${periodId}/students/${studentId}/pdf`,
      );
      if (!r.ok) {
        const e = await r.json();
        throw new Error(e.error || "Der Export ist fehlgeschlagen.");
      }
      download(
        await r.blob(),
        `Mathematik-${filenamePart(currentStudent.name)}-${filenamePart(currentPeriod.name)}.pdf`,
      );
    } catch (e) {
      fail(e);
    } finally {
      busy = false;
    }
  }
</script>

<svelte:window onkeydown={quickNote} />

{#if booting}
  <div class="boot">
    <div class="brand-mark">t<span>✳</span></div>
    <p>Tamara wird geladen …</p>
  </div>
{:else if !teacher}
  <main class="auth-layout">
    <section class="auth-story">
      <a class="wordmark" href="/"
        ><span class="brand-mark">t<span>✳</span></span>tamara<span
          class="brand-dot">.</span
        ></a
      >
      <div>
        <span class="eyebrow light">MEHR RAUM FÜRS UNTERRICHTEN</span>
        <h1>Jeden kleinen<br />Fortschritt<br /><em>im Blick.</em></h1>
        <p>
          Klassen organisieren. Entwicklung begleiten.<br />Gemeinsam mehr Zeit
          für die Kinder haben.
        </p>
        <div class="story-art" aria-hidden="true">
          <span>1</span><span>2</span><span>3</span><i>✓</i><b
            >Ein Schritt weiter.</b
          >
        </div>
      </div>
      <small>Dein Werkzeug für den Schulalltag.</small>
    </section>
    <section class="auth-form-area">
      <form onsubmit={authenticate} class="auth-form">
        <span class="eyebrow">WILLKOMMEN BEI TAMARA</span>
        <h2>
          {authMode === "login"
            ? "Schön, dass du da bist."
            : "Dein Unterricht. Dein Überblick."}
        </h2>
        <p>
          {authMode === "login"
            ? "Melde dich an und begleite die nächsten Schritte."
            : "Erstelle dein Lehrkraftkonto und lege deine erste Klasse an."}
        </p>
        {#if error}<div class="alert" role="alert">
            {error}
          </div>{/if}{#if authMode === "register"}<label
            >Dein Name<input
              bind:value={accountName}
              autocomplete="name"
              required
              maxlength="120"
              placeholder="Vor- und Nachname"
            /></label
          >{/if}<label
          >E-Mail-Adresse<input
            type="email"
            bind:value={email}
            autocomplete="email"
            required
            placeholder="name@schule.de"
          /></label
        >{#if authMode === "register"}<label
            >{setupRequired ? "Einrichtungscode" : "Registrierungscode"}<input
              bind:value={registrationToken}
              required
              autocomplete="off"
              placeholder={setupRequired
                ? "Code aus dem Server-Terminal"
                : "Code von deiner Administration"}
            /></label
          ><small class="muted"
            >{setupRequired
              ? "Mit dem einmaligen Einrichtungscode erstellst du das erste Administrationskonto."
              : "Für die Registrierung benötigst du einen gültigen Einladungscode."}</small
          >{/if}<label
          >Passwort<input
            type="password"
            bind:value={password}
            autocomplete={authMode === "login"
              ? "current-password"
              : "new-password"}
            required
            minlength={authMode === "register" ? 12 : 1}
            maxlength="256"
            placeholder={authMode === "register"
              ? "Mindestens 12 Zeichen"
              : "Dein Passwort"}
          /></label
        ><button class="primary auth-submit" disabled={busy}
          >{busy
            ? "Einen Moment …"
            : authMode === "login"
              ? "Anmelden →"
              : "Konto erstellen →"}</button
        >
        <p class="auth-toggle">
          {authMode === "login" ? "Noch kein Konto?" : "Schon registriert?"}
          <button
            type="button"
            class="text-button"
            onclick={() => {
              authMode = authMode === "login" ? "register" : "login";
              error = "";
            }}>{authMode === "login" ? "Konto erstellen" : "Anmelden"}</button
          >
        </p>
      </form>
    </section>
  </main>
{:else}
  <div class="app-shell">
    <aside class="sidebar">
      <a class="wordmark" href="/"
        ><span class="brand-mark">t<span>✳</span></span>tamara<span
          class="brand-dot">.</span
        ></a
      >
      <div class="class-switcher">
        <label class="sidebar-label" for="class-switcher">KLASSE</label>
        <select
          id="class-switcher"
          value={classId}
          disabled={saving || busy || loading || !classrooms.length}
          onchange={(e) => chooseClass(e.currentTarget.value)}
        >
          {#if !classrooms.length}<option value="">Noch keine Klasse</option
            >{/if}
          {#each classrooms as c (c.id)}<option value={c.id}
              >{c.name} · {c.school_year}{c.role === "editor"
                ? " · Geteilt"
                : ""}</option
            >{/each}
        </select>
        <button
          class="new-class"
          onclick={() => openModal("class")}
          disabled={saving || busy || loading}
          ><Icon name="plus" /> Klasse anlegen</button
        >
      </div>
      <div class="sidebar-label">ARBEITSBEREICHE</div>
      <nav
        aria-label="Arbeitsbereiche der ausgewählten Klasse"
        class="feature-nav"
      >
        {#each workAreas as area (area.id)}
          <a
            class="nav-item"
            class:active={feature === area.id}
            class:settings-nav={area.id === "settings"}
            href={workspaceUrl(classId, area.id, homeworkTab, homeworkSubject)}
            aria-current={feature === area.id ? "page" : undefined}
            aria-disabled={saving || busy || loading || !classId}
            onclick={(e) => navigate(area.id, e)}
          >
            <span aria-hidden="true">{area.icon}</span>{area.label}
          </a>
        {/each}
      </nav>
      {#if teacher.is_admin}
        <div class="sidebar-label">ADMINISTRATION</div>
        <button
          class="new-class admin-nav"
          onclick={() => openModal("invites")}
          disabled={saving || loading || busy}
          ><Icon name="key" /> Registrierungscodes</button
        >
        <button
          class="new-class admin-nav"
          onclick={() => openModal("backup")}
          disabled={saving || loading || busy}
          ><Icon name="backup" /> Datenbank sichern</button
        >
      {/if}
      <div class="sidebar-note">
        <span>✳</span>
        <p>Kleine Schritte.<br /><strong>Große Entwicklung.</strong></p>
      </div>
      <div class="profile">
        <div class="avatar">{initials(teacher.name)}</div>
        <div><strong>{teacher.name}</strong><small>Lehrkraft</small></div>
        <button
          class="icon-button"
          title="Abmelden"
          aria-label="Abmelden"
          onclick={signout}
          disabled={saving || busy}>↪</button
        >
      </div>
    </aside>
    <main class="main-area">
      <header class="topbar">
        <div>
          <span class="breadcrumb"
            >{currentClass?.name || "Dein Arbeitsplatz"}</span
          ><span class="crumb-separator">/</span><strong
            >{currentArea?.label || "Meine Klassen"}</strong
          >
        </div>
        <div class="topbar-right">
          <span class="year-pill"
            >◷ {currentClass?.school_year || "Schulalltag"}</span
          >
        </div>
      </header>
      <div class="page-content">
        {#if error && !modal}<div class="alert" role="alert">
            {error}<button
              class="text-button"
              onclick={() => (error = "")}
              aria-label="Fehlermeldung schließen">×</button
            >
          </div>{/if}
        {#if !classrooms.length}<div class="welcome-empty">
            <span class="large-flower">✳</span><span class="eyebrow"
              >DEIN ERSTER SCHRITT</span
            >
            <h1>Eine Klasse. Viele Möglichkeiten.</h1>
            <p>
              Lege deine Klasse an, füge die Kinder hinzu und<br />halte ihre
              Lernentwicklung an einem Ort fest.
            </p>
            <button class="primary" onclick={() => openModal("class")}
              ><Icon name="plus" /> Erste Klasse anlegen</button
            >
          </div>
        {:else if loading && ["homework", "settings", "students", "notes"].includes(feature)}
          <div class="empty-panel"><p>Klasse wird geladen …</p></div>
        {:else if feature === "homework"}
          {#key classId}<Homework
              {classId}
              {students}
              tab={homeworkTab}
              subject={homeworkSubject}
              bind:saving
              disabled={busy || !!modal}
              onerror={fail}
              onsettings={() => navigate("settings")}
              onfilters={homeworkFilters}
            />{/key}
        {:else if feature === "settings"}
          {#key classId}<ClassSettings
              {classId}
              classroom={currentClass}
              teachers={workspace?.teachers || []}
              bind:saving
              disabled={busy || !!modal}
              onerror={fail}
              onshare={() => openModal("share")}
            />{/key}
        {:else if feature === "notes"}
          {#key classId}<Notes
              bind:this={notesView}
              {classId}
              {students}
              bind:saving
              disabled={busy || !!modal}
              onerror={fail}
              onstudents={() => navigate("students")}
            />{/key}
        {:else if feature === "students"}
          <section class="page-heading with-actions">
            <div>
              <span class="eyebrow">DEINE KLASSE · ALLE KINDER</span>
              <h1>Kinder<span>.</span></h1>
              <p>
                {students.length} Kinder in {currentClass?.name}. Namen und
                Kinderliste zentral verwalten.
              </p>
            </div>
            <button
              class="primary"
              disabled={saving || busy}
              onclick={() => openModal("students")}
              ><Icon name="plus" /> Kinder hinzufügen</button
            >
          </section>
          <section
            class="workspace-card settings-panel"
            aria-label="Kinderliste"
          >
            <label class="pupil-search"
              >Kind suchen<input
                bind:value={rosterSearch}
                placeholder="Name …"
              /></label
            >
            <div class="class-roster">
              {#each visibleStudents as s (s.id)}<div class="collaborator">
                  <div class="pupil-identity">
                    <span class="avatar small-avatar">{initials(s.name)}</span>
                    <strong>{s.name}</strong>
                  </div>
                  <div class="inline-actions">
                    <button
                      class="text-button"
                      disabled={saving || busy}
                      onclick={async () => {
                        targetId = s.id;
                        await openModal("rename");
                        fieldName = s.name;
                      }}>Bearbeiten</button
                    >{#if workspace?.role === "owner"}<button
                        class="text-button danger-text"
                        disabled={saving || busy}
                        onclick={() => {
                          targetId = s.id;
                          openModal("delete");
                        }}>Entfernen</button
                      >{/if}
                  </div>
                </div>
              {:else}<div class="empty-panel">
                  <h3>
                    {students.length
                      ? "Kein passendes Kind"
                      : "Wer lernt in deiner Klasse?"}
                  </h3>
                  <p>
                    {students.length
                      ? "Versuche einen anderen Namen."
                      : "Füge Kinder einzeln oder als Namensliste hinzu."}
                  </p>
                </div>{/each}
            </div>
          </section>
        {:else if feature === "attendance"}
          <section class="page-heading">
            <div>
              <span class="eyebrow">DEINE KLASSE · JEDEN TAG IM BLICK</span>
              <h1>Wer ist heute da<span>?</span></h1>
              <p>
                Gehe die Klasse Kind für Kind durch oder bearbeite einen Tag im
                Kalender.
              </p>
            </div>
            <span class="heading-art" aria-hidden="true">✓</span>
          </section>
          {#if loading}
            <div class="empty-panel"><p>Klasse wird geladen …</p></div>
          {:else if !students.length}
            <div class="empty-panel">
              <h3>Wer lernt in deiner Klasse?</h3>
              <p>Füge zuerst die Kinder deiner Klasse hinzu.</p>
              <button class="primary" onclick={() => openModal("students")}
                ><Icon name="plus" /> Kinder hinzufügen</button
              >
            </div>
          {:else}
            {#key classId}
              <Attendance
                {classId}
                {students}
                className={currentClass?.name || ""}
                bind:saving
                disabled={busy || !!modal}
                onerror={fail}
              />
            {/key}
          {/if}
        {:else}
          <section class="page-heading">
            <div>
              <span class="eyebrow">LERNENTWICKLUNG · MATHEMATIK</span>
              <h1>Fortschritte sichtbar machen<span>.</span></h1>
              <p>
                Jedes Kind lernt in seinem Tempo. Hier behältst du den
                Überblick.
              </p>
            </div>
            <span class="heading-art" aria-hidden="true">✳</span>
          </section>
          <div class="overview-cards">
            <div class="overview-card">
              <span class="stat-icon green">♧</span>
              <div>
                <span class="stat-number">{students.length}</span><span
                  class="stat-label">Kinder in {currentClass?.name}</span
                >
              </div>
              <button
                class="text-button"
                onclick={() => openModal("students")}
                disabled={saving || loading}
                ><Icon name="plus" /> Hinzufügen</button
              >
            </div>
            <div class="overview-card">
              <span class="stat-icon peach">☷</span>
              <div>
                <span class="stat-number">{catalog.length}</span><span
                  class="stat-label">Kompetenzen · 10 Bereiche</span
                >
              </div>
            </div>
            <div class="overview-card progress-stat">
              <span class="stat-icon lavender">◴</span>
              <div>
                <span class="stat-number">{percentage}<small>%</small></span
                ><span class="stat-label">im aktuellen Zeitraum beurteilt</span>
              </div>
              <div class="mini-progress">
                <i style:width={`${percentage}%`}></i>
              </div>
            </div>
          </div>
          <section class="workspace-card" aria-busy={loading}>
            <div class="workspace-header">
              <div class="workspace-title">
                <span class="book-icon">▤</span>
                <div>
                  <h2>Entwicklungsübersicht</h2>
                  <p>Mathematik · Grundschule Bremen</p>
                </div>
              </div>
              <div class="period-controls">
                <button
                  class="secondary small export-all"
                  onclick={exportZip}
                  disabled={!editable || !students.length}
                  title="Ein PDF pro Kind für den ausgewählten Zeitraum herunterladen"
                  >{exportingZip
                    ? "ZIP wird erstellt …"
                    : "↓ Alle PDFs als ZIP"}</button
                >
                <label class="sr-only" for="period">Beurteilungszeitraum</label
                ><select
                  id="period"
                  value={periodId}
                  onchange={choosePeriod}
                  disabled={saving || loading || busy || !periods.length}
                  >{#if !periods.length}<option value=""
                      >Noch kein Zeitraum</option
                    >{/if}{#each periods as p}<option value={p.id}
                      >{p.name}</option
                    >{/each}</select
                ><button
                  class="icon-button outlined"
                  onclick={() => openModal("period")}
                  title="Zeitraum anlegen oder kopieren"
                  aria-label="Zeitraum anlegen oder kopieren"
                  disabled={saving || loading}><Icon name="plus" /></button
                >
              </div>
            </div>
            <div class="workspace-subheader">
              <div
                class="view-tabs"
                role="tablist"
                aria-label="Bearbeitungsansicht"
              >
                {#each modes as m}<button
                    role="tab"
                    aria-selected={mode === m.id}
                    class:active={mode === m.id}
                    onclick={() => {
                      mode = m.id;
                      selected = new Set();
                    }}
                    disabled={saving || loading}
                    ><span>{m.icon}</span>{m.label}</button
                  >{/each}
              </div>
              <div class="save-status" role="status">
                <span class:pending={saving} class="save-dot"></span>{saving
                  ? "Wird gespeichert …"
                  : notice || "Änderungen werden automatisch gespeichert"}
              </div>
            </div>
            {#if loading}<div class="empty-panel">
                <p>Klasse wird geladen …</p>
              </div>
            {:else if !periods.length}<div class="empty-panel">
                <span class="empty-icon">◷</span>
                <h3>Ein neuer Zeitraum, ein klarer Anfang.</h3>
                <p>
                  Lege einen benannten Beurteilungszeitraum an.<br />Später
                  kannst du mit einer Kopie daran anknüpfen.
                </p>
                <button class="primary" onclick={() => openModal("period")}
                  ><Icon name="plus" /> Zeitraum anlegen</button
                >
              </div>
            {:else if !students.length}<div class="empty-panel">
                <span class="empty-icon">♧</span>
                <h3>Wer lernt in deiner Klasse?</h3>
                <p>Füge die Kinder einzeln oder als Namensliste hinzu.</p>
                <button class="primary" onclick={() => openModal("students")}
                  ><Icon name="plus" /> Kinder hinzufügen</button
                >
              </div>
            {:else}
              <div class="filters">
                <label class="filter-section"
                  ><span>Kompetenzbereich</span><select
                    bind:value={section}
                    onchange={() => resetFilters(false)}
                    >{#each domains as domain}<optgroup label={domain}>
                        {#each sections.filter((sec) => catalog.find((c) => c.section === sec)?.domain === domain) as sec}
                          <option value={sec}>{sec}</option>
                        {/each}
                      </optgroup>{/each}</select
                  ></label
                ><label class="filter-grade">
                  <span>Jahrgang · nach PDF S. 3</span>
                  <select
                    bind:value={schoolGrade}
                    onchange={() => {
                      level = "all";
                      matrixPage = 0;
                      selected = new Set();
                    }}
                  >
                    <option value="all">Alle Jahrgänge · Basis–10</option>
                    {#each gradeRanges as g}
                      <option value={g.id}
                        >Jahrgang {g.id} · {String(g.first).padStart(
                          2,
                          "0",
                        )}–{String(g.last).padStart(2, "0")}</option
                      >
                    {/each}
                  </select>
                </label><label class="filter-level"
                  ><span>Entwicklungsstufe</span><select
                    bind:value={level}
                    onchange={() => (matrixPage = 0)}
                    ><option value="all">Alle Stufen</option
                    >{#each relevantLevels as l}<option value={l}
                        >{l === "Basis" ? "Basis" : "Stufe " + l}</option
                      >{/each}</select
                  ></label
                ><label class="search-input"
                  ><span class="sr-only">Kompetenzen durchsuchen</span><span
                    aria-hidden="true">⌕</span
                  ><input
                    placeholder="Kompetenz suchen …"
                    bind:value={search}
                    oninput={() => (matrixPage = 0)}
                  /></label
                >
              </div>
              <div class="pdf-topic" aria-label="Gliederung der PDF-Vorlage">
                <div class="pdf-domain">{sectionInfo?.domain}</div>
                <div class="pdf-section">{section}</div>
                <div class="pdf-subtitle">{sectionInfo?.subtitle}</div>
              </div>
              <div class="pdf-key">
                <span
                  ><i class="band-swatch light"></i><i class="band-swatch dark"
                  ></i>Grau: Zielvorgaben (Regelstandard), wie im PDF</span
                >
                {#if gradeRange}<span class="grade-range-note"
                    >Jahrgang {schoolGrade}: Stufen {String(
                      gradeRange.first,
                    ).padStart(2, "0")}–{String(gradeRange.last).padStart(
                      2,
                      "0",
                    )} · Zielstufe {gradeRange.target}{schoolGrade === "4"
                      ? " (Halbjahr: 08)"
                      : ""}</span
                  >{/if}
                <span
                  >Symbole und Reihenfolge wie in der Vorlage · {filtered.length}
                  Kompetenzen</span
                >
              </div>
              {#if mode === "student"}
                <div class="student-workspace">
                  <aside class="student-list">
                    <div class="roster-heading">
                      <strong>DEINE KLASSE</strong><span>{students.length}</span
                      ><button
                        class="icon-button"
                        title="Kinder verwalten"
                        aria-label="Kinder verwalten"
                        onclick={() => openModal("manage")}
                        disabled={saving}>⚙</button
                      >
                    </div>
                    <label class="roster-search"
                      ><span class="sr-only">Kinder suchen</span><input
                        bind:value={rosterSearch}
                        placeholder="Kind suchen …"
                      /></label
                    >
                    <div class="roster-items">
                      {#each visibleStudents as s, i}<button
                          class="student-item"
                          class:active={studentId === s.id}
                          onclick={() => (studentId = s.id)}
                          disabled={saving}
                          ><span
                            class="avatar"
                            class:peach={i % 3 === 1}
                            class:lavender={i % 3 === 2}
                            >{initials(s.name)}</span
                          ><span
                            ><strong>{s.name}</strong><small
                              >{countFor(s.id)} von {catalog.length} beurteilt</small
                            ></span
                          ><span class="student-chevron">›</span></button
                        >{/each}{#if !visibleStudents.length}<p
                          class="muted inset"
                        >
                          Keine Kinder gefunden.
                        </p>{/if}
                    </div>
                  </aside>
                  <div class="assessment-panel">
                    <div class="student-heading">
                      <div class="student-identity">
                        <span class="avatar big"
                          >{initials(currentStudent?.name || "")}</span
                        >
                        <div>
                          <h3>{currentStudent?.name}</h3>
                          <p>
                            <span class="green-text"
                              >{countFor(studentId, "yes")} erreicht</span
                            >
                            · {countFor(studentId, "no")} noch nicht · {catalog.length -
                              countFor(studentId)} offen
                          </p>
                        </div>
                      </div>
                      <button
                        class="secondary small"
                        onclick={exportPdf}
                        disabled={saving || busy}>↓ PDF exportieren</button
                      >
                    </div>
                    <div class="criteria-scroll">
                      {#each grouped as group}
                        <section
                          class="pdf-level"
                          data-band={group.band}
                          aria-label={group.level === "Basis"
                            ? "Basis"
                            : "Stufe " + group.level}
                        >
                          <div class="level-heading">
                            <span class="level-badge"
                              >{group.level === "Basis"
                                ? "BASIS"
                                : "STUFE " + group.level}</span
                            >
                            {#if group.band !== "white"}<strong
                                >Zielvorgaben (Regelstandard)</strong
                              >{/if}
                            <span class="level-count"
                              >{group.criteria.length} Kompetenzen · PDF S. {group
                                .criteria[0].page}</span
                            >
                          </div>
                          {#each group.criteria as c}
                            <div class="criterion-row">
                              <span
                                class="strand-symbol"
                                title="Kompetenzstrang aus der PDF-Vorlage"
                                aria-label={"Kompetenzstrang " + c.symbol}
                                >{c.symbol}</span
                              >
                              <div class="criterion-copy"><p>{c.label}</p></div>
                              <Status
                                value={value(studentId, c.id)}
                                achievedOn={achievedOn(studentId, c.id)}
                                ondate={(date) =>
                                  grade(studentId, c.id, "yes", date)}
                                onchoose={(v) => grade(studentId, c.id, v)}
                                disabled={!editable}
                                label={`${currentStudent?.name}: ${c.label}`}
                              />
                            </div>
                          {/each}
                        </section>
                      {/each}{#if !filtered.length}<div class="empty-panel">
                          <h3>Keine passende Kompetenz</h3>
                          <p>Ändere die Suche oder die Entwicklungsstufe.</p>
                          <button class="secondary" onclick={resetFilters}
                            >Filter zurücksetzen</button
                          >
                        </div>{/if}
                    </div>
                  </div>
                </div>
              {:else if mode === "class"}
                <div class="matrix-heading">
                  <div>
                    <strong>{section}</strong>
                    <p>Wähle die Bewertung direkt in der Tabelle.</p>
                  </div>
                  <button
                    class="secondary small"
                    onclick={() => openModal("manage")}
                    disabled={saving}>Kinder verwalten</button
                  >
                </div>
                <div class="matrix-scroll">
                  <table>
                    <thead
                      ><tr
                        ><th class="sticky-col"
                          >Kind<small>{students.length} Kinder</small></th
                        >{#each matrixCriteria as c}<th data-band={c.band}
                            ><span class="level-badge small-badge"
                              >{c.level === "Basis"
                                ? "BASIS"
                                : "STUFE " + c.level}</span
                            >
                            <p>
                              <span
                                class="strand-symbol"
                                aria-label={"Kompetenzstrang " + c.symbol}
                                >{c.symbol}</span
                              >
                              {c.label}
                            </p>
                            <small
                              >PDF S. {c.page}{c.band !== "white"
                                ? " · Zielvorgaben"
                                : ""}</small
                            ></th
                          >{/each}</tr
                      ></thead
                    ><tbody
                      >{#each students as s}<tr
                          ><th class="sticky-col"
                            ><button
                              class="table-student"
                              onclick={() => {
                                studentId = s.id;
                                mode = "student";
                              }}
                              ><span class="avatar small-avatar"
                                >{initials(s.name)}</span
                              ><strong>{s.name}</strong></button
                            ></th
                          >{#each matrixCriteria as c}<td data-band={c.band}
                              ><Status
                                compact
                                value={value(s.id, c.id)}
                                achievedOn={achievedOn(s.id, c.id)}
                                ondate={(date) =>
                                  grade(s.id, c.id, "yes", date)}
                                onchoose={(v) => grade(s.id, c.id, v)}
                                disabled={!editable}
                                label={`${s.name}: ${c.label}`}
                              /></td
                            >{/each}</tr
                        >{/each}</tbody
                    >
                  </table>
                  {#if !filtered.length}<p class="inset muted">
                      Keine passenden Kompetenzen. Passe die Filter an.
                    </p>{/if}
                </div>
                <div class="pagination">
                  <span
                    >{filtered.length ? matrixPage * 8 + 1 : 0}–{Math.min(
                      (matrixPage + 1) * 8,
                      filtered.length,
                    )} von {filtered.length} Kompetenzen</span
                  >
                  <div>
                    <button
                      class="secondary small"
                      disabled={matrixPage === 0 || saving}
                      onclick={() => matrixPage--}>← Zurück</button
                    ><span>{matrixPage + 1} / {matrixPages}</span><button
                      class="secondary small"
                      disabled={matrixPage >= matrixPages - 1 || saving}
                      onclick={() => matrixPage++}>Weiter →</button
                    >
                  </div>
                </div>
              {:else}
                <div class="criterion-workspace">
                  <aside class="criterion-list">
                    <div class="roster-heading">
                      <strong>KOMPETENZEN</strong><span>{filtered.length}</span>
                    </div>
                    {#each grouped as group}
                      <section
                        class="pdf-level"
                        data-band={group.band}
                        aria-label={group.level === "Basis"
                          ? "Basis"
                          : "Stufe " + group.level}
                      >
                        <div class="level-heading">
                          <span class="level-badge"
                            >{group.level === "Basis"
                              ? "BASIS"
                              : "STUFE " + group.level}</span
                          >
                          {#if group.band !== "white"}<span>Zielvorgaben</span
                            >{/if}
                        </div>
                        {#each group.criteria as c}
                          <button
                            class="criterion-pick"
                            class:active={currentCriterion?.id === c.id}
                            aria-pressed={currentCriterion?.id === c.id}
                            onclick={() => {
                              criterionId = c.id;
                              selected = new Set();
                            }}
                            disabled={saving}
                          >
                            <span
                              class="strand-symbol"
                              aria-label={"Kompetenzstrang " + c.symbol}
                              >{c.symbol}</span
                            >
                            <p>{c.label}</p>
                          </button>
                        {/each}
                      </section>
                    {/each}
                  </aside>
                  <div class="multiselect-panel">
                    {#if currentCriterion}
                      <div
                        class="criterion-context"
                        data-band={currentCriterion.band}
                      >
                        <span
                          class="strand-symbol"
                          aria-label={"Kompetenzstrang " +
                            currentCriterion.symbol}
                          >{currentCriterion.symbol}</span
                        >
                        <span
                          >{currentCriterion.level === "Basis"
                            ? "Basis"
                            : "Stufe " + currentCriterion.level} · PDF S. {currentCriterion.page}</span
                        >
                        {#if currentCriterion.band !== "white"}<strong
                            >Zielvorgaben (Regelstandard)</strong
                          >{/if}
                      </div>
                      <span class="eyebrow"
                        >{currentCriterion.domain} · {currentCriterion.level ===
                        "Basis"
                          ? "Basis"
                          : "Stufe " + currentCriterion.level}</span
                      >
                      <h3>{currentCriterion.label}</h3>
                      <p class="muted">
                        Wähle Kinder aus und beurteile sie gemeinsam.
                      </p>
                      <div class="bulk-controls">
                        <span>{selected.size} ausgewählt</span><button
                          class="text-button"
                          onclick={() =>
                            (selected = new Set(
                              visibleStudents.map((s) => s.id),
                            ))}
                          disabled={saving}>Alle auswählen</button
                        ><button
                          class="text-button"
                          onclick={() => (selected = new Set())}
                          disabled={saving}>Auswahl aufheben</button
                        >
                      </div>
                      <div class="bulk-actions">
                        <button
                          class="primary small"
                          onclick={() => bulk("yes")}
                          disabled={!selected.size || !editable}
                          >✓ Erreicht</button
                        ><button
                          class="secondary small"
                          onclick={() => bulk("no")}
                          disabled={!selected.size || !editable}
                          >× Noch nicht</button
                        ><button
                          class="secondary small"
                          onclick={() => bulk(null)}
                          disabled={!selected.size || !editable}
                          >– Nicht beurteilt</button
                        >
                      </div>
                      <label class="roster-search"
                        ><span class="sr-only"
                          >Kinder für Mehrfachauswahl suchen</span
                        ><input
                          bind:value={rosterSearch}
                          placeholder="Kind suchen …"
                        /></label
                      >
                      <div class="multiselect-roster">
                        {#each visibleStudents as s}<div
                            class="multiselect-row"
                          >
                            <label
                              ><input
                                type="checkbox"
                                checked={selected.has(s.id)}
                                onchange={() => toggleSelected(s.id)}
                                disabled={saving}
                              /><span class="avatar small-avatar"
                                >{initials(s.name)}</span
                              ><strong>{s.name}</strong></label
                            ><Status
                              compact
                              value={value(s.id, currentCriterion.id)}
                              achievedOn={achievedOn(s.id, currentCriterion.id)}
                              ondate={(date) =>
                                grade(s.id, currentCriterion.id, "yes", date)}
                              onchoose={(v) =>
                                grade(s.id, currentCriterion.id, v)}
                              disabled={!editable}
                              label={`${s.name}: ${currentCriterion.label}`}
                            />
                          </div>{/each}
                      </div>{:else}<div class="empty-panel">
                        <h3>Keine passende Kompetenz</h3>
                        <button class="secondary" onclick={resetFilters}
                          >Filter zurücksetzen</button
                        >
                      </div>{/if}
                  </div>
                </div>
              {/if}
              <footer class="workspace-footer">
                <div>
                  <span><i class="legend-dot yes-dot"></i>Erreicht</span><span
                    ><i class="legend-dot no-dot"></i>Noch nicht</span
                  ><span><i class="legend-dot"></i>Nicht beurteilt</span>
                </div>
                <span
                  >✓ Erreicht-Daten werden gespeichert. Im PDF erscheinen die
                  Kreuze.</span
                >
              </footer>
            {/if}
          </section>
          <div class="page-footer">
            <span>Für die kleinen und großen Aha-Momente.</span><span
              >tamara<span class="green-text">.</span></span
            >
          </div>
        {/if}
      </div>
    </main>
  </div>
{/if}

{#if modal}
  <dialog
    aria-labelledby="modal-title"
    bind:this={dialog}
    oncancel={(e) => {
      if (busy) e.preventDefault();
      else modal = "";
    }}
    onclose={() => {
      if (!busy) modal = "";
    }}
  >
    <form onsubmit={submitModal} class="modal-content">
      <div class="modal-top">
        <span class="eyebrow">{currentClass?.name || "DEIN ARBEITSPLATZ"}</span
        ><button
          type="button"
          class="icon-button"
          onclick={closeModal}
          disabled={busy}
          aria-label="Dialog schließen">×</button
        >
      </div>
      <h2 id="modal-title">
        {{
          class: "Eine neue Klasse",
          students: "Kinder hinzufügen",
          period: "Neuer Beurteilungszeitraum",
          share: "Gemeinsam an einer Klasse arbeiten",
          manage: "Kinder verwalten",
          rename: "Name bearbeiten",
          delete: "Kind entfernen?",
          invites: "Registrierungscodes",
          backup: "Datenbank sichern",
        }[modal]}
      </h2>
      {#if error}<div class="alert" role="alert">{error}</div>{/if}
      {#if modal === "backup"}
        <p>
          Erstelle eine Sicherung der gesamten Datenbank. Das ZIP-Archiv wird
          privat auf dem Server neben der Datenbank gespeichert. Manuelle
          Sicherungen bleiben erhalten.
        </p>
        <p>
          Automatische Sicherungen werden jede Nacht um 02:00 UTC erstellt.
          Davon werden höchstens sieben Sicherungen für bis zu eine Woche
          aufbewahrt.
        </p>
        {#if backupCreated}
          <div class="info-box" role="status">
            Die Datenbanksicherung wurde erfolgreich auf dem Server erstellt.
          </div>
        {/if}
      {:else if modal === "invites"}<p>
          Erstelle einen einmaligen Code für eine neue Lehrkraft. Codes sind
          sieben Tage gültig. Der vollständige Code wird nur direkt nach dem
          Erstellen angezeigt.
        </p>
        <label
          >Bezeichnung<input
            bind:value={fieldName}
            required
            maxlength="120"
            placeholder="z. B. Neue Kollegin"
          /></label
        >{#if invitationToken}<div class="info-box">
            Kopiere diesen Code und gib ihn an die eingeladene Lehrkraft weiter.
          </div>
          <div class="invitation-token" aria-label="Neuer Registrierungscode">
            {invitationToken}
          </div>{/if}
        <div class="invite-list">
          {#each invitations as inv}<div class="collaborator">
              <div>
                <strong>{inv.label}</strong><small
                  >{inv.used_at
                    ? "Verwendet"
                    : inv.revoked
                      ? "Widerrufen"
                      : inv.expires_at * 1000 < Date.now()
                        ? "Abgelaufen"
                        : "Gültig bis " +
                          new Date(inv.expires_at * 1000).toLocaleDateString(
                            "de-DE",
                          )}</small
                >
              </div>
              {#if !inv.used_at && !inv.revoked && inv.expires_at * 1000 > Date.now()}<button
                  type="button"
                  class="text-button danger-text"
                  onclick={() => revokeInvitation(inv.id)}
                  disabled={busy}>Widerrufen</button
                >{/if}
            </div>{/each}
        </div>
      {:else if modal === "class"}<p>
          Ein gemeinsamer Ort für deine Klasse und ihre Lernentwicklung.
        </p>
        <label
          >Klassenname<input
            bind:value={fieldName}
            required
            maxlength="80"
            placeholder="z. B. Klasse 2a"
          /></label
        ><label
          >Schuljahr<input
            bind:value={fieldYear}
            required
            maxlength="40"
            placeholder="2026/27"
          /></label
        >
      {:else if modal === "students"}<p>
          Ein Kind pro Zeile. Du kannst eine ganze Namensliste einfügen.
        </p>
        <label
          >Namen<textarea
            bind:value={fieldNames}
            required
            rows="8"
            placeholder={"Mia Müller\nJonas Schmidt\nAmira Yılmaz"}
          ></textarea></label
        >
      {:else if modal === "period"}<p>
          Beginne leer oder übernimm die Bewertungen eines bestehenden
          Zeitraums. Die Kopie lässt sich unabhängig bearbeiten.
        </p>
        <label
          >Name des Zeitraums<input
            bind:value={fieldName}
            required
            maxlength="120"
            placeholder="z. B. Herbst 2026"
          /></label
        ><label
          >Ausgangsdaten<select bind:value={fieldSource}
            ><option value="">Leer beginnen · alles nicht beurteilt</option
            >{#each periods as p}<option value={p.id}>Kopie von {p.name}</option
              >{/each}</select
          ></label
        >
        <div class="info-box">
          {fieldSource
            ? "„Erreicht“ und „Noch nicht“ werden kopiert. Offene Kompetenzen bleiben offen. Der ursprüngliche Zeitraum bleibt erhalten."
            : "Alle Kompetenzen beginnen mit „Nicht beurteilt“."}
        </div>
      {:else if modal === "share"}<p>
          Freigegebene Lehrkräfte können Kinder, Zeiträume, Bewertungen,
          Anwesenheit, Fächer und Hausaufgaben gemeinsam bearbeiten. Die
          Klassenleitung verwaltet den Zugriff.
        </p>
        <div class="collaborators">
          {#each workspace?.teachers || [] as t}<div class="collaborator">
              <span class="avatar small-avatar">{initials(t.name)}</span>
              <div><strong>{t.name}</strong><small>{t.email}</small></div>
              {#if t.role === "owner"}<span class="role-tag"
                  >Klassenleitung</span
                >{:else if workspace?.role === "owner"}<button
                  type="button"
                  class="text-button danger-text"
                  onclick={() => removeTeacher(t.id)}
                  disabled={busy}>Entfernen</button
                >{:else}<span class="role-tag">Lehrkraft</span>{/if}
            </div>{/each}
        </div>
        {#if workspace?.role === "owner"}<label
            >Registrierte Lehrkraft hinzufügen<input
              type="email"
              bind:value={fieldEmail}
              required
              placeholder="kollegin@schule.de"
            /></label
          ><small class="muted"
            >Die Lehrkraft muss bereits ein eigenes Tamara-Konto haben.</small
          >{/if}
      {:else if modal === "manage"}<p>
          Namensänderungen gelten für alle Zeiträume dieser Klasse.
        </p>
        <div class="manage-roster">
          {#each students as s}<div class="collaborator">
              <span class="avatar small-avatar">{initials(s.name)}</span><strong
                >{s.name}</strong
              ><button
                type="button"
                class="text-button"
                onclick={() => {
                  targetId = s.id;
                  modal = "rename";
                  fieldName = s.name;
                }}>Bearbeiten</button
              >{#if workspace?.role === "owner"}<button
                  type="button"
                  class="text-button danger-text"
                  onclick={() => {
                    targetId = s.id;
                    modal = "delete";
                  }}>Entfernen</button
                >{/if}
            </div>{/each}
        </div>
      {:else if modal === "rename"}<label
          >Name<input bind:value={fieldName} required maxlength="120" /></label
        >
      {:else if modal === "delete"}<p>
          <strong>{students.find((s) => s.id === targetId)?.name}</strong> wird mit
          allen Bewertungen, Notizen, Anwesenheitseinträgen und Hausaufgaben-Abgaben
          dieser Klasse gelöscht. Das kann nicht rückgängig gemacht werden.
        </p>{/if}
      <div class="modal-actions">
        <button
          type="button"
          class="secondary"
          onclick={closeModal}
          disabled={busy}
          >{["manage", "share", "invites", "backup"].includes(modal)
            ? "Schließen"
            : "Abbrechen"}</button
        >{#if modal !== "manage" && !(modal === "backup" && backupCreated) && (modal !== "share" || workspace?.role === "owner")}<button
            class:danger={modal === "delete"}
            class="primary"
            disabled={busy}
            >{modal === "backup"
              ? busy
                ? "Sicherung wird erstellt …"
                : "Sicherung erstellen"
              : busy
                ? "Wird gespeichert …"
                : modal === "invites"
                  ? "Code erstellen"
                  : modal === "share"
                    ? "Zugriff freigeben"
                    : modal === "delete"
                      ? "Kind endgültig entfernen"
                      : modal === "period"
                        ? "Zeitraum anlegen"
                        : "Speichern"}</button
          >{/if}
      </div>
    </form>
  </dialog>
{/if}
