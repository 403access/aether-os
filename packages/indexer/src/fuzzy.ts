/**
 * Bounded edit distance. Used for typo tolerance ("indxer" -> "indexer").
 *
 * The band optimisation matters: fuzzy expansion runs against the caller's
 * candidate set on every keystroke, so we must fail fast when the distance
 * exceeds the budget.
 */
export function boundedLevenshtein(a: string, b: string, maxDistance: number): number {
  if (a === b) return 0;
  if (maxDistance <= 0) return maxDistance + 1;
  if (Math.abs(a.length - b.length) > maxDistance) return maxDistance + 1;

  const previous = new Array<number>(b.length + 1);
  const current = new Array<number>(b.length + 1);
  for (let j = 0; j <= b.length; j += 1) previous[j] = j;

  for (let i = 1; i <= a.length; i += 1) {
    current[0] = i;
    let rowMinimum = i;

    const from = Math.max(1, i - maxDistance);
    const to = Math.min(b.length, i + maxDistance);

    for (let j = 1; j <= b.length; j += 1) {
      if (j < from || j > to) {
        current[j] = maxDistance + 1;
        continue;
      }
      const cost = a[i - 1] === b[j - 1] ? 0 : 1;
      const deletion = (previous[j] ?? maxDistance) + 1;
      const insertion = (current[j - 1] ?? maxDistance) + 1;
      const substitution = (previous[j - 1] ?? maxDistance) + cost;
      const value = Math.min(deletion, insertion, substitution);
      current[j] = value;
      if (value < rowMinimum) rowMinimum = value;
    }

    if (rowMinimum > maxDistance) return maxDistance + 1;
    previous.splice(0, previous.length, ...current);
  }

  const distance = previous[b.length] ?? maxDistance + 1;
  return distance > maxDistance ? maxDistance + 1 : distance;
}

/** Edit budget: one edit for short queries, two once the term is long enough. */
export function editBudget(term: string): number {
  if (term.length <= 3) return 0;
  if (term.length <= 5) return 1;
  return 2;
}
