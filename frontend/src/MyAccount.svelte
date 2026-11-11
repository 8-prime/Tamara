<script lang="ts">
  import { onMount, tick } from "svelte";
  import { api, errorMessage } from "./api.ts";
  import { AccountDeletionSchema, TeacherSchema } from "./models.ts";
  import type { AccountDeletion, Teacher } from "./models.ts";
  import { initials } from "./accounts.ts";
  import AccountDeletionForm from "./AccountDeletionForm.svelte";
  interface Props {
    teacher: Teacher;
    onchange: (teacher: Teacher) => void;
    onclose: () => void;
    onremoved: () => void;
  }
  let { teacher, onchange, onclose, onremoved }: Props = $props();
  let dialog: HTMLDialogElement;
  let editing = $state<"" | "email" | "password">(""),
    busy = $state(false),
    error = $state(""),
    notice = $state("");
  let newEmail = $state(""),
    emailPassword = $state(""),
    currentPassword = $state(""),
    newPassword = $state(""),
    repeatPassword = $state("");
  let deletion = $state<AccountDeletion | null>(null);
  onMount(() => dialog.showModal());

  async function edit(section: typeof editing) {
    editing = section;
    error = "";
    notice = "";
    newEmail = section === "email" ? teacher.email : "";
    emailPassword = currentPassword = newPassword = repeatPassword = "";
    await tick();
    dialog.querySelector<HTMLInputElement>(".setting-form input")?.focus();
  }
  async function saveEmail(event: SubmitEvent) {
    event.preventDefault();
    busy = true;
    error = "";
    try {
      const updated = await api(TeacherSchema, "/auth/me/email", {
        method: "PUT",
        body: JSON.stringify({ email: newEmail, password: emailPassword }),
      });
      onchange(updated);
      editing = "";
      emailPassword = "";
      notice = `Deine E-Mail-Adresse lautet jetzt ${updated.email}. Melde dich künftig damit an.`;
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
  async function savePassword(event: SubmitEvent) {
    event.preventDefault();
    error = "";
    if (newPassword !== repeatPassword) {
      error = "Die beiden neuen Passwörter stimmen nicht überein.";
      return;
    }
    busy = true;
    try {
      await api("/auth/me/password", {
        method: "PUT",
        body: JSON.stringify({ current_password: currentPassword, new_password: newPassword }),
      });
      editing = "";
      currentPassword = newPassword = repeatPassword = "";
      notice = "Dein Passwort wurde geändert. Auf anderen Geräten musst du dich neu anmelden.";
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
  async function previewDeletion() {
    busy = true;
    error = "";
    notice = "";
    try {
      deletion = await api(AccountDeletionSchema, "/auth/me/deletion");
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
</script>

<dialog
  bind:this={dialog}
  aria-labelledby="my-account-title"
  oncancel={(e) => {
    if (busy) e.preventDefault();
  }}
  onclose={onclose}
>
  <div class="modal-content">
    <div class="modal-top">
      <span class="eyebrow">MEIN KONTO</span><button
        class="icon-button"
        disabled={busy}
        onclick={() => dialog.close()}
        aria-label="Schließen">×</button
      >
    </div>
    {#if deletion}
      <h2 id="my-account-title">Konto endgültig löschen?</h2>
      <AccountDeletionForm {deletion} own bind:busy onback={() => (deletion = null)} ondeleted={onremoved} />
    {:else}
      <div class="identity">
        <span class="profile-avatar" aria-hidden="true">{initials(teacher.name)}</span>
        <div>
          <h2 id="my-account-title">{teacher.name}</h2>
          <span class="muted">{teacher.is_admin ? "Administration" : "Lehrkraft"}</span>
        </div>
      </div>
      {#if notice}<div class="info-box" role="status">{notice}</div>{/if}
      {#if error}<div class="alert" role="alert">{error}</div>{/if}

      <section class="setting" aria-labelledby="email-setting">
        <div class="setting-summary">
          <div>
            <strong id="email-setting">E-Mail-Adresse</strong>
            <span>{teacher.email}</span>
            <small class="muted">Damit meldest du dich an.</small>
          </div>
          {#if editing !== "email"}<button class="secondary" disabled={busy} onclick={() => edit("email")}>Ändern</button>{/if}
        </div>
        {#if editing === "email"}
          <form class="setting-form" onsubmit={saveEmail}>
            <label
              >Neue E-Mail-Adresse<input
                type="email"
                bind:value={newEmail}
                autocomplete="email"
                maxlength="254"
                required
              /></label
            >
            <label
              >Aktuelles Passwort zur Bestätigung<input
                type="password"
                bind:value={emailPassword}
                autocomplete="current-password"
                maxlength="256"
                required
              /></label
            >
            <div class="form-actions">
              <button type="button" class="secondary" disabled={busy} onclick={() => edit("")}>Abbrechen</button><button
                class="primary"
                disabled={busy}>E-Mail-Adresse speichern</button
              >
            </div>
          </form>
        {/if}
      </section>

      <section class="setting" aria-labelledby="password-setting">
        <div class="setting-summary">
          <div>
            <strong id="password-setting">Passwort</strong>
            <span aria-hidden="true">••••••••••••</span>
            <small class="muted">Beim Ändern werden andere Geräte abgemeldet.</small>
          </div>
          {#if editing !== "password"}<button class="secondary" disabled={busy} onclick={() => edit("password")}
              >Ändern</button
            >{/if}
        </div>
        {#if editing === "password"}
          <form class="setting-form" onsubmit={savePassword}>
            <!-- Lets password managers associate the new password with this account. -->
            <input class="sr-only" type="email" value={teacher.email} autocomplete="username" readonly tabindex="-1" aria-hidden="true" />
            <label
              >Aktuelles Passwort<input
                type="password"
                bind:value={currentPassword}
                autocomplete="current-password"
                maxlength="256"
                required
              /></label
            >
            <label
              >Neues Passwort <span class="muted">mindestens 12 Zeichen</span><input
                type="password"
                bind:value={newPassword}
                autocomplete="new-password"
                minlength="12"
                maxlength="256"
                required
              /></label
            >
            <label
              >Neues Passwort wiederholen<input
                type="password"
                bind:value={repeatPassword}
                autocomplete="new-password"
                minlength="12"
                maxlength="256"
                required
              /></label
            >
            <div class="form-actions">
              <button type="button" class="secondary" disabled={busy} onclick={() => edit("")}>Abbrechen</button><button
                class="primary"
                disabled={busy}>Passwort ändern</button
              >
            </div>
          </form>
        {/if}
      </section>

      <section class="setting danger-zone" aria-labelledby="delete-setting">
        <div class="setting-summary">
          <div>
            <strong id="delete-setting">Konto löschen</strong>
            <small class="muted">Entfernt dein Konto und deine eigenen Klassen dauerhaft.</small>
          </div>
          <button class="secondary danger-text" disabled={busy} onclick={previewDeletion}>Konto löschen …</button>
        </div>
      </section>

      <div class="modal-actions">
        <button class="secondary" disabled={busy} onclick={() => dialog.close()}>Schließen</button>
      </div>
    {/if}
  </div>
</dialog>

<style>
  dialog {
    width: min(560px, calc(100vw - 32px));
  }
  .identity {
    display: flex;
    align-items: center;
    gap: 14px;
    margin: 6px 0 20px;
  }
  .identity h2 {
    line-height: 1.2;
  }
  .setting {
    padding: 16px 0;
    border-top: 1px solid var(--line);
  }
  .setting-summary {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  .setting-summary > div {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .setting-summary small {
    font-size: 12px;
  }
  .setting-form {
    margin-top: 4px;
  }
  .setting-form label {
    margin-top: 12px;
  }
  .form-actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 16px;
  }
  .danger-zone .secondary {
    flex: none;
  }
  .info-box,
  .alert {
    margin-bottom: 12px;
  }
</style>
