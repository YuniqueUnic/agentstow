export function basenameFromPath(value: string): string {
  const normalized = value.replace(/\\/g, '/').replace(/\/$/, '');
  const segments = normalized.split('/');
  return segments[segments.length - 1] || normalized || 'workspace';
}

export function truncateMiddle(value: string, edge = 18): string {
  if (value.length <= edge * 2 + 1) {
    return value;
  }

  return `${value.slice(0, edge)}…${value.slice(-edge)}`;
}

export function formatRelativeTime(input: string | null | undefined): string {
  if (!input) {
    return '未记录时间';
  }

  const timestamp = Date.parse(input);
  if (Number.isNaN(timestamp)) {
    return input;
  }

  const deltaMs = timestamp - Date.now();
  const deltaMinutes = Math.round(deltaMs / 60000);
  const formatter = new Intl.RelativeTimeFormat('zh-CN', { numeric: 'auto' });

  if (Math.abs(deltaMinutes) < 60) {
    return formatter.format(deltaMinutes, 'minute');
  }

  const deltaHours = Math.round(deltaMinutes / 60);
  if (Math.abs(deltaHours) < 48) {
    return formatter.format(deltaHours, 'hour');
  }

  const deltaDays = Math.round(deltaHours / 24);
  if (Math.abs(deltaDays) < 31) {
    return formatter.format(deltaDays, 'day');
  }

  return new Intl.DateTimeFormat('zh-CN', {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit'
  }).format(timestamp);
}

