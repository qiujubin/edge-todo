/** 倒计时/日期的紧凑格式化（等宽数字友好） */

const pad = (n: number) => String(n).padStart(2, "0");

export function parseDue(iso: string): Date | null {
  const d = new Date(iso.length === 10 ? `${iso}T23:59:00` : `${iso}:00`);
  return isNaN(d.getTime()) ? null : d;
}

export function isSameDay(a: Date, b: Date): boolean {
  return (
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate()
  );
}

function weekdayShort(d: Date): string {
  return ["周日", "周一", "周二", "周三", "周四", "周五", "周六"][d.getDay()];
}

/** 短日期 "11-20" */
export function shortDate(d: Date): string {
  return `${pad(d.getMonth() + 1)}.${pad(d.getDate())}`;
}

/**
 * 徽章文案：
 *   今日带时刻 → 「剩 2h05」/「过 1h」（过期红）
 *   今日无时刻 → 「今天」
 *   明天 → 「明天」；本周 → 周几；更远 → 「11.20」
 */
export function dueLabel(iso: string, now: Date): { text: string; tone: "danger" | "warn" | "calm" } {
  const due = parseDue(iso);
  if (!due) return { text: "", tone: "calm" };

  const hasTime = iso.length > 10;
  const today = isSameDay(due, now);
  const tomorrow = isSameDay(due, new Date(now.getTime() + 86400_000));

  if (today) {
    if (!hasTime) return { text: "今天", tone: "calm" };
    const diffMin = Math.round((due.getTime() - now.getTime()) / 60000);
    if (diffMin < 0) {
      const h = Math.floor(-diffMin / 60);
      const m = -diffMin % 60;
      return { text: h > 0 ? `过 ${h}h${pad(m)}` : `过 ${m}m`, tone: "danger" };
    }
    if (diffMin <= 60) return { text: `剩 ${diffMin}m`, tone: "danger" };
    if (diffMin <= 180) {
      return { text: `剩 ${Math.floor(diffMin / 60)}h${pad(diffMin % 60)}`, tone: "warn" };
    }
    return { text: `${pad(due.getHours())}:${pad(due.getMinutes())}`, tone: "calm" };
  }
  if (tomorrow) return { text: "明天", tone: "calm" };
  const endOfWeek = new Date(now);
  endOfWeek.setDate(now.getDate() + (6 - ((now.getDay() + 6) % 7)));
  if (due <= endOfWeek) return { text: weekdayShort(due), tone: "calm" };
  return { text: shortDate(due), tone: "calm" };
}

/** 今天日期头部的大数字 "11.16" 与 "周一" */
export function headerDate(now: Date): { big: string; zh: string; en: string } {
  return {
    big: shortDate(now),
    zh: weekdayShort(now),
    en: ["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"][now.getDay()],
  };
}

/** 本地 ISO 日期 "2026-11-16" */
export function isoDate(d: Date): string {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** 本地 ISO 时刻 "2026-11-16T18:00" */
export function isoDateTime(d: Date): string {
  return `${isoDate(d)}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/** 明天的 ISO 日期 */
export function tomorrowIso(): string {
  return isoDate(new Date(Date.now() + 86400_000));
}
