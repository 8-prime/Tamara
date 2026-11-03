<script lang="ts">
  import { api, errorMessage } from './api.ts';
  interface Props { token: string; ondone: () => void }
  let {token, ondone}: Props = $props();
  let password = $state(''), repeated = $state(''), error = $state(''), busy = $state(false), complete = $state(false);
  async function submit(event: SubmitEvent) {
    event.preventDefault(); error = '';
    if (password !== repeated) { error = 'Die Passwörter stimmen nicht überein.'; return; }
    busy = true;
    try { await api('/auth/reset-password', {method:'POST',body:JSON.stringify({token,password})}); password=''; repeated=''; complete=true; }
    catch(e) {error=errorMessage(e);} finally {busy=false;}
  }
</script>
<main class="auth-form-area">
  <form class="auth-form" onsubmit={submit}>
    <h1>Passwort zurücksetzen</h1>
    {#if error}<div class="alert" role="alert">{error}</div>{/if}
    {#if complete}<p role="status">Das Passwort wurde geändert. Bestehende Sitzungen wurden abgemeldet.</p><button type="button" class="primary" onclick={ondone}>Zur Anmeldung</button>
    {:else}<p>Lege ein neues Passwort fest. Falls der Link abgelaufen ist, bitte deine Administration um einen neuen.</p>
      <label>Neues Passwort<input type="password" bind:value={password} autocomplete="new-password" minlength="12" maxlength="256" required /></label>
      <label>Passwort wiederholen<input type="password" bind:value={repeated} autocomplete="new-password" minlength="12" maxlength="256" required /></label>
      <button class="primary" disabled={busy}>Passwort speichern</button><button type="button" class="text-button" disabled={busy} onclick={ondone}>Zur Anmeldung</button>
    {/if}
  </form>
</main>
