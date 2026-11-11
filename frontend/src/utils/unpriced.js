// Pure helpers for the holdings the price round couldn't price (the
// report's `unpriced` list). They are left out of every total, so the UI
// says so instead of letting them vanish.

// What a holding of each class needs to get a price, in one sentence.
const FIX_BY_CLASS = {
  "br-equities":
    "brapi only prices B3 tickers. For a fund, Tesouro bond or cash in reais, add it in Portfolio Targets as brl-manual, with its value in R$ as the quantity.",
  "brl-manual": "There is no USD/BRL rate this round. Reais holdings come back with the next price update.",
};
const DEFAULT_FIX = "Check the symbol in Portfolio Targets: the price provider doesn't know it.";

// One fix sentence per distinct asset class, in the order they appear.
export function unpricedFixes(unpriced) {
  const classes = [...new Set(unpriced.map((u) => u.asset_class))];
  return [...new Set(classes.map((c) => FIX_BY_CLASS[c] ?? DEFAULT_FIX))];
}

export function unpricedTitle(count) {
  return count === 1
    ? "1 holding has no price and is left out of every total"
    : `${count} holdings have no price and are left out of every total`;
}

// The unpriced holdings the Holdings text filter matches (same fields as
// the table rows: name, group, BARCA).
export function matchUnpriced(unpriced, query) {
  const q = query.trim().toLowerCase();
  if (!q) return [];
  return unpriced.filter((u) => [u.symbol, u.group, u.barca].some((field) => String(field || "").toLowerCase().includes(q)));
}
