<script>
  import { onMount } from "svelte";
  import Icon from "./Icon.svelte";
  let { classrooms, action, onchoose, onclose } = $props();
  let dialog = $state(null);
  const actionLabels = {
    attendance: "Anwesenheit erfassen",
    note: "Notiz erfassen",
    materials: "Material finden & teilen",
  };
  onMount(() => dialog.showModal());
  function choose(classroom) {
    dialog.close();
    onchoose(classroom);
  }
</script>

<dialog
  bind:this={dialog}
  class="class-action-dialog"
  aria-labelledby="class-action-title"
  {onclose}
>
  <div class="modal-content">
    <div class="panel-heading">
      <h2 id="class-action-title">Klasse auswählen</h2>
      <button
        class="icon-button"
        aria-label="Dialog schließen"
        onclick={() => dialog.close()}><Icon name="close" /></button
      >
    </div>
    <p>{actionLabels[action]}: Für welche Klasse?</p>
    <div class="class-action-list">
      {#each classrooms as classroom (classroom.id)}
        <button class="secondary" onclick={() => choose(classroom)}>
          <Icon name="users" /><span
            ><strong>Klasse {classroom.name}</strong><small
              >{classroom.school_year}</small
            ></span
          ><Icon name="chevron" />
        </button>
      {/each}
    </div>
  </div>
</dialog>

<style>
  .class-action-dialog {
    width: min(480px, calc(100vw - 32px));
  }
  .class-action-list {
    display: grid;
    gap: 8px;
    margin-top: 20px;
  }
  .class-action-list button {
    justify-content: flex-start;
    text-align: left;
    padding: 13px;
  }
  .class-action-list span {
    flex: 1;
  }
  .class-action-list strong,
  .class-action-list small {
    display: block;
  }
  .class-action-list small {
    color: var(--muted);
    margin-top: 3px;
  }
</style>
