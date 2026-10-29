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
  {:else if name === "home"}
    <path d="m3 10 9-7 9 7v11h-6v-7H9v7H3Z" />
  {:else if name === "calendar"}
    <rect x="3" y="5" width="18" height="16" rx="2" /><path
      d="M7 3v4m10-4v4M3 11h18"
    />
  {:else if name === "users"}
    <circle cx="9" cy="7" r="3" /><path
      d="M3 21v-3a6 6 0 0 1 12 0v3M16 4a3 3 0 0 1 0 6m2 4a5 5 0 0 1 3 5v2"
    />
  {:else if name === "chart"}
    <path d="M4 20V13h3v7Zm7 0V8h3v12Zm7 0V3h3v17Z" />
  {:else if name === "note"}
    <path d="M14 3H5v18h14V8Zm0 0v6h5M8 13h8m-8 4h6" />
  {:else if name === "book"}
    <path
      d="M12 5c-3-2-6-2-10-1v15c4-1 7-1 10 1 3-2 6-2 10-1V4c-4-1-7-1-10 1Zm0 0v15"
    />
  {:else if name === "check"}
    <rect x="3" y="3" width="18" height="18" rx="3" /><path d="m7 12 3 3 7-7" />
  {:else if name === "folder"}
    <path d="M3 7V4h7l2 3h9v14H3Zm0 4h18" />
  {:else if name === "settings"}
    <path
      d="m9 3-1 3-3 1-2 3 2 2-1 4 3 2 3-1 3 4 3-2v-3l4-2-1-4-3-1-1-4-3-1Z"
    /><circle cx="12" cy="12" r="3" />
  {:else if name === "star"}
    <path d="m12 2 3 6 7 1-5 5 1 7-6-3-6 3 1-7-5-5 7-1Z" />
  {:else if name === "clock"}
    <circle cx="12" cy="12" r="9" /><path d="M12 6v6l4 2" />
  {:else if name === "search"}
    <circle cx="10" cy="10" r="7" /><path d="m15 15 6 6" />
  {:else if name === "arrow"}
    <path d="M4 12h16m-6-6 6 6-6 6" />
  {:else if name === "chevron"}
    <path d="m9 5 7 7-7 7" />
  {:else if name === "leaf"}
    <path d="M20 3C7 1 1 9 5 16c7 5 16-1 15-13ZM3 21 16 8" />
  {:else if name === "bolt"}
    <path d="m13 2-9 12h7l-1 8 10-13h-8Z" />
  {:else if name === "edit"}
    <path d="m16 3 5 5-12 12-6 1 1-6Zm-2 2 5 5" />
  {:else if name === "close"}
    <path d="m6 6 12 12M6 18 18 6" />
  {:else}
    <path d="M12 5v14M5 12h14" />
  {/if}
</svg>
