<script lang="ts">
  import * as api from "../api";
  import { hasPermission } from "../permissions";
  import type {
    ActiveUsersPoint,
    ActivityCalendar,
    ActivityPunchcard,
    AnalyticsBundle,
    AnalyticsPoint,
  } from "../types";
  import { servers } from "../workspace";
  import { statsElevatedVisible } from "../stats";
  import Icon from "./Icon.svelte";
  import StatsChart from "./StatsChart.svelte";

  export let serverId: string;
  export let channelId: string;
  export let permissionBits: string | null = null;

  type ActivityMetric = "presence_count" | "member_count" | "voice_peak";
  type ActiveMetric = "dau" | "wau" | "mau";

  const activityMetrics: Array<{ value: ActivityMetric; label: string }> = [
    { value: "presence_count", label: "Online" },
    { value: "member_count", label: "Members" },
    { value: "voice_peak", label: "Voice peak" },
  ];

  const activityScopes = [
    { value: "1h", label: "1 hour" },
    { value: "6h", label: "6 hours" },
    { value: "day", label: "Day" },
    { value: "week", label: "Week" },
    { value: "month", label: "Month" },
    { value: "6months", label: "6 months" },
    { value: "year", label: "Year" },
    { value: "all", label: "All" },
  ];

  const activeMetrics: Array<{ value: ActiveMetric; label: string }> = [
    { value: "dau", label: "Daily active" },
    { value: "wau", label: "Weekly active" },
    { value: "mau", label: "Monthly active" },
  ];

  const activeScopeOptions: Record<ActiveMetric, Array<{ value: string; label: string }>> = {
    dau: [
      { value: "week", label: "Week" },
      { value: "month", label: "Month" },
    ],
    wau: [
      { value: "month", label: "Month" },
      { value: "6months", label: "6 months" },
      { value: "year", label: "Year" },
    ],
    mau: [
      { value: "6months", label: "6 months" },
      { value: "year", label: "Year" },
      { value: "all", label: "All" },
    ],
  };

  const colors: Record<ActivityMetric | ActiveMetric, string> = {
    presence_count: "#6FA8DC",
    member_count: "#EF5252",
    voice_peak: "#8FBF9F",
    dau: "#EF5252",
    wau: "#6FA8DC",
    mau: "#8FBF9F",
  };

  const punchcardOptions = [
    { value: 8, label: "This week" },
    { value: 28, label: "Month" },
    { value: 91, label: "3 Months" },
  ];

  const monthNames = [
    "Jan",
    "Feb",
    "Mar",
    "Apr",
    "May",
    "Jun",
    "Jul",
    "Aug",
    "Sep",
    "Oct",
    "Nov",
    "Dec",
  ];
  const weekdayLabels = ["", "Mon", "", "Wed", "", "Fri", ""];
  const weekdayShort = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
  const weekdayLong = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
  ];

  let bundle: AnalyticsBundle | null = null;
  let bundleLoading = true;
  let bundleError = "";
  let permissionDenied = false;

  let activityMetric: ActivityMetric = "presence_count";
  let activityScope = "day";
  let activeMetric: ActiveMetric = "dau";
  let activeScope = "week";

  let calendarYear = new Date().getUTCFullYear();
  let calendar: ActivityCalendar | null = null;
  let calendarLoading = false;
  let calendarError = "";

  let punchcardDays = 8;
  let punchcard: ActivityPunchcard | null = null;
  let punchcardLoading = false;
  let punchcardError = "";

  let requestKey = "";
  let calendarKey = "";
  let punchcardKey = "";

  $: key = `${serverId}:${channelId}`;
  $: server = $servers.find((entry) => entry.id === serverId) ?? null;
  $: canGeneral =
    permissionBits === null || hasPermission(permissionBits, "ViewGeneralStats");
  $: canElevated =
    permissionBits !== null && hasPermission(permissionBits, "ViewElevatedStats");
  $: showElevated = canElevated && $statsElevatedVisible;
  $: deniedFromPermission = permissionBits !== null && !canGeneral;
  $: showDenied = deniedFromPermission || permissionDenied;
  $: years = calendarYears(server?.created_at);

  $: if (years.length > 0 && !years.includes(calendarYear)) {
    calendarYear = years[0];
  }

  $: if (key !== requestKey) {
    requestKey = key;
    bundle = null;
    bundleError = "";
    permissionDenied = false;
    calendar = null;
    calendarError = "";
    calendarKey = "";
    punchcard = null;
    punchcardError = "";
    punchcardKey = "";
    if (canGeneral) {
      void loadBundle(key);
    } else {
      bundleLoading = false;
    }
  }

  $: if (showElevated && key) {
    const nextKey = `${key}:${calendarYear}`;
    if (nextKey !== calendarKey) {
      calendarKey = nextKey;
      void loadCalendar(nextKey);
    }
    const nextPunch = `${key}:${punchcardDays}`;
    if (nextPunch !== punchcardKey) {
      punchcardKey = nextPunch;
      void loadPunchcard(nextPunch);
    }
  }

  $: if (!showElevated) {
    calendarLoading = false;
    punchcardLoading = false;
  }

  $: activeScopes = activeScopeOptions[activeMetric];
  $: if (!activeScopes.some((option) => option.value === activeScope)) {
    activeScope = activeScopes[0].value;
  }

  $: memberCount = server?.approx_member_count ?? bundle?.live.member_count ?? 0;
  $: onlineCount = server?.approx_presence_count ?? bundle?.live.presence_count ?? 0;
  $: voiceCount = bundle?.live.voice_count ?? 0;

  $: weekSeries = bundle?.scopes?.week?.series ?? [];
  $: memberSpark = weekSeries.map((point) => numberAt(point, "member_count"));
  $: onlineSpark = weekSeries.map((point) => numberAt(point, "presence_count"));
  $: voiceSpark = weekSeries.map((point) => numberAt(point, "voice_peak"));
  $: memberDelta = seriesDelta(memberSpark);
  $: onlineDelta = seriesDelta(onlineSpark);
  $: voiceDelta = seriesDelta(voiceSpark);

  $: activitySeries = bundle?.scopes?.[activityScope]?.series ?? [];
  $: activityPoints = activitySeries.map((point) => ({
    at: point.at,
    value: numberAt(point, activityMetric),
  }));
  $: activityColor = colors[activityMetric];
  $: activityLabel =
    activityMetrics.find((option) => option.value === activityMetric)?.label ?? "Activity";

  $: liveActive = bundle?.active_users?.live ?? null;
  $: monthActiveSeries = bundle?.active_users?.scopes?.month?.series ?? [];
  $: activeSeries = bundle?.active_users?.scopes?.[activeScope]?.series ?? [];
  $: activePoints = activeSeries.map((point) => ({
    at: point.at,
    value: numberAt(point, activeMetric),
  }));
  $: dauDelta = activeDelta(liveActive, monthActiveSeries, "dau", 1);
  $: wauDelta = activeDelta(liveActive, monthActiveSeries, "wau", 7);
  $: mauDelta = activeDelta(liveActive, monthActiveSeries, "mau", 30);
  $: dauShare = memberCount > 0 ? ((liveActive?.dau ?? 0) / memberCount) * 100 : null;
  $: wauShare = memberCount > 0 ? ((liveActive?.wau ?? 0) / memberCount) * 100 : null;
  $: mauShare = memberCount > 0 ? ((liveActive?.mau ?? 0) / memberCount) * 100 : null;
  $: weeklyPeriod = periodLabel(6);
  $: monthlyPeriod = periodLabel(29);

  $: calendarModel = buildCalendar(calendar?.days ?? [], calendarYear);
  $: punchcardModel = buildPunchcard(punchcard);

  async function loadBundle(targetKey: string) {
    bundleLoading = true;
    bundleError = "";
    permissionDenied = false;
    try {
      const data = await api.fetchChannelAnalytics(serverId, channelId);
      if (requestKey !== targetKey) return;
      bundle = data;
    } catch (error) {
      if (requestKey !== targetKey) return;
      const message = error instanceof Error ? error.message : String(error);
      if (/403|forbidden|permission/i.test(message)) permissionDenied = true;
      else bundleError = message || "Could not load activity history.";
    } finally {
      if (requestKey === targetKey) bundleLoading = false;
    }
  }

  async function loadCalendar(targetKey: string) {
    calendarLoading = true;
    calendarError = "";
    calendar = null;
    try {
      const data = await api.fetchChannelActivityCalendar(
        serverId,
        channelId,
        calendarYear,
      );
      if (calendarKey !== targetKey) return;
      calendar = data;
    } catch (error) {
      if (calendarKey !== targetKey) return;
      calendarError =
        error instanceof Error ? error.message : "Could not load activity calendar.";
    } finally {
      if (calendarKey === targetKey) calendarLoading = false;
    }
  }

  async function loadPunchcard(targetKey: string) {
    punchcardLoading = true;
    punchcardError = "";
    punchcard = null;
    try {
      const data = await api.fetchChannelMessagePunchcard(
        serverId,
        channelId,
        punchcardDays,
      );
      if (punchcardKey !== targetKey) return;
      punchcard = data;
    } catch (error) {
      if (punchcardKey !== targetKey) return;
      punchcardError =
        error instanceof Error ? error.message : "Could not load hourly activity.";
    } finally {
      if (punchcardKey === targetKey) punchcardLoading = false;
    }
  }

  function retryBundle() {
    if (canGeneral) void loadBundle(requestKey);
  }

  function retryCalendar() {
    void loadCalendar(calendarKey);
  }

  function retryPunchcard() {
    void loadPunchcard(punchcardKey);
  }

  function numberAt(point: AnalyticsPoint | ActiveUsersPoint, key: string): number {
    const value = (point as unknown as Record<string, unknown>)[key];
    return typeof value === "number" && Number.isFinite(value) ? value : 0;
  }

  function seriesDelta(values: number[]): number | null {
    if (values.length < 2) return null;
    const first = values[0];
    const last = values[values.length - 1];
    if (first > 0) return ((last - first) / first) * 100;
    return last > 0 ? 100 : 0;
  }

  function activeDelta(
    live: { dau: number; wau: number; mau: number } | null,
    series: ActiveUsersPoint[],
    metric: ActiveMetric,
    days: number,
  ): number | null {
    if (!live || series.length <= days) return null;
    const current = live[metric];
    const previous = series[series.length - 1 - days]?.[metric];
    if (typeof previous !== "number") return null;
    if (previous > 0) return ((current - previous) / previous) * 100;
    return current > 0 ? 100 : 0;
  }

  function deltaClass(value: number | null): string {
    if (value === null || Math.abs(value) <= 0.05) return "delta neutral";
    return value > 0 ? "delta positive" : "delta negative";
  }

  function deltaIcon(value: number | null): string {
    if (value === null || Math.abs(value) <= 0.05) return "minus";
    return value > 0 ? "trendUp" : "trendDown";
  }

  function formatDelta(value: number): string {
    const absolute = Math.abs(value);
    if (absolute <= 0.05) return "0%";
    return `${absolute >= 10 ? Math.round(absolute) : absolute.toFixed(1)}%`;
  }

  function formatNumber(value: number | null | undefined): string {
    if (value === null || value === undefined || !Number.isFinite(value)) return "0";
    return value.toLocaleString();
  }

  function formatShare(value: number | null): string {
    if (value === null) return "";
    return value >= 10 ? `${Math.round(value)}%` : `${value.toFixed(1)}%`;
  }

  function sparkline(values: number[]): string {
    if (values.length < 2) return "";
    const minimum = Math.min(...values);
    const maximum = Math.max(...values);
    const span = maximum - minimum;
    return values
      .map((value, index) => {
        const x = 1.5 + (index / (values.length - 1)) * 61;
        const y = 1.5 + (span === 0 ? 0.5 : 1 - (value - minimum) / span) * 13;
        return `${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(" ");
  }

  function sparkColor(value: number | null): string {
    if (value === null || Math.abs(value) <= 0.05) return "var(--muted)";
    return value > 0 ? "#8FBF9F" : "#EF5252";
  }

  function calendarYears(createdAt: string | null | undefined): number[] {
    const current = new Date().getUTCFullYear();
    let earliest = current - 4;
    if (createdAt) {
      const created = new Date(createdAt);
      const createdYear = created.getUTCFullYear();
      if (Number.isFinite(createdYear)) earliest = Math.max(earliest, createdYear);
    }
    const result: number[] = [];
    for (let year = current; year >= earliest; year--) result.push(year);
    return result;
  }

  function periodLabel(daysAgo: number): string {
    const end = new Date();
    const start = new Date(end.getFullYear(), end.getMonth(), end.getDate());
    start.setDate(start.getDate() - daysAgo);
    if (start.getMonth() === end.getMonth() && start.getFullYear() === end.getFullYear()) {
      return `${start.getDate()} to ${end.getDate()} ${monthNames[end.getMonth()]}`;
    }
    return `${start.getDate()} ${monthNames[start.getMonth()]} to ${end.getDate()} ${
      monthNames[end.getMonth()]
    }`;
  }

  function buildCalendar(days: Array<{ date: string; count: number }>, year: number) {
    const dayMs = 86_400_000;
    const weekMs = dayMs * 7;
    const byDate = new Map(days.map((day) => [day.date, day.count]));
    const maximum = Math.max(1, ...days.map((day) => day.count));
    const today = new Date().toISOString().slice(0, 10);
    const first = new Date(Date.UTC(year, 0, 1));
    const last = new Date(Date.UTC(year, 11, 31));
    const start = new Date(first);
    start.setUTCDate(first.getUTCDate() - first.getUTCDay());
    const end = new Date(last);
    end.setUTCDate(last.getUTCDate() + (6 - last.getUTCDay()));
    const cells: Array<{
      date: string;
      count: number;
      day: number;
      week: number;
      level: number;
      future: boolean;
    }> = [];

    for (let time = start.getTime(); time <= end.getTime(); time += dayMs) {
      const date = new Date(time);
      if (date < first || date > last) continue;
      const iso = date.toISOString().slice(0, 10);
      const count = byDate.get(iso) ?? 0;
      const index = Math.round((time - start.getTime()) / dayMs);
      cells.push({
        date: iso,
        count,
        day: index % 7,
        week: Math.floor(index / 7),
        level: count === 0 ? 0 : Math.min(4, Math.ceil((count / maximum) * 4)),
        future: iso > today && !byDate.has(iso),
      });
    }

    const months: Array<{ label: string; week: number }> = [];
    let lastMonthWeek = Number.NEGATIVE_INFINITY;
    for (let month = 0; month < 12; month++) {
      const monthStart = Date.UTC(year, month, 1);
      const week = Math.floor((monthStart - start.getTime()) / weekMs);
      if (week - lastMonthWeek >= 3) {
        months.push({ label: monthNames[month], week });
        lastMonthWeek = week;
      }
    }

    return {
      cells,
      weeks: Math.round((end.getTime() - start.getTime() + dayMs) / weekMs),
      total: days.reduce((sum, day) => sum + day.count, 0),
      months,
    };
  }

  type PunchcardSlot = { count: number; future: boolean };
  type PunchcardRow = { label: string; tooltipDay: string; slots: PunchcardSlot[] };

  function buildPunchcard(data: ActivityPunchcard | null) {
    const now = new Date();
    const thisWeek = punchcardDays <= 8;
    const bySlot = new Map<string, number>();
    const weekdayMatrix = Array.from({ length: 7 }, () => Array<number>(24).fill(0));

    for (const cell of data?.cells ?? []) {
      const [year, month, day] = cell.date.split("-").map(Number);
      if (!year || !month || !day) continue;
      const local = new Date(Date.UTC(year, month - 1, day, cell.hour));
      const key = `${local.getFullYear()}-${String(local.getMonth() + 1).padStart(2, "0")}-${String(
        local.getDate(),
      ).padStart(2, "0")}`;
      const slotKey = `${key}:${local.getHours()}`;
      bySlot.set(slotKey, (bySlot.get(slotKey) ?? 0) + cell.count);
      weekdayMatrix[local.getDay()][local.getHours()] += cell.count;
    }

    let rows: PunchcardRow[];
    if (thisWeek) {
      const start = new Date(now);
      start.setHours(0, 0, 0, 0);
      start.setDate(start.getDate() - start.getDay());
      rows = Array.from({ length: 7 }, (_, index) => {
        const date = new Date(start);
        date.setDate(start.getDate() + index);
        const key = `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(
          2,
          "0",
        )}-${String(date.getDate()).padStart(2, "0")}`;
        const slots = Array.from({ length: 24 }, (_, hour) => {
          const slotTime = new Date(date);
          slotTime.setHours(hour, 0, 0, 0);
          return {
            count: bySlot.get(`${key}:${hour}`) ?? 0,
            future: slotTime.getTime() > now.getTime(),
          };
        });
        return {
          label: weekdayShort[index],
          tooltipDay: date.toLocaleDateString(undefined, {
            day: "numeric",
            month: "short",
          }),
          slots,
        };
      });
    } else {
      rows = weekdayLong.map((label, index) => ({
        label: weekdayShort[index],
        tooltipDay: `All ${label}s`,
        slots: weekdayMatrix[index].map((count) => ({ count, future: false })),
      }));
    }

    let maximum = 0;
    let total = 0;
    for (const row of rows) {
      for (const slot of row.slots) {
        if (slot.future) continue;
        maximum = Math.max(maximum, slot.count);
        total += slot.count;
      }
    }

    return {
      rows,
      maximum: Math.max(1, maximum),
      total,
      caption: thisWeek
        ? `${total.toLocaleString()} ${total === 1 ? "message" : "messages"} this week`
        : `${total.toLocaleString()} messages in the last ${punchcardDays} days`,
    };
  }

  function heatColor(level: number): string {
    if (level <= 0) return "var(--panel-3)";
    return `rgba(var(--accent-rgb), ${(0.16 + level * 0.17).toFixed(2)})`;
  }

  function punchColor(count: number, maximum: number): string {
    if (count <= 0) return "var(--panel-3)";
    return `rgba(var(--accent-rgb), ${(0.16 + Math.min(1, count / maximum) * 0.68).toFixed(2)})`;
  }

  function calendarDate(date: string): string {
    const value = new Date(`${date}T00:00:00Z`);
    return value.toLocaleDateString(undefined, {
      day: "numeric",
      month: "short",
      year: "numeric",
      timeZone: "UTC",
    });
  }

  function hourLabel(hour: number): string {
    const value = hour % 12 || 12;
    return `${value} ${hour < 12 ? "AM" : "PM"}`;
  }
</script>

<div class="stats-scroll">
  {#if showDenied}
    <div class="stats-denied">
      <span class="denied-icon"><Icon name="chart" size={22} /></span>
      <p>You don't have permission to view the stats.</p>
    </div>
  {:else if bundleError}
    <div class="stats-denied">
      <span class="denied-icon"><Icon name="alert" size={22} /></span>
      <p>{bundleError}</p>
      <button on:click={retryBundle}>Try again</button>
    </div>
  {:else if bundleLoading && !bundle}
    <div class="stats-loading"><span class="spin"></span><span>Loading stats…</span></div>
  {:else}
    <div class="stats-inner">
      <section class="stats-section">
        <h2>General Stats</h2>
        <div class="metric-grid">
          <div class="metric">
            <span class="metric-label">Members</span>
            <div class="metric-line">
              <b>{formatNumber(memberCount)}</b>
              <div class="metric-side">
                {#if memberSpark.length > 1}
                  <svg class="spark" width="64" height="16" viewBox="0 0 64 16" aria-hidden="true">
                    <polyline
                      points={sparkline(memberSpark)}
                      fill="none"
                      stroke={sparkColor(memberDelta)}
                      stroke-width="1.5"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                    />
                  </svg>
                {/if}
                {#if memberDelta !== null}
                  <span class={deltaClass(memberDelta)} title="Change over the last 7 days">
                    <Icon name={deltaIcon(memberDelta)} size={12} />
                    {formatDelta(memberDelta)}
                  </span>
                {/if}
              </div>
            </div>
          </div>

          <div class="metric">
            <span class="metric-label">Online</span>
            <div class="metric-line">
              <b>{formatNumber(onlineCount)}</b>
              <div class="metric-side">
                {#if onlineSpark.length > 1}
                  <svg class="spark" width="64" height="16" viewBox="0 0 64 16" aria-hidden="true">
                    <polyline
                      points={sparkline(onlineSpark)}
                      fill="none"
                      stroke={sparkColor(onlineDelta)}
                      stroke-width="1.5"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                    />
                  </svg>
                {/if}
                {#if onlineDelta !== null}
                  <span class={deltaClass(onlineDelta)} title="Change over the last 7 days">
                    <Icon name={deltaIcon(onlineDelta)} size={12} />
                    {formatDelta(onlineDelta)}
                  </span>
                {/if}
              </div>
            </div>
          </div>

          <div class="metric">
            <span class="metric-label">In voice</span>
            <div class="metric-line">
              <b>{formatNumber(voiceCount)}</b>
              <div class="metric-side">
                {#if voiceSpark.length > 1}
                  <svg class="spark" width="64" height="16" viewBox="0 0 64 16" aria-hidden="true">
                    <polyline
                      points={sparkline(voiceSpark)}
                      fill="none"
                      stroke={sparkColor(voiceDelta)}
                      stroke-width="1.5"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                    />
                  </svg>
                {/if}
                {#if voiceDelta !== null}
                  <span class={deltaClass(voiceDelta)} title="Change over the last 7 days">
                    <Icon name={deltaIcon(voiceDelta)} size={12} />
                    {formatDelta(voiceDelta)}
                  </span>
                {/if}
              </div>
            </div>
          </div>
        </div>

        <div class="chart-block">
          <div class="chart-head">
            <h3>Activity</h3>
            <div class="chart-controls">
              <select bind:value={activityMetric} aria-label="Activity data">
                {#each activityMetrics as option (option.value)}
                  <option value={option.value}>{option.label}</option>
                {/each}
              </select>
              <select bind:value={activityScope} aria-label="Activity time span">
                {#each activityScopes as option (option.value)}
                  <option value={option.value}>{option.label}</option>
                {/each}
              </select>
            </div>
          </div>
          <StatsChart
            points={activityPoints}
            color={activityColor}
            scope={activityScope}
            label={activityLabel}
            loading={bundleLoading}
            error=""
          />
        </div>
      </section>

      {#if showElevated}
        <section class="stats-section elevated">
          <h2>Elevated Stats</h2>
          <div class="metric-grid elevated-grid">
            <div class="metric elevated-metric">
              <div class="metric-top">
                <span class="metric-label">Daily active</span>
                <span class="period">Today</span>
              </div>
              <div class="metric-line">
                <b>{formatNumber(liveActive?.dau)}</b>
                <div class="elevated-side">
                  {#if dauShare !== null}
                    <span class="share" title={`${formatNumber(liveActive?.dau)} of ${formatNumber(memberCount)} members active`}>{formatShare(dauShare)}</span>
                  {/if}
                  {#if dauDelta !== null}
                    <span class={deltaClass(dauDelta)} title="Change compared to yesterday">
                      <Icon name={deltaIcon(dauDelta)} size={12} />
                      {formatDelta(dauDelta)}
                    </span>
                  {/if}
                </div>
              </div>
            </div>

            <div class="metric elevated-metric">
              <div class="metric-top">
                <span class="metric-label">Weekly active</span>
                <span class="period">{weeklyPeriod}</span>
              </div>
              <div class="metric-line">
                <b>{formatNumber(liveActive?.wau)}</b>
                <div class="elevated-side">
                  {#if wauShare !== null}
                    <span class="share" title={`${formatNumber(liveActive?.wau)} of ${formatNumber(memberCount)} members active`}>{formatShare(wauShare)}</span>
                  {/if}
                  {#if wauDelta !== null}
                    <span class={deltaClass(wauDelta)} title="Change compared to last week">
                      <Icon name={deltaIcon(wauDelta)} size={12} />
                      {formatDelta(wauDelta)}
                    </span>
                  {/if}
                </div>
              </div>
            </div>

            <div class="metric elevated-metric">
              <div class="metric-top">
                <span class="metric-label">Monthly active</span>
                <span class="period">{monthlyPeriod}</span>
              </div>
              <div class="metric-line">
                <b>{formatNumber(liveActive?.mau)}</b>
                <div class="elevated-side">
                  {#if mauShare !== null}
                    <span class="share" title={`${formatNumber(liveActive?.mau)} of ${formatNumber(memberCount)} members active`}>{formatShare(mauShare)}</span>
                  {/if}
                  {#if mauDelta !== null}
                    <span class={deltaClass(mauDelta)} title="Change compared to last month">
                      <Icon name={deltaIcon(mauDelta)} size={12} />
                      {formatDelta(mauDelta)}
                    </span>
                  {/if}
                </div>
              </div>
            </div>
          </div>

          <div class="chart-block">
            <div class="chart-head">
              <h3>Active users</h3>
              <div class="chart-controls">
                <select bind:value={activeMetric} aria-label="Active users data">
                  {#each activeMetrics as option (option.value)}
                    <option value={option.value}>{option.label}</option>
                  {/each}
                </select>
                <select bind:value={activeScope} aria-label="Active users time span">
                  {#each activeScopes as option (option.value)}
                    <option value={option.value}>{option.label}</option>
                  {/each}
                </select>
              </div>
            </div>
            <StatsChart
              points={activePoints}
              color={colors[activeMetric]}
              scope={activeScope}
              label={activeMetrics.find((option) => option.value === activeMetric)?.label ?? "Active users"}
              loading={bundleLoading}
              error=""
            />
          </div>

          <div class="chart-block calendar-block">
            <div class="chart-head">
              <h3>Activity calendar</h3>
              <div class="chart-controls">
                <select bind:value={calendarYear} aria-label="Calendar year">
                  {#each years as year (year)}
                    <option value={year}>{year}</option>
                  {/each}
                </select>
              </div>
            </div>

            {#if calendarError}
              <div class="chart-error">
                <span>{calendarError}</span>
                <button on:click={retryCalendar}>Try again</button>
              </div>
            {:else if calendarLoading && !calendar}
              <div class="chart-loading">Loading calendar…</div>
            {:else if calendarModel.cells.length === 0}
              <div class="chart-loading">No activity recorded for this year.</div>
            {:else}
              <div class="calendar-shell">
                <div
                  class="calendar-months"
                  style={`grid-template-columns: repeat(${calendarModel.weeks}, 12px);`}
                >
                  {#each calendarModel.months as month}
                    <span style={`grid-column: ${month.week + 1}`}>{month.label}</span>
                  {/each}
                </div>
                <div class="calendar-main">
                  <div class="calendar-weekdays">
                    {#each weekdayLabels as label}
                      <span>{label}</span>
                    {/each}
                  </div>
                  <div
                    class="calendar-grid"
                    style={`grid-template-columns: repeat(${calendarModel.weeks}, 12px); grid-template-rows: repeat(7, 12px);`}
                  >
                    {#each calendarModel.cells as cell (cell.date)}
                      {#if cell.future}
                        <span
                          class="heat-cell future"
                          style={`grid-column: ${cell.week + 1}; grid-row: ${cell.day + 1};`}
                          aria-hidden="true"
                        ></span>
                      {:else}
                        <button
                          class="heat-cell"
                          style={`grid-column: ${cell.week + 1}; grid-row: ${cell.day + 1}; background: ${heatColor(cell.level)};`}
                          title={`${calendarDate(cell.date)}: ${cell.count} active`}
                          aria-label={`${calendarDate(cell.date)}: ${cell.count} active`}
                        ></button>
                      {/if}
                    {/each}
                  </div>
                </div>
                <div class="heat-foot">
                  <span>{formatNumber(calendarModel.total)} active user days in {calendarYear}</span>
                  <span class="heat-legend"><span>Less</span>{#each [0, 1, 2, 3, 4] as level}<i style={`background:${heatColor(level)}`}></i>{/each}<span>More</span></span>
                </div>
              </div>
            {/if}
          </div>

          <div class="chart-block">
            <div class="chart-head">
              <h3>Hourly activity</h3>
              <div class="chart-controls">
                <select bind:value={punchcardDays} aria-label="Hourly activity time span">
                  {#each punchcardOptions as option (option.value)}
                    <option value={option.value}>{option.label}</option>
                  {/each}
                </select>
              </div>
            </div>

            {#if punchcardError}
              <div class="chart-error">
                <span>{punchcardError}</span>
                <button on:click={retryPunchcard}>Try again</button>
              </div>
            {:else if punchcardLoading && !punchcard}
              <div class="chart-loading">Loading hourly activity…</div>
            {:else}
              <div class="punchcard-shell">
                <div class="punch-hours">
                  {#each [0, 3, 6, 9, 12, 15, 18, 21] as hour}
                    <span style={`grid-column: ${hour + 1}`}>{hourLabel(hour)}</span>
                  {/each}
                </div>
                <div class="punch-main">
                  <div class="punch-days">
                    {#each punchcardModel.rows as row}
                      <span>{row.label}</span>
                    {/each}
                  </div>
                  <div class="punch-grid">
                    {#each punchcardModel.rows as row}
                      {#each row.slots as slot, hour}
                        <button
                          class="heat-cell"
                          disabled={slot.future}
                          style={`background:${slot.future ? "var(--panel)" : punchColor(slot.count, punchcardModel.maximum)};${slot.future ? "opacity:0.4;" : ""}`}
                          title={`${row.tooltipDay} from ${hourLabel(hour)} to ${hourLabel((hour + 1) % 24)}: ${slot.count} messages`}
                          aria-label={`${row.tooltipDay} from ${hourLabel(hour)} to ${hourLabel((hour + 1) % 24)}: ${slot.count} messages`}
                        ></button>
                      {/each}
                    {/each}
                  </div>
                </div>
                <div class="heat-foot">
                  <span>{punchcardModel.caption}</span>
                  <span class="heat-legend"><span>Less</span>{#each [0, 1, 2, 3, 4] as level}<i style={`background:${punchColor((punchcardModel.maximum * level) / 4, punchcardModel.maximum)}`}></i>{/each}<span>More</span></span>
                </div>
              </div>
            {/if}
          </div>
        </section>
      {/if}
    </div>
  {/if}
</div>

<style>
  .stats-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    background: var(--chat);
  }
  .stats-inner {
    width: 100%;
    max-width: 896px;
    margin: 0 auto;
    padding: 34px 22px 64px;
    display: flex;
    flex-direction: column;
    gap: 46px;
  }
  .stats-section {
    display: flex;
    flex-direction: column;
    gap: 22px;
  }
  .stats-section h2 {
    margin: 0;
    font-size: 19px;
    line-height: 1.2;
    font-weight: 680;
    letter-spacing: -0.035em;
    color: var(--text);
  }
  .metric-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    border-top: 1px solid var(--line);
    border-bottom: 1px solid var(--line);
  }
  .metric {
    min-width: 0;
    padding: 15px 18px;
    display: flex;
    flex-direction: column;
    gap: 9px;
  }
  .metric + .metric {
    border-left: 1px solid var(--line);
  }
  .metric-label {
    font-size: 12px;
    font-weight: 560;
    color: var(--text-dim);
  }
  .metric-line {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 14px;
    min-width: 0;
  }
  .metric-line b {
    font-size: 29px;
    line-height: 1;
    font-weight: 650;
    letter-spacing: -0.04em;
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }
  .metric-side {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 5px;
    flex: none;
  }
  .spark {
    display: block;
  }
  .delta {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    font-size: 10px;
    font-weight: 600;
    line-height: 1;
    color: var(--muted);
  }
  .delta.positive {
    color: #8fbf9f;
  }
  .delta.negative {
    color: #ef5252;
  }
  .chart-block {
    display: flex;
    flex-direction: column;
    gap: 13px;
    padding-top: 6px;
  }
  .chart-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  .chart-head h3 {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: -0.01em;
    color: var(--text-dim);
  }
  .chart-controls {
    display: flex;
    gap: 7px;
    flex: none;
  }
  select {
    height: 30px;
    max-width: 130px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--panel-2);
    color: var(--text-dim);
    font-size: 10.5px;
    font-weight: 550;
    padding: 0 8px;
    outline: none;
  }
  select:hover,
  select:focus {
    border-color: var(--line-strong);
    color: var(--text);
  }
  .elevated-metric {
    gap: 10px;
  }
  .metric-top {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .period {
    font-size: 10px;
    color: var(--muted);
  }
  .elevated-side {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 5px;
    flex: none;
  }
  .share {
    font-size: 10px;
    font-weight: 600;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .chart-loading,
  .chart-error {
    min-height: 150px;
    border: 1px solid var(--line);
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    color: var(--muted);
    font-size: 11px;
    text-align: center;
  }
  .chart-error button,
  .stats-denied button {
    border: 1px solid var(--line-strong);
    border-radius: 7px;
    background: var(--panel-2);
    color: var(--text-dim);
    font-size: 10.5px;
    padding: 5px 9px;
  }
  .chart-error button:hover,
  .stats-denied button:hover {
    color: var(--text);
    background: var(--panel-3);
  }
  .calendar-shell {
    display: flex;
    flex-direction: column;
    gap: 8px;
    overflow-x: auto;
    padding: 2px 0 4px;
  }
  .calendar-months {
    display: grid;
    gap: 2px;
    margin-left: 38px;
    min-width: max-content;
    height: 14px;
  }
  .calendar-months span {
    font-size: 9px;
    color: var(--muted);
    white-space: nowrap;
  }
  .calendar-main {
    display: flex;
    gap: 8px;
    min-width: max-content;
  }
  .calendar-weekdays {
    width: 30px;
    display: grid;
    grid-template-rows: repeat(7, 12px);
    gap: 2px;
    flex: none;
  }
  .calendar-weekdays span {
    font-size: 8.5px;
    line-height: 12px;
    color: var(--muted);
    text-align: right;
  }
  .calendar-grid {
    display: grid;
    gap: 2px;
    grid-auto-flow: row;
  }
  .heat-cell {
    width: 12px;
    height: 12px;
    padding: 0;
    border: 0;
    border-radius: 2.5px;
    background: var(--panel-3);
    cursor: default;
    outline: none;
  }
  .heat-cell.future {
    background: var(--panel-2);
    opacity: 0.55;
  }
  .heat-cell:not(.future):hover,
  .heat-cell:not(.future):focus-visible {
    box-shadow: 0 0 0 1.5px var(--text);
    position: relative;
    z-index: 2;
  }
  .heat-foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
    margin-left: 38px;
    margin-top: 3px;
    font-size: 9.5px;
    color: var(--muted);
  }
  .heat-legend {
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }
  .heat-legend i {
    display: block;
    width: 11px;
    height: 11px;
    border-radius: 2.5px;
  }
  .heat-legend span {
    margin: 0 3px;
  }
  .punchcard-shell {
    display: flex;
    flex-direction: column;
    gap: 7px;
    overflow-x: auto;
    padding: 2px 0 4px;
  }
  .punch-hours {
    display: grid;
    grid-template-columns: repeat(24, minmax(13px, 1fr));
    gap: 2px;
    margin-left: 38px;
    min-width: 430px;
  }
  .punch-hours span {
    font-size: 8px;
    color: var(--muted);
    white-space: nowrap;
    overflow: visible;
  }
  .punch-main {
    display: flex;
    gap: 8px;
    min-width: 430px;
  }
  .punch-days {
    width: 30px;
    display: grid;
    grid-template-rows: repeat(7, 15px);
    gap: 2px;
    flex: none;
  }
  .punch-days span {
    font-size: 8.5px;
    line-height: 15px;
    color: var(--muted);
    text-align: right;
  }
  .punch-grid {
    flex: 1;
    display: grid;
    grid-template-columns: repeat(24, minmax(13px, 1fr));
    grid-template-rows: repeat(7, 15px);
    gap: 2px;
  }
  .punch-grid .heat-cell {
    width: 100%;
    height: 15px;
    border-radius: 2.5px;
  }
  .punch-grid .heat-cell:disabled {
    cursor: default;
  }
  .stats-loading,
  .stats-denied {
    min-height: 260px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 11px;
    padding: 30px;
    text-align: center;
    color: var(--muted);
    font-size: 11.5px;
  }
  .stats-loading {
    flex-direction: row;
  }
  .stats-denied p {
    margin: 0;
  }
  .denied-icon {
    width: 44px;
    height: 44px;
    display: grid;
    place-items: center;
    border: 1px solid var(--line);
    border-radius: 13px;
    background: var(--panel-2);
    color: var(--muted);
  }
  .spin {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 2px solid var(--line-strong);
    border-top-color: var(--accent);
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (max-width: 720px) {
    .stats-inner {
      padding: 26px 15px 52px;
      gap: 36px;
    }
    .metric-grid {
      grid-template-columns: 1fr;
    }
    .metric + .metric {
      border-left: 0;
      border-top: 1px solid var(--line);
    }
    .metric-line b {
      font-size: 25px;
    }
  }
</style>
