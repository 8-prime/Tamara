<script>
  import { tick } from "svelte";
  import Icon from "./Icon.svelte";
  import { dateKey, dateLabel, subjectStyle } from "./teaching.js";
  import {
    timeMinutes,
    timeString,
    snapMinute,
    moveRange,
    resizeRange,
    dayLayout,
  } from "./calendar.js";

  let {
    days,
    lessons,
    selectedId = "",
    saving = false,
    onselect,
    oncreate,
    onchange,
    onedit,
  } = $props();
  let columns = $state(null),
    calendarRoot = $state(null),
    gesture = $state(null),
    focusSlot = $state(null),
    suppressClick = false;
  let captured = null;
  const pixelsPerMinute = 1.2;
  const visibleLessons = $derived(
    lessons.filter((l) => days.includes(l.lesson_date)),
  );
  const firstMinute = $derived(
    Math.min(
      7 * 60,
      ...visibleLessons.map(
        (l) => Math.floor(timeMinutes(l.starts_at) / 60) * 60,
      ),
    ),
  );
  const lastMinute = $derived(
    Math.max(
      16 * 60,
      ...visibleLessons.map((l) => Math.ceil(timeMinutes(l.ends_at) / 60) * 60),
    ),
  );
  const height = $derived((lastMinute - firstMinute) * pixelsPerMinute);
  const slots = $derived(
    Array.from(
      { length: (lastMinute - firstMinute) / 15 },
      (_, i) => firstMinute + i * 15,
    ),
  );
  const activeSlot = $derived(
    focusSlot &&
      days.includes(focusSlot.day) &&
      slots.includes(focusSlot.minute)
      ? focusSlot
      : null,
  );
  const hours = $derived(
    Array.from(
      { length: (lastMinute - firstMinute) / 60 + 1 },
      (_, i) => firstMinute + i * 60,
    ),
  );
  const layout = $derived(
    days.map((day) => ({
      day,
      items: dayLayout(visibleLessons.filter((l) => l.lesson_date === day)),
    })),
  );

  function begin(
    event,
    kind,
    day,
    start,
    lesson = null,
    explicitHandle = false,
  ) {
    suppressClick = false;
    if (
      saving ||
      event.button !== 0 ||
      (event.pointerType === "touch" && !explicitHandle)
    )
      return;
    event.preventDefault();
    event.stopPropagation();
    suppressClick = false;
    captured = event.currentTarget;
    captured.focus({ preventScroll: true });
    captured.setPointerCapture(event.pointerId);
    gesture = {
      kind,
      lesson,
      pointer: event.pointerId,
      x: event.clientX,
      y: event.clientY,
      day,
      start,
      end: lesson ? timeMinutes(lesson.ends_at) : Math.min(start + 45, 1439),
      originalStart: start,
      moved: false,
    };
  }

  function move(event) {
    if (!gesture || gesture.pointer !== event.pointerId) return;
    const distance = Math.hypot(
      event.clientX - gesture.x,
      event.clientY - gesture.y,
    );
    if (!gesture.moved && distance < 5) return;
    const rect = columns.getBoundingClientRect();
    const axis = 54;
    const dayIndex = Math.max(
      0,
      Math.min(
        days.length - 1,
        Math.floor(
          (event.clientX - rect.left - axis) /
            ((rect.width - axis) / days.length),
        ),
      ),
    );
    if (gesture.kind === "move") {
      const range = moveRange(
        gesture.originalStart,
        timeMinutes(gesture.lesson.ends_at),
        gesture.originalStart + (event.clientY - gesture.y) / pixelsPerMinute,
      );
      gesture = { ...gesture, ...range, day: days[dayIndex], moved: true };
    } else if (gesture.kind === "resize") {
      const range = resizeRange(
        gesture.originalStart,
        timeMinutes(gesture.lesson.ends_at) +
          (event.clientY - gesture.y) / pixelsPerMinute,
      );
      gesture = { ...gesture, ...range, moved: true };
    } else {
      const target = Math.max(
        firstMinute,
        Math.min(
          lastMinute,
          snapMinute(
            firstMinute + (event.clientY - rect.top) / pixelsPerMinute,
          ),
        ),
      );
      const start = Math.min(gesture.originalStart, target);
      const end = Math.min(
        1439,
        Math.max(start + 15, gesture.originalStart, target),
      );
      gesture = { ...gesture, start, end, moved: true };
    }
  }

  function releaseCapture() {
    if (captured && gesture && captured.hasPointerCapture(gesture.pointer))
      captured.releasePointerCapture(gesture.pointer);
    captured = null;
  }
  function cancel() {
    if (!gesture) return;
    releaseCapture();
    gesture = null;
    suppressClick = true;
  }
  async function finish(event) {
    if (!gesture || gesture.pointer !== event.pointerId) return;
    const done = gesture;
    releaseCapture();
    gesture = null;
    suppressClick = done.moved;
    if (!done.moved) return; // Native click remains the keyboard/touch equivalent.
    if (done.kind === "create")
      oncreate(done.day, timeString(done.start), timeString(done.end));
    else if (
      done.day !== done.lesson.lesson_date ||
      done.start !== timeMinutes(done.lesson.starts_at) ||
      done.end !== timeMinutes(done.lesson.ends_at)
    ) {
      await onchange(
        done.lesson,
        done.day,
        timeString(done.start),
        timeString(done.end),
      );
    }
  }
  function clickSlot(day, minute) {
    if (suppressClick) {
      suppressClick = false;
      return;
    }
    if (!saving)
      oncreate(
        day,
        timeString(minute),
        timeString(Math.min(minute + 45, 1439)),
      );
  }
  function slotKeyboard(event, day, minute) {
    if (
      !["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"].includes(event.key)
    )
      return;
    event.preventDefault();
    const dayIndex = Math.max(
      0,
      Math.min(
        days.length - 1,
        days.indexOf(day) +
          (event.key === "ArrowLeft" ? -1 : event.key === "ArrowRight" ? 1 : 0),
      ),
    );
    const nextMinute = Math.max(
      firstMinute,
      Math.min(
        lastMinute - 15,
        minute +
          (event.key === "ArrowUp" ? -15 : event.key === "ArrowDown" ? 15 : 0),
      ),
    );
    focusSlot = { day: days[dayIndex], minute: nextMinute };
    [...calendarRoot.querySelectorAll("button[data-minute]")]
      .find(
        (el) =>
          el.dataset.day === focusSlot.day &&
          Number(el.dataset.minute) === nextMinute,
      )
      ?.focus({ preventScroll: false });
  }
  function clickLesson(lesson) {
    if (suppressClick) {
      suppressClick = false;
      return;
    }
    if (!saving) onselect(lesson);
  }
  async function keyboard(event, lesson, kind) {
    const deltas = { ArrowUp: -15, ArrowDown: 15 };
    if (
      !(event.key in deltas) &&
      !(kind === "move" && ["ArrowLeft", "ArrowRight"].includes(event.key))
    )
      return;
    event.preventDefault();
    event.stopPropagation();
    if (saving) return;
    let day = lesson.lesson_date;
    if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
      const index = days.indexOf(day) + (event.key === "ArrowLeft" ? -1 : 1);
      if (!days[index]) return;
      day = days[index];
    }
    const start = timeMinutes(lesson.starts_at),
      end = timeMinutes(lesson.ends_at);
    const delta = deltas[event.key] || 0;
    const range =
      kind === "resize"
        ? resizeRange(start, end + delta)
        : moveRange(start, end, start + delta);
    await onchange(lesson, day, timeString(range.start), timeString(range.end));
    await tick();
    [...calendarRoot.querySelectorAll("button[data-lesson]")]
      .find(
        (el) => el.dataset.lesson === lesson.id && el.dataset.action === kind,
      )
      ?.focus({ preventScroll: true });
  }
  function position(item) {
    return `top:${(item.start - firstMinute) * pixelsPerMinute}px;height:${Math.max(18, (item.end - item.start) * pixelsPerMinute)}px;left:calc(${(item.lane * 100) / item.lanes}% + 3px);width:calc(${100 / item.lanes}% - 6px)`;
  }
</script>

<svelte:window
  onpointermove={move}
  onpointerup={finish}
  onpointercancel={cancel}
  onkeydown={(e) => {
    if (e.key === "Escape" && gesture) {
      e.preventDefault();
      cancel();
    }
  }}
/>

<div class="calendar-instructions">
  <span
    ><Icon name="plus" /> Freie Zeit anklicken oder einen Zeitraum aufziehen</span
  ><span>Stunden verschieben · am unteren Rand verlängern</span>
</div>
<p id="calendar-keyboard-help" class="sr-only">
  Freie Zeiten mit den Pfeiltasten wählen, mit Enter eine Stunde hinzufügen. Zum
  Verschieben den Griff fokussieren und Pfeiltasten verwenden. Oben und unten
  ändern die Zeit um 15 Minuten, links und rechts den Tag. Am unteren Griff
  ändern oben und unten die Dauer. Escape bricht Ziehen ab.
</p>
<div class="time-calendar-scroll" class:single-day={days.length === 1}>
  <div
    class="time-calendar"
    bind:this={calendarRoot}
    style={`--calendar-days:${days.length};--calendar-height:${height}px`}
  >
    <div class="time-calendar-header">
      <div class="calendar-zone">Zeit</div>
      {#each days as day}<div class:today={day === dateKey()}>
          <strong>{dateLabel(day, { weekday: "short" })}</strong><span
            >{dateLabel(day, { day: "numeric", month: "short" })}</span
          >
        </div>{/each}
    </div>
    <div class="time-calendar-columns" bind:this={columns}>
      <div class="calendar-time-axis" aria-hidden="true">
        {#each hours as minute}<span
            style={`top:${(minute - firstMinute) * pixelsPerMinute}px`}
            >{timeString(minute)}</span
          >{/each}
      </div>
      {#each layout as { day, items }}<div
          class="calendar-day-column"
          class:today={day === dateKey()}
        >
          {#each slots as minute}<button
              class="calendar-time-slot"
              class:hour={minute % 60 === 0}
              class:half={minute % 30 === 0}
              data-day={day}
              data-minute={minute}
              tabindex={(
                activeSlot
                  ? activeSlot.day === day && activeSlot.minute === minute
                  : day === days[0] && minute === firstMinute
              )
                ? 0
                : -1}
              style={`top:${(minute - firstMinute) * pixelsPerMinute}px;height:${15 * pixelsPerMinute}px`}
              aria-label={`Stunde am ${dateLabel(day)} um ${timeString(minute)} hinzufügen`}
              aria-describedby="calendar-keyboard-help"
              disabled={saving}
              onfocus={() => (focusSlot = { day, minute })}
              onkeydown={(e) => slotKeyboard(e, day, minute)}
              onpointerdown={(e) => begin(e, "create", day, minute)}
              onclick={() => clickSlot(day, minute)}
              ><span aria-hidden="true">+</span></button
            >{/each}
          {#each items as item (item.lesson.id)}{@const lesson = item.lesson}
            <article
              class={`calendar-event ${subjectStyle(lesson.subject_name)}`}
              class:selected={lesson.id === selectedId}
              class:compact={item.end - item.start < 45}
              class:short={item.end - item.start < 30}
              class:cancelled={lesson.status === "cancelled"}
              class:dragging={gesture?.lesson?.id === lesson.id &&
                gesture.moved}
              style={position(item)}
            >
              <button
                class="calendar-event-content"
                aria-label={`${lesson.subject_name}, Klasse ${lesson.class_name}, ${lesson.starts_at} bis ${lesson.ends_at}${lesson.status === "cancelled" ? ", fällt aus" : ""}`}
                disabled={saving}
                onpointerdown={(e) => {
                  if (lesson.status !== "cancelled")
                    begin(e, "move", day, item.start, lesson);
                }}
                onclick={() => clickLesson(lesson)}
                ondblclick={() => {
                  if (!saving) onedit(lesson);
                }}
                ><strong>{lesson.subject_name}</strong><span
                  >{lesson.class_name}{lesson.room
                    ? ` · ${lesson.room}`
                    : ""}</span
                ><small
                  >{lesson.starts_at}–{lesson.ends_at}{lesson.status ===
                  "cancelled"
                    ? " · Fällt aus"
                    : ""}</small
                ></button
              >
              {#if lesson.status !== "cancelled"}<button
                  class="calendar-move-handle"
                  data-lesson={lesson.id}
                  data-action="move"
                  aria-label={`${lesson.subject_name} verschieben`}
                  aria-describedby="calendar-keyboard-help"
                  disabled={saving}
                  onpointerdown={(e) =>
                    begin(e, "move", day, item.start, lesson, true)}
                  onkeydown={(e) => keyboard(e, lesson, "move")}
                  ><span aria-hidden="true">⠿</span></button
                ><button
                  class="calendar-resize-handle"
                  data-lesson={lesson.id}
                  data-action="resize"
                  aria-label={`Dauer von ${lesson.subject_name} ändern`}
                  aria-describedby="calendar-keyboard-help"
                  disabled={saving}
                  onpointerdown={(e) =>
                    begin(e, "resize", day, item.start, lesson, true)}
                  onkeydown={(e) => keyboard(e, lesson, "resize")}
                  ><span aria-hidden="true"></span></button
                >{/if}
            </article>{/each}
          {#if gesture?.moved && gesture.day === day}<div
              class="calendar-placement"
              style={`top:${(gesture.start - firstMinute) * pixelsPerMinute}px;height:${(gesture.end - gesture.start) * pixelsPerMinute}px`}
            >
              <strong>{gesture.lesson?.subject_name || "Neue Stunde"}</strong
              ><span>{timeString(gesture.start)}–{timeString(gesture.end)}</span
              >
            </div>{/if}
        </div>{/each}
    </div>
  </div>
</div>
<div class="sr-only" aria-live="polite">
  {#if gesture?.moved}{dateLabel(gesture.day)}, {timeString(gesture.start)} bis {timeString(
      gesture.end,
    )}{/if}
</div>

<style>
  .calendar-instructions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 22px;
    padding: 0 16px 13px;
    color: var(--muted);
    font-size: 12px;
  }
  .calendar-instructions span:first-child {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .time-calendar-scroll {
    overflow-x: auto;
    padding-bottom: 8px;
  }
  .time-calendar {
    min-width: 620px;
  }
  .single-day .time-calendar {
    min-width: 0;
  }
  .time-calendar-header,
  .time-calendar-columns {
    display: grid;
    grid-template-columns: 54px repeat(var(--calendar-days), minmax(0, 1fr));
  }
  .time-calendar-header {
    border-top: 1px solid var(--line);
    border-bottom: 1px solid var(--line);
    background: #fbfdff;
  }
  .time-calendar-header > div {
    text-align: center;
    padding: 12px 4px;
    font-size: 12px;
  }
  .time-calendar-header strong,
  .time-calendar-header span {
    display: block;
  }
  .time-calendar-header span {
    margin-top: 3px;
    color: var(--muted);
    font-size: 11px;
  }
  .time-calendar-header .today {
    background: #eaf5ee;
    color: #23654e;
  }
  .time-calendar-header .today span {
    color: #397358;
  }
  .time-calendar-header .calendar-zone {
    color: var(--muted);
    align-self: end;
    text-align: left;
    padding-left: 11px;
    font-size: 11px;
  }
  .time-calendar-columns {
    height: var(--calendar-height);
  }
  .calendar-time-axis {
    position: relative;
    background: #fff;
  }
  .calendar-time-axis > span {
    position: absolute;
    right: 9px;
    transform: translateY(-50%);
    font-size: 11px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .calendar-time-axis > span:first-child {
    transform: translateY(3px);
  }
  .calendar-time-axis > span:last-child {
    transform: translateY(-100%);
  }
  .calendar-day-column {
    position: relative;
    border-left: 1px solid var(--line);
    background: #fff;
    min-width: 0;
  }
  .calendar-day-column.today {
    background: #fbfefc;
  }
  .calendar-time-slot {
    position: absolute;
    left: 0;
    width: 100%;
    background: transparent;
    border: 0;
    border-radius: 0;
    padding: 0;
    color: #438065;
  }
  .calendar-time-slot.half {
    border-top: 1px dotted #e7edf2;
  }
  .calendar-time-slot.hour {
    border-top: 1px solid #e1eaf1;
  }
  .calendar-time-slot:first-child {
    border-top: 0;
  }
  .calendar-time-slot span {
    display: none;
    font-size: 19px;
  }
  .calendar-time-slot:hover,
  .calendar-time-slot:focus-visible {
    background: #edf6f0;
  }
  .calendar-time-slot:hover span,
  .calendar-time-slot:focus-visible span {
    display: block;
  }
  .calendar-event {
    position: absolute;
    z-index: 1;
    border-radius: 6px;
    border: 1px solid #b9d9c3;
    border-left: 3px solid #5a9d72;
    background: #e6f3e9;
    color: #194e3d;
    box-shadow: 0 1px 2px #17334d0b;
    min-width: 0;
  }
  .calendar-event.blue {
    background: #e6f1ff;
    border-color: #bdd6f3;
    border-left-color: #5c9cdc;
    color: #234a79;
  }
  .calendar-event.rose {
    background: #fbeaf0;
    border-color: #ebc5d2;
    border-left-color: #cd7899;
    color: #75334e;
  }
  .calendar-event.orange {
    background: #fff0dc;
    border-color: #efd6b5;
    border-left-color: #d4a063;
    color: #6c4c25;
  }
  .calendar-event.purple {
    background: #eee9fb;
    border-color: #d4c8ec;
    border-left-color: #9a81c5;
    color: #58407e;
  }
  .calendar-event.selected {
    outline: 2px solid #286b50;
    outline-offset: 0;
    z-index: 2;
  }
  .calendar-event.cancelled {
    opacity: 0.55;
  }
  .calendar-event.cancelled strong {
    text-decoration: line-through;
  }
  .calendar-event.dragging {
    opacity: 0.28;
  }
  .calendar-event-content {
    display: flex;
    flex-direction: column;
    justify-content: flex-start;
    align-items: stretch;
    gap: 0;
    width: 100%;
    height: 100%;
    padding: 4px 23px 7px 6px;
    border-radius: 4px;
    border: 0;
    background: transparent;
    color: inherit;
    text-align: left;
    overflow: hidden;
    cursor: grab;
    touch-action: pan-y;
  }
  .calendar-event-content strong {
    font-size: 12px;
    line-height: 1.2;
    font-weight: 650;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: none;
  }
  .calendar-event-content span {
    font-size: 11px;
    line-height: 1.2;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: none;
  }
  .calendar-event-content small {
    font-size: 10px;
    line-height: 1.2;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    flex: none;
  }
  .calendar-event.compact .calendar-event-content span,
  .calendar-event.short .calendar-event-content small {
    display: none;
  }
  .calendar-event.compact .calendar-event-content {
    padding-top: 2px;
    padding-bottom: 6px;
  }
  .calendar-event.short .calendar-event-content {
    padding: 0 23px 0 5px;
  }
  .calendar-move-handle {
    position: absolute;
    top: 1px;
    right: 1px;
    width: 21px;
    height: min(22px, 100%);
    padding: 0;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: inherit;
    opacity: 0.65;
    font-size: 17px;
    cursor: grab;
    touch-action: none;
  }
  .calendar-move-handle:hover,
  .calendar-move-handle:focus-visible {
    opacity: 1;
    background: #ffffff90;
  }
  .calendar-resize-handle {
    position: absolute;
    bottom: 0;
    left: 0;
    width: 100%;
    height: 9px;
    border: 0;
    border-radius: 0 0 4px 4px;
    padding: 0;
    background: transparent;
    cursor: ns-resize;
    touch-action: none;
  }
  .calendar-resize-handle span {
    display: block;
    height: 3px;
    width: 23px;
    margin: auto;
    border-radius: 3px;
    background: currentColor;
    opacity: 0.32;
  }
  .calendar-resize-handle:hover span,
  .calendar-resize-handle:focus-visible span {
    opacity: 0.8;
  }
  .calendar-placement {
    pointer-events: none;
    position: absolute;
    z-index: 4;
    left: 3px;
    right: 3px;
    border: 2px solid #377b5d;
    border-radius: 6px;
    background: #dcf0e5ea;
    color: #1f5a41;
    padding: 3px 6px;
    overflow: hidden;
    box-shadow: 0 3px 10px #1a46351c;
  }
  .calendar-placement strong,
  .calendar-placement span {
    display: block;
    font-size: 12px;
    white-space: nowrap;
  }
  .calendar-placement span {
    font-size: 11px;
  }
  @media (pointer: coarse) {
    .calendar-move-handle {
      width: 27px;
      height: 26px;
      font-size: 20px;
    }
    .calendar-event-content {
      cursor: default;
      padding-right: 28px;
    }
    .calendar-resize-handle {
      height: 13px;
    }
  }
  @media (max-width: 700px) {
    .calendar-instructions {
      font-size: 11px;
      padding: 0 12px 12px;
    }
    .time-calendar-header > div {
      padding: 10px 3px;
    }
    .calendar-event-content strong {
      font-size: 12px;
    }
  }
</style>
