export function timeMinutes(time) {
  const [hours, minutes] = time.split(":").map(Number);
  return hours * 60 + minutes;
}

export function timeString(minutes) {
  return `${String(Math.floor(minutes / 60)).padStart(2, "0")}:${String(minutes % 60).padStart(2, "0")}`;
}

export function snapMinute(minutes, step = 15) {
  return Math.round(minutes / step) * step;
}

export function moveRange(start, end, target) {
  const duration = end - start;
  const next = Math.max(0, Math.min(1439 - duration, snapMinute(target)));
  return { start: next, end: next + duration };
}

export function resizeRange(start, target) {
  return {
    start,
    end: Math.min(1439, Math.max(start + 15, snapMinute(target))),
  };
}

// Interval lanes also keep cancelled lessons legible beside replacement lessons.
export function dayLayout(lessons) {
  const sorted = lessons
    .map((lesson) => ({
      lesson,
      start: timeMinutes(lesson.starts_at),
      end: timeMinutes(lesson.ends_at),
    }))
    .sort(
      (a, b) =>
        a.start - b.start ||
        b.end - a.end ||
        a.lesson.id.localeCompare(b.lesson.id),
    );
  const result = [];
  let group = [],
    groupEnd = -1;
  function flush() {
    const lanes = [];
    for (const item of group) {
      let lane = lanes.findIndex((end) => end <= item.start);
      if (lane < 0) lane = lanes.length;
      lanes[lane] = item.end;
      item.lane = lane;
    }
    result.push(...group.map((item) => ({ ...item, lanes: lanes.length })));
  }
  for (const item of sorted) {
    if (item.start >= groupEnd) {
      flush();
      group = [];
      groupEnd = -1;
    }
    group.push(item);
    groupEnd = Math.max(groupEnd, item.end);
  }
  flush();
  return result;
}
