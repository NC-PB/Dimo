/**
 * How many ribbon groups fit into one row (T3.15). The ribbon never wraps: groups that do not
 * fit go into the "More" menu at the end, in order, and the menu button takes `more` px.
 * Unknown widths count as 0, so everything is shown until it was measured once.
 */
export function fitGroups(widths: readonly number[], available: number, more: number): number {
  const total = widths.reduce((sum, w) => sum + w, 0);
  if (total <= available) {
    return widths.length;
  }
  let used = more;
  let count = 0;
  for (const width of widths) {
    if (used + width > available) {
      break;
    }
    used += width;
    count++;
  }
  return count;
}
