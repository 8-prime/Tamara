<script>
  let { name = "plus", size = 16 } = $props();
</script>

<svg
  class="ui-icon"
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.75"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
  focusable="false"
>
  {#if name === "key"}
    <circle cx="8" cy="8" r="5" />
    <path d="m11.5 11.5 9 9M17 17l3-3M14 14l3-3" />
  {:else if name === "backup"}
    <ellipse cx="12" cy="5" rx="8" ry="3" />
    <path d="M4 5v14c0 4 16 4 16 0V5M4 12c0 4 16 4 16 0" />
  {:else}
    <path d="M12 5v14M5 12h14" />
  {/if}
</svg>
