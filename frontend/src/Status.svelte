<script>
  let {
    value = null,
    achievedOn = null,
    ondate,
    onchoose,
    compact = false,
    disabled = false,
    label = "Bewertung",
  } = $props();
  const options = [
    { v: null, s: "–", t: "Nicht beurteilt" },
    { v: "no", s: "×", t: "Noch nicht" },
    { v: "yes", s: "✓", t: "Erreicht" },
  ];
</script>

<div class:compact class="assessment-control">
  <div class:compact class="status-switch" role="group" aria-label={label}>
    {#each options as o}
      <button
        type="button"
        class:active={value === o.v}
        class:yes={o.v === "yes"}
        class:no={o.v === "no"}
        aria-pressed={value === o.v}
        title={o.t}
        aria-label={`${label}: ${o.t}`}
        {disabled}
        onclick={() => onchoose(o.v)}
        ><span>{o.s}</span>{#if !compact}<span class="status-word">{o.t}</span
          >{/if}</button
      >
    {/each}
  </div>

  {#if value === "yes" && achievedOn}
    <label class="achieved-date">
      <span>Erreicht am</span>
      <input
        type="date"
        value={achievedOn}
        required
        {disabled}
        aria-label={`${label}: Erreicht am`}
        onchange={async (e) => {
          const input = e.currentTarget;
          if (input.value && input.validity.valid) await ondate(input.value);
          input.value = achievedOn || "";
        }}
      />
    </label>
  {/if}
</div>
