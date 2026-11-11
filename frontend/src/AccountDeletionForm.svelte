<script lang="ts">
  import { api, errorMessage } from "./api.ts";
  import type { AccountDeletion } from "./models.ts";
  interface Props {
    deletion: AccountDeletion;
    /** Deleting the signed-in account rather than another one as administrator. */
    own: boolean;
    busy?: boolean;
    onback: () => void;
    ondeleted: () => void;
  }
  let { deletion, own, busy = $bindable(false), onback, ondeleted }: Props = $props();
  let password = $state(""), confirmation = $state(false), error = $state("");

  async function remove(event: SubmitEvent) {
    event.preventDefault();
    if (!confirmation) return;
    busy = true;
    error = "";
    try {
      await api(own ? "/auth/me" : `/admin/accounts/${deletion.id}`, {
        method: "DELETE",
        body: JSON.stringify({ password, owned_classroom_ids: deletion.owned_classrooms.map((c) => c.id) }),
      });
      password = "";
      ondeleted();
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
</script>

<form onsubmit={remove}>
  {#if error}<div class="alert" role="alert">{error}</div>{/if}
  <p>
    <strong>{deletion.name} ({deletion.email})</strong> wird gelöscht. Anmeldung, Sitzungen, Rücksetzlinks und private
    Konferenznotizen werden entfernt.
  </p>
  <p>
    Diese eigenen Klassen werden mit allen Kindern, Bewertungen, Notizen, Hausaufgaben und Materialien gelöscht – auch für
    mitarbeitende Lehrkräfte:
  </p>
  <ul>
    {#each deletion.owned_classrooms as classroom (classroom.id)}<li>
        {classroom.name} ({classroom.student_count} Kinder)
      </li>{:else}<li>Keine eigenen Klassen</li>{/each}
  </ul>
  <p>
    Einträge in Klassen anderer Lehrkräfte bleiben ohne Kontozuordnung erhalten. Bestehende Serversicherungen werden
    dadurch nicht geändert.
  </p>
  <label
    >Dein aktuelles Passwort<input
      type="password"
      bind:value={password}
      autocomplete="current-password"
      maxlength="256"
      required
    /></label
  >
  <label class="confirm"
    ><input type="checkbox" bind:checked={confirmation} required /> Ich habe die Folgen geprüft. Die Löschung kann nicht
    rückgängig gemacht werden.</label
  >
  <div class="modal-actions">
    <button type="button" class="secondary" disabled={busy} onclick={onback}>Zurück</button><button
      class="primary danger"
      disabled={busy || !confirmation}>{own ? "Mein Konto und eigene Daten löschen" : "Konto und eigene Daten löschen"}</button
    >
  </div>
</form>

<style>
  .confirm {
    flex-direction: row;
    align-items: flex-start;
    gap: 10px;
    font-weight: 500;
  }
</style>
