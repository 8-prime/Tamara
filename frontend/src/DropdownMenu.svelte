<script lang="ts" module>
  export interface MenuItem {
    label: string;
    onselect: () => void;
    danger?: boolean;
    /** Draws a divider above the item. */
    separated?: boolean;
  }
</script>

<script lang="ts">
  import { tick, type Snippet } from "svelte";
  interface Props {
    /** Accessible name of the menu (and of the trigger unless triggerLabel is set). */
    label: string;
    triggerLabel?: string;
    items: MenuItem[];
    disabled?: boolean;
    triggerClass?: string;
    trigger: Snippet;
  }
  let { label, triggerLabel = label, items, disabled = false, triggerClass = "icon-button", trigger }: Props = $props();
  const id = $props.id();
  let open = $state(false);
  let root: HTMLDivElement, button: HTMLButtonElement;
  let menu = $state<HTMLDivElement>();

  function entries() {
    return [...(menu?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]') ?? [])];
  }
  async function show(focus: "first" | "last" = "first") {
    open = true;
    await tick();
    const all = entries();
    (focus === "first" ? all[0] : all.at(-1))?.focus();
  }
  function hide(restoreFocus = true) {
    open = false;
    if (restoreFocus) button.focus();
  }
  function triggerKey(event: KeyboardEvent) {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      show(event.key === "ArrowDown" ? "first" : "last");
    }
  }
  function menuKey(event: KeyboardEvent) {
    const all = entries();
    const index = all.indexOf(document.activeElement as HTMLButtonElement);
    const move = (to: number) => {
      event.preventDefault();
      all[(to + all.length) % all.length]?.focus();
    };
    if (event.key === "ArrowDown") move(index + 1);
    else if (event.key === "ArrowUp") move(index - 1);
    else if (event.key === "Home") move(0);
    else if (event.key === "End") move(all.length - 1);
    else if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      hide();
    } else if (event.key === "Tab") hide(false);
  }
  function select(item: MenuItem) {
    hide();
    item.onselect();
  }
  function outside(event: PointerEvent) {
    if (open && !root.contains(event.target as Node)) hide(false);
  }
</script>

<svelte:window onpointerdown={outside} />
<div class="dropdown" bind:this={root}>
  <button
    bind:this={button}
    type="button"
    class={triggerClass}
    aria-label={triggerLabel}
    aria-haspopup="menu"
    aria-expanded={open}
    aria-controls={open ? id : undefined}
    {disabled}
    onclick={() => (open ? hide(false) : show())}
    onkeydown={triggerKey}>{@render trigger()}</button
  >
  {#if open}
    <div class="dropdown-menu" role="menu" {id} aria-label={label} tabindex="-1" bind:this={menu} onkeydown={menuKey}>
      {#each items as item (item.label)}
        {#if item.separated}<div class="separator" role="separator"></div>{/if}
        <button type="button" role="menuitem" tabindex="-1" class:danger={item.danger} onclick={() => select(item)}
          >{item.label}</button
        >
      {/each}
    </div>
  {/if}
</div>

<style>
  .dropdown {
    position: relative;
    display: inline-flex;
  }
  .dropdown-menu {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    z-index: 40;
    display: flex;
    flex-direction: column;
    min-width: 210px;
    padding: 6px;
    background: #fff;
    border: 1px solid var(--line);
    border-radius: 10px;
    box-shadow: 0 12px 30px #182b4a1f;
  }
  .dropdown-menu button {
    padding: 9px 11px;
    border-radius: 7px;
    background: transparent;
    color: var(--ink);
    font-size: 13px;
    text-align: left;
    white-space: nowrap;
  }
  .dropdown-menu button:hover,
  .dropdown-menu button:focus-visible {
    background: var(--pale);
    outline: none;
  }
  .dropdown-menu button.danger {
    color: #c4483a;
  }
  .dropdown-menu button.danger:hover,
  .dropdown-menu button.danger:focus-visible {
    background: #fbecea;
  }
  .separator {
    height: 1px;
    margin: 5px 4px;
    background: var(--line);
  }
</style>
