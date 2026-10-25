<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { notify } from "../workspace";

  export let src: string;
  export let maxHeight = 300;
  export let showPip = false;
  export let onLoaded: (() => void) | undefined = undefined;

  let container: HTMLDivElement;
  let video: HTMLVideoElement;
  let track: HTMLDivElement;
  let volumeTrack: HTMLDivElement | undefined;

  let playing = false;
  let ended = false;
  let duration = 0;
  let current = 0;
  let buffered = 0;
  let muted = false;
  let volume = 1;
  let controlsVisible = true;
  let scrubbing = false;
  let volumeDragging = false;
  let hoveringControls = false;
  let focused = false;
  let fullscreen = false;
  let hideTimer: ReturnType<typeof setTimeout> | undefined;
  let clickTimer: ReturnType<typeof setTimeout> | undefined;

  $: progress = duration > 0 ? Math.min(100, (current / duration) * 100) : 0;
  $: volumePercent = muted ? 0 : Math.min(100, Math.max(0, volume * 100));

  function clamp(value: number, min: number, max: number) {
    return Math.max(min, Math.min(value, max));
  }

  function formatTime(value: number): string {
    if (!Number.isFinite(value) || value < 0) return "0:00";
    const total = Math.floor(value);
    const hours = Math.floor(total / 3600);
    const minutes = Math.floor((total % 3600) / 60);
    const seconds = total % 60;
    return hours > 0
      ? `${hours}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`
      : `${minutes}:${String(seconds).padStart(2, "0")}`;
  }

  function clearHideTimer() {
    if (hideTimer) {
      clearTimeout(hideTimer);
      hideTimer = undefined;
    }
  }

  function scheduleHide() {
    clearHideTimer();
    if (!playing || scrubbing || hoveringControls || focused) return;
    hideTimer = setTimeout(() => {
      if (!video?.paused && !scrubbing && !hoveringControls && !focused) {
        controlsVisible = false;
      }
    }, 2200);
  }

  function revealControls() {
    controlsVisible = true;
    scheduleHide();
  }

  function onMouseLeave() {
    if (scrubbing || volumeDragging) return;
    if (!playing || focused) return;
    controlsVisible = false;
    clearHideTimer();
  }

  function togglePlay() {
    if (!video) return;
    if (video.paused) {
      if (video.ended) video.currentTime = 0;
      void video.play().catch(() => {});
    } else {
      video.pause();
    }
  }

  function onVideoClick() {
    if (clickTimer) {
      clearTimeout(clickTimer);
      clickTimer = undefined;
      return;
    }
    clickTimer = setTimeout(() => {
      clickTimer = undefined;
      togglePlay();
    }, 180);
  }

  function toggleFullscreen() {
    if (!container) return;
    if (document.fullscreenElement) {
      void document.exitFullscreen().catch(() => {});
    } else {
      void container.requestFullscreen().catch(() => {});
    }
  }

  function onVideoDoubleClick(event: MouseEvent) {
    event.preventDefault();
    if (clickTimer) {
      clearTimeout(clickTimer);
      clickTimer = undefined;
    }
    toggleFullscreen();
  }

  function toggleMute() {
    if (!video) return;
    video.muted = !video.muted;
  }

  async function togglePictureInPicture() {
    if (!video) return;
    try {
      if (document.pictureInPictureElement) await document.exitPictureInPicture();
      else await video.requestPictureInPicture();
    } catch {
      notify("Picture in picture is not available for this video.");
    }
  }

  function seekFromPointer(event: PointerEvent) {
    if (!video || !track || !Number.isFinite(video.duration) || video.duration <= 0) return;
    const rect = track.getBoundingClientRect();
    if (rect.width <= 0) return;
    const ratio = clamp((event.clientX - rect.left) / rect.width, 0, 1);
    video.currentTime = ratio * video.duration;
    current = video.currentTime;
  }

  function onSeekDown(event: PointerEvent) {
    event.preventDefault();
    scrubbing = true;
    controlsVisible = true;
    clearHideTimer();
    track.setPointerCapture(event.pointerId);
    seekFromPointer(event);
  }

  function onSeekMove(event: PointerEvent) {
    if (!scrubbing) return;
    seekFromPointer(event);
  }

  function onSeekUp(event: PointerEvent) {
    if (!scrubbing) return;
    scrubbing = false;
    if (track.hasPointerCapture(event.pointerId)) track.releasePointerCapture(event.pointerId);
    scheduleHide();
  }

  function volumeFromPointer(event: PointerEvent) {
    if (!video || !volumeTrack) return;
    const rect = volumeTrack.getBoundingClientRect();
    if (rect.width <= 0) return;
    const next = clamp((event.clientX - rect.left) / rect.width, 0, 1);
    video.volume = next;
    video.muted = next === 0;
  }

  function onVolumeDown(event: PointerEvent) {
    event.preventDefault();
    if (!volumeTrack) return;
    volumeDragging = true;
    clearHideTimer();
    volumeTrack.setPointerCapture(event.pointerId);
    volumeFromPointer(event);
  }

  function onVolumeMove(event: PointerEvent) {
    if (!volumeDragging) return;
    volumeFromPointer(event);
  }

  function onVolumeUp(event: PointerEvent) {
    if (!volumeDragging || !volumeTrack) return;
    volumeDragging = false;
    if (volumeTrack.hasPointerCapture(event.pointerId)) {
      volumeTrack.releasePointerCapture(event.pointerId);
    }
    scheduleHide();
  }

  function updateBuffered() {
    if (!video || !Number.isFinite(video.duration) || video.duration <= 0) {
      buffered = 0;
      return;
    }
    try {
      if (video.buffered.length === 0) {
        buffered = 0;
        return;
      }
      buffered = clamp((video.buffered.end(video.buffered.length - 1) / video.duration) * 100, 0, 100);
    } catch {
      buffered = 0;
    }
  }

  function onKeyDown(event: KeyboardEvent) {
    if ((event.target as HTMLElement).closest("button")) return;
    if (!video) return;
    switch (event.key) {
      case " ":
      case "k":
        event.preventDefault();
        togglePlay();
        break;
      case "f":
        event.preventDefault();
        toggleFullscreen();
        break;
      case "m":
        event.preventDefault();
        toggleMute();
        break;
      case "ArrowLeft":
        event.preventDefault();
        video.currentTime = Math.max(0, video.currentTime - 5);
        revealControls();
        break;
      case "ArrowRight":
        event.preventDefault();
        video.currentTime = Math.min(video.duration || 0, video.currentTime + 5);
        revealControls();
        break;
      case "p":
        if (showPip) {
          event.preventDefault();
          void togglePictureInPicture();
        }
        break;
    }
  }

  function onMetadata() {
    duration = Number.isFinite(video.duration) ? video.duration : 0;
    updateBuffered();
    onLoaded?.();
  }

  function onFullscreenChange() {
    fullscreen = document.fullscreenElement === container;
    if (fullscreen) {
      controlsVisible = true;
      clearHideTimer();
    } else {
      scheduleHide();
    }
  }

  onMount(() => {
    document.addEventListener("fullscreenchange", onFullscreenChange);
    return () => document.removeEventListener("fullscreenchange", onFullscreenChange);
  });

  onDestroy(() => {
    clearHideTimer();
    if (clickTimer) clearTimeout(clickTimer);
  });
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex a11y_no_static_element_interactions -->
<div
  bind:this={container}
  class="player"
  class:fullscreen
  style={`--max-video-height:${maxHeight}px`}
  tabindex="0"
  aria-label="Video player. Press Space to play or pause, F for fullscreen, M to mute, and the arrow keys to seek."
  on:keydown={onKeyDown}
  on:focus={() => {
    focused = true;
    controlsVisible = true;
    clearHideTimer();
  }}
  on:blur={() => {
    focused = false;
    scheduleHide();
  }}
  on:mousemove={revealControls}
  on:mouseleave={onMouseLeave}
>
  <!-- svelte-ignore a11y_media_has_caption -->
  <video
    bind:this={video}
    {src}
    preload="metadata"
    playsinline
    on:click={onVideoClick}
    on:dblclick={onVideoDoubleClick}
    on:loadedmetadata={onMetadata}
    on:durationchange={onMetadata}
    on:timeupdate={() => {
      current = video?.currentTime ?? 0;
      if (video?.ended) ended = true;
    }}
    on:progress={updateBuffered}
    on:play={() => {
      playing = true;
      ended = false;
      scheduleHide();
    }}
    on:pause={() => {
      playing = false;
      controlsVisible = true;
      clearHideTimer();
    }}
    on:ended={() => {
      playing = false;
      ended = true;
      controlsVisible = true;
      clearHideTimer();
    }}
    on:volumechange={() => {
      if (!video) return;
      muted = video.muted || video.volume === 0;
      volume = video.volume;
    }}
  ></video>

  {#if !playing}
    <button
      class="play-overlay"
      type="button"
      aria-label="Play video"
      on:click={onVideoClick}
      on:dblclick={onVideoDoubleClick}
    >
      <span class="play-circle">
        <svg viewBox="0 0 24 24" width="27" height="27" aria-hidden="true">
          <path
            fill="currentColor"
            d="M4.5 5.653c0-1.427 1.529-2.33 2.779-1.643l11.54 6.347c1.295.712 1.295 2.573 0 3.286L7.28 19.99c-1.25.687-2.779-.217-2.779-1.643V5.653Z"
          />
        </svg>
      </span>
    </button>
  {/if}

  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="controls"
    class:visible={controlsVisible}
    on:mouseenter={() => {
      hoveringControls = true;
      controlsVisible = true;
      clearHideTimer();
    }}
    on:mouseleave={() => {
      hoveringControls = false;
      scheduleHide();
    }}
  >
    <div class="controls-inner">
      <div
        bind:this={track}
        class="progress"
        role="slider"
        aria-label="Seek"
        aria-valuemin="0"
        aria-valuemax={duration || 0}
        aria-valuenow={current}
        tabindex="-1"
        on:pointerdown={onSeekDown}
        on:pointermove={onSeekMove}
        on:pointerup={onSeekUp}
        on:pointercancel={onSeekUp}
      >
        <div class="progress-rail">
          <div class="progress-buffered" style={`width:${buffered}%`}></div>
          <div class="progress-played" style={`width:${progress}%`}></div>
          <div class="progress-knob" style={`left:${progress}%`}></div>
        </div>
      </div>

      <div class="control-row">
        <button type="button" aria-label={playing ? "Pause" : "Play"} on:click={togglePlay}>
          {#if playing}
            <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true">
              <path fill="currentColor" d="M7.5 5h3v14h-3zM13.5 5h3v14h-3z" />
            </svg>
          {:else}
            <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true">
              <path
                fill="currentColor"
                d="M4.5 5.653c0-1.427 1.529-2.33 2.779-1.643l11.54 6.347c1.295.712 1.295 2.573 0 3.286L7.28 19.99c-1.25.687-2.779-.217-2.779-1.643V5.653Z"
              />
            </svg>
          {/if}
        </button>

        <span class="time">{formatTime(current)} / {formatTime(duration)}</span>
        <span class="spacer"></span>

        <div class="volume-wrap" class:muted>
          <button type="button" aria-label={muted ? "Unmute" : "Mute"} on:click={toggleMute}>
            <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true">
              <path
                fill="currentColor"
                d="M11 5 6 9H3v6h3l5 4V5Zm6.03 1.72a.75.75 0 0 1 1.06 0 7.5 7.5 0 0 1 0 10.6.75.75 0 0 1-1.06-1.06 6 6 0 0 0 0-8.48.75.75 0 0 1 0-1.06Z"
              />
              {#if muted}
                <path
                  fill="none"
                  stroke="currentColor"
                  stroke-width="1.8"
                  stroke-linecap="round"
                  d="m4 4 16 16"
                />
              {/if}
            </svg>
          </button>
          <div
            bind:this={volumeTrack}
            class="volume-track"
            role="slider"
            aria-label="Volume"
            aria-valuemin="0"
            aria-valuemax="100"
            aria-valuenow={Math.round(volumePercent)}
            tabindex="-1"
            on:pointerdown={onVolumeDown}
            on:pointermove={onVolumeMove}
            on:pointerup={onVolumeUp}
            on:pointercancel={onVolumeUp}
          >
            <div class="volume-rail">
              <div class="volume-fill" style={`width:${volumePercent}%`}></div>
              <div class="volume-knob" style={`left:${volumePercent}%`}></div>
            </div>
          </div>
        </div>

        {#if showPip}
          <button type="button" aria-label="Picture in picture" on:click={togglePictureInPicture}>
            <svg viewBox="0 0 24 24" width="19" height="19" aria-hidden="true">
              <path
                fill="none"
                stroke="currentColor"
                stroke-width="1.7"
                stroke-linejoin="round"
                d="M3 5h18v14H3zM12 12h6v4h-6z"
              />
            </svg>
          </button>
        {/if}

        <button type="button" aria-label="Fullscreen" on:click={toggleFullscreen}>
          <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true">
            <path
              fill="none"
              stroke="currentColor"
              stroke-width="1.7"
              stroke-linecap="round"
              stroke-linejoin="round"
              d="M8 3H5a2 2 0 0 0-2 2v3M16 3h3a2 2 0 0 1 2 2v3M8 21H5a2 2 0 0 1-2-2v-3M16 21h3a2 2 0 0 0 2-2v-3"
            />
          </svg>
        </button>
      </div>
    </div>
  </div>
</div>

<style>
  .player {
    position: relative;
    display: block;
    width: 100%;
    max-width: 400px;
    overflow: hidden;
    border-radius: 8px;
    background: #000;
    color: #fff;
    cursor: pointer;
    outline: none;
    user-select: none;
  }
  .player:focus-visible {
    box-shadow: 0 0 0 2px rgba(var(--accent-rgb), 0.5);
  }
  .player.fullscreen {
    width: 100vw;
    max-width: none;
    height: 100vh;
    border-radius: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  video {
    display: block;
    width: 100%;
    max-width: 100%;
    max-height: var(--max-video-height, 300px);
    object-fit: contain;
    background: #000;
    cursor: pointer;
  }
  .fullscreen video {
    width: 100%;
    height: 100%;
    max-height: none;
    object-fit: contain;
  }
  .play-overlay {
    position: absolute;
    inset: 0;
    z-index: 2;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    border: 0;
    background: transparent;
    color: #fff;
    cursor: pointer;
  }
  .play-circle {
    width: 56px;
    height: 56px;
    display: grid;
    place-items: center;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.18);
    backdrop-filter: blur(8px);
    box-shadow: 0 7px 24px rgba(0, 0, 0, 0.34);
    transition: transform 120ms ease, background 120ms ease;
  }
  .play-overlay:hover .play-circle {
    transform: scale(1.08);
    background: rgba(255, 255, 255, 0.24);
  }
  .play-circle svg {
    margin-left: 2px;
  }
  .controls {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 3;
    opacity: 0;
    pointer-events: none;
    transition: opacity 160ms ease;
  }
  .controls.visible {
    opacity: 1;
    pointer-events: auto;
  }
  .controls-inner {
    padding: 7px 10px 8px;
    background: rgba(8, 9, 13, 0.84);
    backdrop-filter: blur(7px);
    border-top: 1px solid rgba(255, 255, 255, 0.07);
  }
  .progress {
    height: 16px;
    display: flex;
    align-items: center;
    margin-bottom: 2px;
    cursor: pointer;
    touch-action: none;
  }
  .progress-rail {
    position: relative;
    width: 100%;
    height: 4px;
    border-radius: 99px;
    background: rgba(255, 255, 255, 0.16);
    transition: height 100ms ease;
  }
  .progress:hover .progress-rail {
    height: 6px;
  }
  .progress-buffered,
  .progress-played,
  .volume-fill {
    position: absolute;
    inset: 0 auto 0 0;
    border-radius: 99px;
    pointer-events: none;
  }
  .progress-buffered {
    background: rgba(255, 255, 255, 0.22);
  }
  .progress-played {
    background: var(--accent);
  }
  .progress-knob {
    position: absolute;
    top: 50%;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.4);
    opacity: 0;
    pointer-events: none;
    transform: translate(-50%, -50%);
    transition: opacity 100ms ease;
  }
  .progress:hover .progress-knob,
  .progress:focus-within .progress-knob {
    opacity: 1;
  }
  .control-row {
    display: flex;
    align-items: center;
    gap: 4px;
    min-height: 28px;
  }
  .control-row button {
    width: 27px;
    height: 27px;
    flex: none;
    display: grid;
    place-items: center;
    padding: 0;
    border: 0;
    border-radius: 5px;
    background: transparent;
    color: rgba(255, 255, 255, 0.82);
    cursor: pointer;
    transition: color 100ms ease, background 100ms ease;
  }
  .control-row button:hover {
    color: #fff;
    background: rgba(255, 255, 255, 0.09);
  }
  .time {
    margin-left: 2px;
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 11px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.02em;
    color: #fff;
    white-space: nowrap;
  }
  .spacer {
    flex: 1;
  }
  .volume-wrap {
    display: flex;
    align-items: center;
    gap: 0;
  }
  .volume-wrap.muted .volume-fill {
    background: rgba(255, 255, 255, 0.35);
  }
  .volume-track {
    width: 0;
    height: 22px;
    overflow: hidden;
    display: flex;
    align-items: center;
    opacity: 0;
    cursor: pointer;
    touch-action: none;
    transition: width 160ms ease, opacity 160ms ease;
  }
  .volume-wrap:hover .volume-track,
  .volume-track:focus-within {
    width: 64px;
    opacity: 1;
  }
  .volume-rail {
    position: relative;
    width: 56px;
    height: 4px;
    margin: 0 4px;
    border-radius: 99px;
    background: rgba(255, 255, 255, 0.18);
  }
  .volume-fill {
    background: var(--accent);
  }
  .volume-knob {
    position: absolute;
    top: 50%;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--accent);
    opacity: 0;
    pointer-events: none;
    transform: translate(-50%, -50%);
    transition: opacity 100ms ease;
  }
  .volume-wrap:hover .volume-knob,
  .volume-track:focus-within .volume-knob {
    opacity: 1;
  }
</style>
