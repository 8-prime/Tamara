<script lang="ts">
  import { onMount } from 'svelte';
  import { api, errorMessage } from './api.ts';
  import { AccountSummarySchema, AccountDeletionSchema, PasswordResetResultSchema } from './models.ts';
  import type { Teacher, AccountSummary, AccountDeletion } from './models.ts';
  interface Props { teacher: Teacher; onclose: () => void; onremoved: () => void; onrolechange: () => Promise<void> }
  let {teacher, onclose, onremoved, onrolechange}: Props = $props();
  let dialog: HTMLDialogElement;
  let accounts = $state<AccountSummary[]>([]), deletion = $state<AccountDeletion | null>(null);
  let password = $state(''), confirmation = $state(false), busy = $state(false), error = $state('');
  let resetLink = $state(''), resetName = $state(''), resetExpiry = $state(0);
  async function refresh() { if (teacher.is_admin) accounts = await api(AccountSummarySchema.array(), '/admin/accounts'); }
  onMount(() => { dialog.showModal(); refresh().catch(e => error = errorMessage(e)); });
  async function preview(id: string) {
    busy = true; error = ''; resetLink = ''; password = ''; confirmation = false;
    try { deletion = await api(AccountDeletionSchema, id === teacher.id ? '/auth/me/deletion' : `/admin/accounts/${id}/deletion`); }
    catch(e) { error = errorMessage(e); } finally { busy = false; }
  }
  async function remove(event: SubmitEvent) {
    event.preventDefault(); if (!deletion || !confirmation) return;
    const target = deletion; busy = true; error = '';
    try {
      await api(target.id === teacher.id ? '/auth/me' : `/admin/accounts/${target.id}`, {method:'DELETE', body:JSON.stringify({password, owned_classroom_ids:target.owned_classrooms.map(c => c.id)})});
      password = ''; deletion = null;
      if (target.id === teacher.id) { onremoved(); onclose(); }
      else await refresh();
    } catch(e) { error = errorMessage(e); } finally { busy = false; }
  }
  async function reset(account: AccountSummary) {
    busy = true; error = ''; resetLink = '';
    try {
      const result = await api(PasswordResetResultSchema, `/admin/accounts/${account.id}/password-reset`, {method:'POST'});
      const link = new URL('/', window.location.origin); link.hash = `reset-password=${result.token}`;
      resetLink = link.href; resetName = account.name; resetExpiry = result.expires_at;
    } catch(e) { error = errorMessage(e); } finally { busy = false; }
  }
  async function role(account: AccountSummary) {
    if (!confirm(`${account.name}: Administrationsrechte ${account.is_admin ? 'entziehen' : 'erteilen'}?`)) return;
    busy = true; error = '';
    try {
      await api(`/admin/accounts/${account.id}`, {method:'PATCH',body:JSON.stringify({is_admin:!account.is_admin})});
      await onrolechange();
      if (teacher.is_admin) await refresh(); else accounts = [];
    } catch(e) { error = errorMessage(e); } finally { busy = false; }
  }
</script>
<dialog bind:this={dialog} aria-labelledby="account-title" oncancel={e => { if (busy) e.preventDefault(); else onclose(); }} onclose={onclose}>
  <div class="modal-content">
    <div class="modal-top"><span class="eyebrow">KONTO &amp; ZUGRIFF</span><button class="icon-button" disabled={busy} onclick={onclose} aria-label="Schließen">×</button></div>
    <h2 id="account-title">{deletion ? 'Konto endgültig löschen?' : teacher.is_admin ? 'Konten verwalten' : 'Mein Konto'}</h2>
    {#if error}<div class="alert" role="alert">{error}</div>{/if}
    {#if deletion}
      <form onsubmit={remove}>
        <p><strong>{deletion.name} ({deletion.email})</strong> wird gelöscht. Anmeldung, Sitzungen, Rücksetzlinks und private Konferenznotizen werden entfernt.</p>
        <p>Diese eigenen Klassen werden mit allen Kindern, Bewertungen, Notizen, Hausaufgaben und Materialien gelöscht – auch für mitarbeitende Lehrkräfte:</p>
        <ul>{#each deletion.owned_classrooms as classroom}<li>{classroom.name} ({classroom.student_count} Kinder)</li>{:else}<li>Keine eigenen Klassen</li>{/each}</ul>
        <p>Einträge in Klassen anderer Lehrkräfte bleiben ohne Kontozuordnung erhalten. Bestehende Serversicherungen werden dadurch nicht geändert.</p>
        <label>Dein aktuelles Passwort<input type="password" bind:value={password} autocomplete="current-password" maxlength="256" required /></label>
        <label><input type="checkbox" bind:checked={confirmation} required /> Ich habe die Folgen geprüft. Die Löschung kann nicht rückgängig gemacht werden.</label>
        <div class="modal-actions"><button type="button" class="secondary" disabled={busy} onclick={() => {deletion=null; password=''; error='';}}>Zurück</button><button class="primary danger" disabled={busy || !confirmation}>Konto und eigene Daten löschen</button></div>
      </form>
    {:else}
      {#if resetLink}<div class="info-box" role="status"><strong>Rücksetzlink für {resetName}</strong><p>Einmal verwendbar, gültig bis {new Date(resetExpiry*1000).toLocaleString('de-DE')}. Bitte vertraulich an die Lehrkraft weitergeben. Ein neuer Link ersetzt den vorherigen.</p><label>Link zum Kopieren<input readonly value={resetLink} onclick={e => e.currentTarget.select()} /></label></div>{/if}
      {#if teacher.is_admin}
        <p>Registrierte Konten: {accounts.length}</p>
        {#each accounts as account}<section class="account-row">
          <strong>{account.name}</strong><div>{account.email}</div><small>{account.is_admin ? 'Administration' : 'Lehrkraft'} · {account.owned_classrooms} eigene Klassen · registriert {new Date(account.created_at*1000).toLocaleDateString('de-DE')}</small>
          <div class="account-actions"><button class="secondary" disabled={busy} onclick={() => reset(account)}>Passwort zurücksetzen</button><button class="secondary" disabled={busy} onclick={() => role(account)}>{account.is_admin ? 'Adminrechte entziehen' : 'Zur Administration ernennen'}</button><button class="text-button danger-text" disabled={busy} onclick={() => preview(account.id)}>Konto löschen</button></div>
        </section>{/each}
      {:else}<p>{teacher.name} · {teacher.email}</p>{/if}
      <div class="modal-actions"><button class="text-button danger-text" disabled={busy} onclick={() => preview(teacher.id)}>Mein Konto löschen</button><button class="secondary" disabled={busy} onclick={onclose}>Schließen</button></div>
    {/if}
  </div>
</dialog>
<style>
  dialog { width: min(760px, calc(100vw - 32px)); }
  .account-row {padding: 16px 0; border-bottom: 1px solid var(--line, #ddd);}
  .account-actions {display:flex;flex-wrap:wrap;gap:8px;margin-top:10px;}
</style>
