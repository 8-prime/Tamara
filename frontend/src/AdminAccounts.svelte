<script lang="ts">
  import { onMount, tick } from "svelte";
  import { api, errorMessage } from "./api.ts";
  import { AccountDeletionSchema, AccountSummarySchema, PasswordResetResultSchema, TeacherSchema } from "./models.ts";
  import type { AccountDeletion, AccountSummary, Teacher } from "./models.ts";
  import { filterAccounts, initials, type AccountRoleFilter, type AccountSort } from "./accounts.ts";
  import AccountDeletionForm from "./AccountDeletionForm.svelte";
  import DropdownMenu, { type MenuItem } from "./DropdownMenu.svelte";
  import Icon from "./Icon.svelte";
  interface Props {
    teacher: Teacher;
    /** Opens the signed-in administrator's own account settings. */
    onownaccount: () => void;
    oninvite: () => void;
    onrolechange: () => Promise<void>;
  }
  let { teacher, onownaccount, oninvite, onrolechange }: Props = $props();
  let accounts = $state<AccountSummary[]>([]),
    loading = $state(true),
    busy = $state(false),
    error = $state(""),
    notice = $state("");
  let query = $state(""),
    role = $state<AccountRoleFilter>("all"),
    sort = $state<AccountSort>("name");
  let reset = $state<{ name: string; link: string; expires: number } | null>(null),
    copied = $state(false);
  let emailDialog: HTMLDialogElement,
    emailTarget = $state<AccountSummary | null>(null),
    newEmail = $state(""),
    emailError = $state("");
  let deletionDialog: HTMLDialogElement, deletion = $state<AccountDeletion | null>(null);
  const visible = $derived(filterAccounts(accounts, query, role, sort));
  const adminCount = $derived(accounts.filter((a) => a.is_admin).length);

  async function refresh() {
    accounts = await api(AccountSummarySchema.array(), "/admin/accounts");
  }
  onMount(() => {
    refresh()
      .catch((e) => (error = errorMessage(e)))
      .finally(() => (loading = false));
  });
  async function run(action: () => Promise<void>) {
    busy = true;
    error = "";
    notice = "";
    try {
      await action();
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
  function actions(account: AccountSummary): MenuItem[] {
    const roleItem = {
      label: account.is_admin ? "Adminrechte entziehen" : "Zur Administration ernennen",
      onselect: () => toggleRole(account),
    };
    if (account.id === teacher.id)
      return [{ label: "Mein Konto bearbeiten", onselect: onownaccount }, roleItem];
    return [
      { label: "E-Mail-Adresse ändern", onselect: () => openEmail(account) },
      { label: "Passwort-Rücksetzlink erstellen", onselect: () => issueReset(account) },
      roleItem,
      { label: "Konto löschen …", onselect: () => previewDeletion(account), danger: true, separated: true },
    ];
  }
  function issueReset(account: AccountSummary) {
    return run(async () => {
      reset = null;
      copied = false;
      const result = await api(PasswordResetResultSchema, `/admin/accounts/${account.id}/password-reset`, {
        method: "POST",
      });
      const link = new URL("/", window.location.origin);
      link.hash = `reset-password=${result.token}`;
      reset = { name: account.name, link: link.href, expires: result.expires_at };
    });
  }
  async function copyReset() {
    if (!reset) return;
    try {
      await navigator.clipboard.writeText(reset.link);
      copied = true;
    } catch {
      error = "Kopieren nicht möglich. Bitte den Link markieren und manuell kopieren.";
    }
  }
  function toggleRole(account: AccountSummary) {
    const own = account.id === teacher.id;
    const question = account.is_admin
      ? own
        ? "Deine eigenen Administrationsrechte entziehen? Du kannst danach keine Konten mehr verwalten."
        : `${account.name}: Administrationsrechte entziehen?`
      : `${account.name} zur Administration ernennen?`;
    if (!confirm(question)) return;
    return run(async () => {
      await api(`/admin/accounts/${account.id}`, {
        method: "PATCH",
        body: JSON.stringify({ is_admin: !account.is_admin }),
      });
      if (own) {
        await onrolechange();
        return;
      }
      await refresh();
      notice = `${account.name} ist jetzt ${account.is_admin ? "Lehrkraft ohne Adminrechte" : "Administration"}.`;
    });
  }
  async function openEmail(account: AccountSummary) {
    emailTarget = account;
    newEmail = account.email;
    error = "";
    notice = "";
    await tick();
    emailDialog.showModal();
  }
  async function saveEmail(event: SubmitEvent) {
    event.preventDefault();
    const target = emailTarget;
    if (!target) return;
    busy = true;
    emailError = "";
    try {
      const updated = await api(TeacherSchema, `/admin/accounts/${target.id}/email`, {
        method: "PUT",
        body: JSON.stringify({ email: newEmail }),
      });
      emailDialog.close();
      notice = `Die Anmelde-E-Mail von ${updated.name} lautet jetzt ${updated.email}. Bitte informiere die Lehrkraft.`;
      await refresh();
    } catch (e) {
      // The dialog stays open on failure so the address can be corrected.
      if (emailDialog.open) emailError = errorMessage(e);
      else error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
  function previewDeletion(account: AccountSummary) {
    return run(async () => {
      deletion = await api(AccountDeletionSchema, `/admin/accounts/${account.id}/deletion`);
      await tick();
      deletionDialog.showModal();
    });
  }
  async function deleted() {
    const name = deletion?.name ?? "Das Konto";
    deletionDialog.close();
    await run(refresh);
    notice = `${name} wurde gelöscht.`;
  }
</script>

<section class="page-heading with-actions">
  <div>
    <span class="eyebrow">ADMINISTRATION</span>
    <h1>Konten verwalten<span>.</span></h1>
    <p>
      {#if loading}Konten werden geladen …{:else}{accounts.length}
        {accounts.length === 1 ? "Konto" : "Konten"} · {adminCount} mit Administrationsrechten{/if}
    </p>
  </div>
  <button class="secondary" onclick={oninvite}><Icon name="key" /> Lehrkraft einladen</button>
</section>

{#if error}<div class="alert" role="alert">{error}</div>{/if}
{#if notice}<div class="info-box" role="status">{notice}</div>{/if}
{#if reset}
  <div class="info-box reset" role="status">
    <strong>Rücksetzlink für {reset.name}</strong>
    <p>
      Einmal verwendbar, gültig bis {new Date(reset.expires * 1000).toLocaleString("de-DE")}. Bitte vertraulich an die
      Lehrkraft weitergeben. Ein neuer Link ersetzt den vorherigen.
    </p>
    <div class="copy-row">
      <label class="grow"
        ><span class="sr-only">Rücksetzlink</span><input
          readonly
          value={reset.link}
          onclick={(e) => e.currentTarget.select()}
        /></label
      >
      <button class="secondary" onclick={copyReset}>{copied ? "Kopiert ✓" : "Link kopieren"}</button>
      <button class="text-button" onclick={() => (reset = null)}>Ausblenden</button>
    </div>
  </div>
{/if}

<div class="toolbar">
  <label class="search"
    ><span class="sr-only">Konten durchsuchen</span><Icon name="search" /><input
      type="search"
      bind:value={query}
      placeholder="Name oder E-Mail suchen …"
    /></label
  >
  <label
    ><span class="sr-only">Rolle</span><select bind:value={role}>
      <option value="all">Alle Rollen</option>
      <option value="admin">Administration</option>
      <option value="teacher">Lehrkräfte</option>
    </select></label
  >
  <label
    ><span class="sr-only">Sortierung</span><select bind:value={sort}>
      <option value="name">Nach Name</option>
      <option value="newest">Neueste zuerst</option>
    </select></label
  >
</div>

<section class="account-list" aria-label="Konten">
  {#if !loading && accounts.length}
    <div class="row head" aria-hidden="true">
      <span>Konto</span><span>Rolle</span><span>Eigene Klassen</span><span>Registriert</span><span></span>
    </div>
  {/if}
  <ul>
    {#each visible as account (account.id)}
      <li class="row">
        <div class="person">
          <span class="profile-avatar" aria-hidden="true">{initials(account.name)}</span>
          <div>
            <strong>{account.name}{#if account.id === teacher.id}{" "}<span class="muted">(du)</span>{/if}</strong>
            <span class="muted">{account.email}</span>
          </div>
        </div>
        <div class="details">
        <span><span class="badge" class:admin={account.is_admin}>{account.is_admin ? "Administration" : "Lehrkraft"}</span></span>
        <span class="meta"><span class="mobile-label">Eigene Klassen:{" "}</span>{account.owned_classrooms}</span>
        <span class="meta"
          ><span class="mobile-label">Registriert{" "}</span>{new Date(account.created_at * 1000).toLocaleDateString("de-DE")}</span
        >
        </div>
        <DropdownMenu label={`Aktionen für ${account.name}`} items={actions(account)} disabled={busy}
          >{#snippet trigger()}<span aria-hidden="true">⋯</span>{/snippet}</DropdownMenu
        >
      </li>
    {:else}
      {#if !loading}<li class="empty">Keine Konten passen zur Suche.</li>{/if}
    {/each}
  </ul>
</section>

<dialog
  bind:this={emailDialog}
  aria-labelledby="account-email-title"
  oncancel={(e) => {
    if (busy) e.preventDefault();
  }}
  onclose={() => {
    emailTarget = null;
    emailError = "";
  }}
>
  {#if emailTarget}
    <form class="modal-content" onsubmit={saveEmail}>
      <div class="modal-top">
        <span class="eyebrow">KONTO</span><button
          type="button"
          class="icon-button"
          disabled={busy}
          onclick={() => emailDialog.close()}
          aria-label="Schließen">×</button
        >
      </div>
      <h2 id="account-email-title">E-Mail-Adresse ändern</h2>
      <p>
        {emailTarget.name} meldet sich danach nur noch mit der neuen Adresse an. Bestehende Sitzungen bleiben aktiv.
      </p>
      {#if emailError}<div class="alert" role="alert">{emailError}</div>{/if}
      <label
        >Neue E-Mail-Adresse<input type="email" bind:value={newEmail} autocomplete="off" maxlength="254" required /></label
      >
      <div class="modal-actions">
        <button type="button" class="secondary" disabled={busy} onclick={() => emailDialog.close()}>Abbrechen</button
        ><button class="primary" disabled={busy}>Speichern</button>
      </div>
    </form>
  {/if}
</dialog>

<dialog
  class="wide"
  bind:this={deletionDialog}
  aria-labelledby="account-deletion-title"
  oncancel={(e) => {
    if (busy) e.preventDefault();
  }}
  onclose={() => (deletion = null)}
>
  {#if deletion}
    <div class="modal-content">
      <div class="modal-top">
        <span class="eyebrow">KONTO</span><button
          class="icon-button"
          disabled={busy}
          onclick={() => deletionDialog.close()}
          aria-label="Schließen">×</button
        >
      </div>
      <h2 id="account-deletion-title">Konto endgültig löschen?</h2>
      <AccountDeletionForm {deletion} own={false} bind:busy onback={() => deletionDialog.close()} ondeleted={deleted} />
    </div>
  {/if}
</dialog>

<style>
  .toolbar {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    margin-bottom: 14px;
  }
  .toolbar label {
    flex-direction: row;
  }
  .toolbar input,
  .toolbar select {
    font-weight: 450;
  }
  .search {
    position: relative;
    flex: 1 1 280px;
    max-width: 460px;
    align-items: center;
    color: var(--muted);
  }
  .search :global(svg) {
    position: absolute;
    left: 12px;
    pointer-events: none;
  }
  .search input {
    width: 100%;
    padding-left: 38px;
  }
  .account-list {
    background: #fff;
    border: 1px solid var(--line);
    border-radius: 12px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .row {
    display: grid;
    grid-template-columns: minmax(0, 2.4fr) minmax(0, 1fr) minmax(0, 0.8fr) minmax(0, 0.8fr) 40px;
    align-items: center;
    gap: 14px;
    padding: 12px 16px;
    border-top: 1px solid var(--line);
  }
  li.row:first-child {
    border-top: 0;
  }
  .row.head {
    border-top: 0;
    border-bottom: 1px solid var(--line);
    color: var(--muted);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.4px;
    text-transform: uppercase;
  }
  .person {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }
  .person > div {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .person strong,
  .person .muted {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .person .profile-avatar {
    width: 34px;
    height: 34px;
    font-size: 12px;
  }
  .badge {
    display: inline-block;
    padding: 3px 9px;
    border-radius: 999px;
    background: #eef3f8;
    color: #3d5378;
    font-size: 12px;
    font-weight: 600;
  }
  .badge.admin {
    background: var(--pale);
    color: var(--green);
  }
  .meta {
    color: #3d5378;
  }
  /* Grid cells on wide screens; one wrapped line under the name on small ones. */
  .details {
    display: contents;
  }
  .mobile-label {
    display: none;
  }
  .empty {
    padding: 28px 16px;
    color: var(--muted);
    text-align: center;
  }
  .info-box,
  .alert {
    margin-bottom: 14px;
  }
  .reset p {
    margin: 4px 0 10px;
  }
  .copy-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
  }
  .copy-row .grow {
    flex: 1 1 260px;
  }
  .copy-row input {
    width: 100%;
  }
  dialog {
    width: min(520px, calc(100vw - 32px));
  }
  dialog.wide {
    width: min(620px, calc(100vw - 32px));
  }
  @media (max-width: 760px) {
    .row {
      grid-template-columns: minmax(0, 1fr) auto;
      gap: 6px 12px;
    }
    .row.head {
      display: none;
    }
    .person {
      grid-area: 1 / 1;
    }
    .row > :global(.dropdown) {
      grid-area: 1 / 2;
    }
    .details {
      grid-area: 2 / 1 / 3 / 3;
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      gap: 4px 14px;
      padding-left: 46px;
      font-size: 12px;
    }
    .mobile-label {
      display: inline;
      color: var(--muted);
    }
  }
</style>
