<script lang="ts">
  export let points: Array<{ at: string; value: number }> = [];
  export let color = "#6BA6E8";
  export let scope = "day";
  export let label = "Activity";
  export let loading = false;
  export let error = "";
  export let onRetry: (() => void) | null = null;

  const width = 760;
  const height = 220;
  const padLeft = 44;
  const padRight = 10;
  const padTop = 14;
  const padBottom = 28;

  $: values = points.map((point) =>
    Number.isFinite(point.value) ? Math.max(0, point.value) : 0,
  );
  $: maxValue = Math.max(1, ...values);
  $: innerWidth = width - padLeft - padRight;
  $: innerHeight = height - padTop - padBottom;
  $: coords = values.map((value, index) => {
    const x =
      padLeft + (values.length <= 1 ? innerWidth / 2 : (index / (values.length - 1)) * innerWidth);
    const y = padTop + innerHeight - (value / maxValue) * innerHeight;
    return { x, y };
  });
  $: linePath = coords
    .map((point, index) => `${index === 0 ? "M" : "L"}${point.x.toFixed(1)} ${point.y.toFixed(1)}`)
    .join(" ");
  $: areaPath =
    coords.length > 0
      ? `${linePath} L${(padLeft + innerWidth).toFixed(1)} ${(padTop + innerHeight).toFixed(1)} L${padLeft.toFixed(1)} ${(padTop + innerHeight).toFixed(1)} Z`
      : "";
  $: yTicks = [0, 0.25, 0.5, 0.75, 1].map((ratio) => ({
    y: padTop + innerHeight * ratio,
    value: maxValue * (1 - ratio),
  }));
  $: xTicks = tickIndices(points.length).map((index) => ({
    index,
    x:
      padLeft +
      (points.length <= 1 ? innerWidth / 2 : (index / (points.length - 1)) * innerWidth),
    label: axisLabel(points[index]?.at ?? "", scope),
    anchor: index === 0 ? "start" : index === points.length - 1 ? "end" : "middle",
  }));

  function tickIndices(length: number): number[] {
    if (length === 0) return [];
    if (length === 1) return [0];
    const last = length - 1;
    return [...new Set([0, Math.round(last * 0.25), Math.round(last * 0.5), Math.round(last * 0.75), last])];
  }

  function compact(value: number): string {
    if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
    if (value >= 1_000) return `${(value / 1_000).toFixed(1)}k`;
    return Math.round(value).toLocaleString();
  }

  function axisLabel(at: string, span: string): string {
    const date = new Date(at);
    if (!Number.isFinite(date.getTime())) return "";
    if (span === "1h" || span === "6h" || span === "day") {
      return date.toLocaleTimeString(undefined, {
        hour: "2-digit",
        minute: "2-digit",
        hour12: false,
      });
    }
    if (span === "year" || span === "all") {
      return date.toLocaleDateString(undefined, { month: "short" });
    }
    return date.toLocaleDateString(undefined, { month: "short", day: "numeric" });
  }
</script>

{#if error}
  <div class="chart-state">
    <span>{error}</span>
    {#if onRetry}
      <button on:click={onRetry}>Try again</button>
    {/if}
  </div>
{:else if loading}
  <div class="chart-state"><span>Loading chart…</span></div>
{:else if points.length === 0}
  <div class="chart-state"><span>No activity recorded for this period.</span></div>
{:else}
  <svg
    class="chart"
    viewBox={`0 0 ${width} ${height}`}
    role="img"
    aria-label={`${label} chart`}
  >
    {#each yTicks as tick}
      <line
        x1={padLeft}
        x2={width - padRight}
        y1={tick.y}
        y2={tick.y}
        stroke="var(--line)"
        stroke-width="1"
      />
      <text
        x={padLeft - 8}
        y={tick.y + 3.5}
        text-anchor="end"
        fill="var(--muted)"
        font-size="10.5"
      >{compact(tick.value)}</text>
    {/each}

    <path d={areaPath} fill={color} fill-opacity="0.13" />
    <path
      d={linePath}
      fill="none"
      stroke={color}
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
    />

    {#each xTicks as tick}
      <text
        x={tick.x}
        y={height - 8}
        text-anchor={tick.anchor}
        fill="var(--muted)"
        font-size="10.5"
      >{tick.label}</text>
    {/each}
  </svg>
{/if}

<style>
  .chart {
    display: block;
    width: 100%;
    height: auto;
    overflow: visible;
  }
  .chart-state {
    min-height: 190px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    color: var(--muted);
    font-size: 11.5px;
    text-align: center;
  }
  .chart-state button {
    border: 1px solid var(--line-strong);
    border-radius: 7px;
    background: var(--panel-2);
    color: var(--text-dim);
    font-size: 10.5px;
    padding: 5px 9px;
  }
  .chart-state button:hover {
    color: var(--text);
    background: var(--panel-3);
  }
</style>
