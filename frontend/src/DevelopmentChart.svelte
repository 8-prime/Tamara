<script lang="ts">
  import { dateLabel } from "./teaching.ts";
  import { performancePoints } from "./pupil-history.ts";
  let { records, label } = $props();
  const points = $derived(performancePoints(records));
  const subjects = $derived(
    [
      ...new Map<string, { id: string; name: string }>(
        records.map((r) => [
          r.subject_id,
          { id: r.subject_id, name: r.subject_name },
        ]),
      ).values(),
    ].sort((a, b) => a.name.localeCompare(b.name, "de")),
  );
  const series = $derived(
    subjects.map((s, i) => ({
      ...s,
      color: ["#267052", "#365eb5", "#ac4c7b", "#93651c", "#6d4fab"][i % 5],
      points: points.filter((p) => p.subject_id === s.id),
    })),
  );
</script>

<div class="chart-scroll">
  <svg viewBox="0 0 720 265" role="img" aria-label={label}>
    {#each [1, 2, 3, 4] as rating}<line
        x1="60"
        x2="660"
        y1={210 - (rating - 1) * 60}
        y2={210 - (rating - 1) * 60}
        stroke="var(--line)"
      /><text x="35" y={215 - (rating - 1) * 60}>{rating}</text>{/each}
    {#each series as line}<polyline
        points={line.points.map((p) => `${p.x},${p.y}`).join(" ")}
        fill="none"
        stroke={line.color}
        stroke-width="2"
      />{#each line.points as point}<circle
          cx={point.x}
          cy={point.y}
          r="5"
          fill={line.color}
          ><title
            >{dateLabel(point.date)} · {point.time} · {point.subject_name}: {point.rating.toLocaleString(
              "de-DE",
              { maximumFractionDigits: 2 },
            )}/4{point.count
              ? ` · ${point.count} Kinder bewertet`
              : ""}{point.observation ? ` · ${point.observation}` : ""}</title
          ></circle
        >{/each}{/each}
    {#if points.length}<text x="60" y="245">{dateLabel(points[0].date)}</text
      ><text x="660" y="245" text-anchor="end"
        >{dateLabel(points.at(-1).date)}</text
      >{/if}
  </svg>
</div>
<div class="legend">
  {#each series as line}<span
      ><i style:background={line.color}></i>{line.name}</span
    >{/each}
</div>

<style>
  .chart-scroll {
    overflow-x: auto;
  }
  svg {
    display: block;
    width: 100%;
    min-width: 440px;
  }
  svg text {
    fill: var(--muted);
    font-size: 13px;
  }
  .legend {
    display: flex;
    gap: 16px;
    flex-wrap: wrap;
    margin: 20px 0;
    font-size: 13px;
  }
  .legend span {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .legend i {
    width: 10px;
    height: 10px;
    border-radius: 50%;
  }
</style>
