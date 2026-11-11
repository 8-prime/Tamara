<script lang="ts">
  import { api } from "./api.ts";
  import { UsageSchema } from "./models.ts";
  import type { ErrorHandler, Usage } from "./models.ts";
  import { dateKey, dateLabel } from "./teaching.ts";
  import {
    BUCKETS,
    METRIC_GROUPS,
    METRICS,
    PRESETS,
    formatValue,
    metricValue,
    niceMax,
    periodLabel,
    presetRange,
    usagePath,
    type Bucket,
    type Range,
  } from "./usage.ts";
  interface Props {
    onerror: ErrorHandler;
  }
  let { onerror }: Props = $props();

  // Single-series bar colour, checked for lightness, chroma and contrast on white.
  const BAR = "#1a7d57";
  const W = 720,
    H = 250,
    LEFT = 48,
    RIGHT = 704,
    TOP = 16,
    BASE = 206;

  const initial = presetRange("30d", dateKey(), dateKey());
  let preset = $state("30d"),
    range = $state<Range>(initial),
    data = $state<(Usage & { bucket: Bucket }) | null>(null),
    loading = $state(true),
    metric = $state("note_created"),
    hovered = $state(-1);
  let request = 0;

  const selected = $derived(METRICS.find((m) => m.id === metric) ?? METRICS[0]!);
  const bucket = $derived(
    BUCKETS.find((b) => b.id === (data?.bucket ?? range.bucket)) ?? BUCKETS[0]!,
  );
  const bars = $derived(
    (data?.series ?? []).map((p) => ({
      start: p.start,
      days: p.days,
      value: metricValue(metric, p.counts, p.days),
    })),
  );
  const tip = $derived(hovered >= 0 ? bars[hovered] : undefined);
  const max = $derived(niceMax(Math.max(0, ...bars.map((b) => b.value))));
  const band = $derived(bars.length ? (RIGHT - LEFT) / bars.length : 0);
  const barWidth = $derived(Math.max(2, Math.min(24, band - 2, band * 0.7)));
  const y = (v: number) => BASE - (v / max) * (BASE - TOP);
  const labelled = $derived(
    new Set(
      bars.length <= 12
        ? bars.map((_, i) => i)
        : [0, Math.floor((bars.length - 1) / 2), bars.length - 1],
    ),
  );

  // A rectangle whose data end (top) is rounded by 4px and baseline stays square.
  function barPath(x: number, top: number, w: number) {
    const h = BASE - top;
    if (h <= 0) return "";
    const r = Math.min(4, w / 2, h);
    return `M${x},${BASE}V${top + r}Q${x},${top} ${x + r},${top}H${x + w - r}Q${x + w},${top} ${x + w},${top + r}V${BASE}Z`;
  }

  async function load(next: Range) {
    range = next;
    const id = ++request;
    loading = true;
    try {
      const result = await api(UsageSchema, usagePath(next));
      if (id === request) data = { ...result, bucket: next.bucket };
    } catch (e) {
      if (id === request) onerror(e);
    } finally {
      if (id === request) loading = false;
    }
  }
  function choosePreset(id: string) {
    preset = id;
    load(presetRange(id, dateKey(), data?.counting_since ?? dateKey()));
  }
  function chooseDates(from: string, until: string) {
    preset = "";
    if (from && until && from <= until) load({ ...range, from, until });
    else range = { ...range, from, until };
  }

  load(initial);
</script>

<section class="page-heading">
  <div>
    <span class="eyebrow">ADMINISTRATION · NUR SCHULWEITE SUMMEN</span>
    <h1>So wird Tamara genutzt<span>.</span></h1>
    <p>
      Tageszählungen für die ganze Schule, ohne Namen, Klassen, Kinder oder
      Inhalte.{#if data}{" "}Gezählt wird seit dem {dateLabel(data.counting_since)}; frühere
        Einträge sind nicht enthalten.{/if}
    </p>
  </div>
  <span class="heading-art" aria-hidden="true">∑</span>
</section>

<section class="workspace-card usage" aria-label="Nutzungsstatistik">
  <div class="filters">
    <div class="segmented" role="group" aria-label="Zeitraum">
      {#each PRESETS as item (item.id)}<button
          class:active={preset === item.id}
          aria-pressed={preset === item.id}
          onclick={() => choosePreset(item.id)}>{item.label}</button
        >{/each}
    </div>
    <label
      >Von<input
        type="date"
        value={range.from}
        max={range.until}
        onchange={(e) => chooseDates(e.currentTarget.value, range.until)}
      /></label
    ><label
      >Bis<input
        type="date"
        value={range.until}
        min={range.from}
        onchange={(e) => chooseDates(range.from, e.currentTarget.value)}
      /></label
    >
    <div class="segmented" role="group" aria-label="Raster">
      {#each BUCKETS as item (item.id)}<button
          class:active={range.bucket === item.id}
          aria-pressed={range.bucket === item.id}
          onclick={() => load({ ...range, bucket: item.id })}
          >{item.label}</button
        >{/each}
    </div>
  </div>
  {#if range.from && range.until && range.from > range.until}<p
      class="alert"
      role="alert"
    >
      Das Enddatum muss am oder nach dem Startdatum liegen.
    </p>{/if}

  {#if !data}
    <p class="muted" role="status">Statistik wird geladen …</p>
  {:else}
    <p class="muted range" role="status" aria-busy={loading}>
      {#if data.days}{dateLabel(data.from)} – {dateLabel(data.until)} · {data.days}
        {data.days === 1 ? "Tag" : "Tage"}{:else}In diesem Zeitraum wurde noch
        nicht gezählt.{/if}{#if loading}
        · wird aktualisiert …{/if}
    </p>

    {#each METRIC_GROUPS as group (group.label)}
      <h3>{group.label}</h3>
      <div class="tiles">
        {#each group.metrics as item (item.id)}
          <button
            class="tile"
            class:active={metric === item.id}
            aria-pressed={metric === item.id}
            onclick={() => (metric = item.id)}
          >
            <span class="tile-label">{item.label}</span>
            <strong
              >{formatValue(
                metricValue(item.id, data.totals, data.days),
              )}</strong
            >
            {#if item.id === "teacher_active"}<small
                >Ø pro Tag · höchstens {data.peak_active_teachers} an einem Tag</small
              >{/if}
          </button>
        {/each}
      </div>
    {/each}

    <div class="chart-card">
      <h3>
        {selected.label}
        {selected.unit ? `(${selected.unit})` : ""}
        {bucket.per}
      </h3>
      {#if bars.length}
        <div class="chart">
          <svg
            viewBox="0 0 {W} {H}"
            role="img"
            aria-label="{selected.label} {bucket.per}, {dateLabel(
              data.from,
            )} bis {dateLabel(data.until)}. Die Werte stehen in der Tabelle darunter."
            onpointerleave={() => (hovered = -1)}
          >
            {#each [0, max / 2, max] as tick}<line
                x1={LEFT}
                x2={RIGHT}
                y1={y(tick)}
                y2={y(tick)}
                class:baseline={tick === 0}
              /><text class="axis" x={LEFT - 10} y={y(tick) + 4} text-anchor="end"
                >{formatValue(tick)}</text
              >{/each}
            {#each bars as bar, i (bar.start)}
              {@const x = LEFT + i * band + (band - barWidth) / 2}
              <path
                d={barPath(x, y(bar.value), barWidth)}
                fill={BAR}
                opacity={hovered === -1 || hovered === i ? 1 : 0.55}
              />
              {#if labelled.has(i)}<text
                  class="axis"
                  x={LEFT + i * band + band / 2}
                  y={BASE + 20}
                  text-anchor={bars.length > 12 && i === 0
                    ? "start"
                    : bars.length > 12 && i === bars.length - 1
                      ? "end"
                      : "middle"}>{periodLabel(bar.start, data.bucket)}</text
                >{/if}
              <!-- Hover only adds a tooltip; the table below lists every value. -->
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <rect
                class="hit"
                x={LEFT + i * band}
                y={TOP}
                width={band}
                height={BASE - TOP + 4}
                onpointerenter={() => (hovered = i)}
              />
            {/each}
          </svg>
          {#if tip}
            {@const bar = tip}
            <div
              class="tooltip"
              style:left="{((LEFT + hovered * band + band / 2) / W) * 100}%"
              style:top="{(y(bar.value) / H) * 100}%"
            >
              <span>{periodLabel(bar.start, data.bucket, true)}</span>
              <strong>{formatValue(bar.value)}</strong>
              {selected.unit ?? ""}
            </div>
          {/if}
        </div>
        <details>
          <summary>Werte als Tabelle ({bars.length})</summary>
          <div class="table-scroll">
            <table>
              <thead
                ><tr
                  ><th>Zeitraum</th><th>{selected.label}</th></tr
                ></thead
              ><tbody
                >{#each bars.toReversed() as bar (bar.start)}<tr
                    ><td>{periodLabel(bar.start, data.bucket, true)}</td><td
                      >{formatValue(bar.value)}</td
                    ></tr
                  >{/each}</tbody
              >
            </table>
          </div>
        </details>
      {:else}
        <p class="empty-panel">Keine Tage in diesem Zeitraum.</p>
      {/if}
    </div>
  {/if}
</section>

<style>
  .usage {
    margin-bottom: 20px;
  }
  .filters {
    display: flex;
    flex-wrap: wrap;
    align-items: end;
    gap: 16px;
    margin-bottom: 12px;
  }
  .filters label {
    flex: 0 1 160px;
  }
  .muted {
    font-size: 13px;
    line-height: 1.6;
  }
  .range {
    margin: 0 0 4px;
  }
  h3 {
    margin: 22px 0 10px;
    font-size: 15px;
  }
  .tiles {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
    gap: 10px;
  }
  .tile {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    padding: 12px 14px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: #fff;
    color: var(--ink);
    text-align: left;
  }
  .tile:hover {
    border-color: #b9d8c8;
  }
  .tile.active {
    border-color: var(--green);
    box-shadow: inset 0 0 0 1px var(--green);
  }
  .tile-label {
    color: var(--muted);
    font-size: 13px;
  }
  .tile strong {
    font-size: 24px;
    line-height: 1.2;
    font-variant-numeric: tabular-nums;
  }
  .tile small {
    color: var(--muted);
    font-size: 12px;
  }
  .chart-card {
    margin-top: 26px;
    padding-top: 6px;
    border-top: 1px solid var(--line);
  }
  .chart {
    position: relative;
    overflow-x: auto;
  }
  svg {
    display: block;
    width: 100%;
    min-width: 440px;
  }
  svg line {
    stroke: var(--line);
  }
  svg line.baseline {
    stroke: #c5d3e0;
  }
  .axis {
    fill: var(--muted);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .hit {
    fill: transparent;
  }
  .tooltip {
    position: absolute;
    transform: translate(-50%, calc(-100% - 10px));
    padding: 6px 10px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: #fff;
    box-shadow: 0 4px 14px rgb(16 33 76 / 12%);
    font-size: 12px;
    white-space: nowrap;
    pointer-events: none;
  }
  .tooltip span {
    display: block;
    color: var(--muted);
  }
  .tooltip strong {
    font-size: 15px;
    font-variant-numeric: tabular-nums;
  }
  .table-scroll {
    overflow-x: auto;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    text-align: left;
    font-size: 13px;
  }
  td,
  th {
    padding: 10px;
    border-bottom: 1px solid var(--line);
  }
  td:last-child,
  th:last-child {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  summary {
    cursor: pointer;
    padding: 12px 0;
  }
</style>
