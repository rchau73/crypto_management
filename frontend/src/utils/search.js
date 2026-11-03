// What the header search box offers for a query: pages, assets, groups and
// BARCAs, each with where it leads ({ page, params }). Pure and tested.

const MAX_RESULTS = 8;

function unique(values) {
  return Array.from(new Set(values.filter(Boolean))).sort();
}

// `pages`: the visible pages; `allocations`: per-position rows.
export function buildSearchIndex(pages, allocations = []) {
  const entries = pages.map((p) => ({ kind: "Page", label: p.label, page: p.key, params: {} }));
  unique(allocations.map((a) => a.symbol)).forEach((symbol) => {
    const barcas = unique(allocations.filter((a) => a.symbol === symbol).map((a) => a.barca));
    entries.push({ kind: `Asset · ${barcas.join(", ")}`, label: symbol, page: "holdings", params: { view: "asset", q: symbol } });
  });
  unique(allocations.map((a) => a.barca)).forEach((barca) => {
    entries.push({ kind: "BARCA", label: barca, page: "holdings", params: { view: "barca", q: barca } });
  });
  unique(allocations.map((a) => a.group)).forEach((group) => {
    entries.push({ kind: "Group", label: group, page: "holdings", params: { view: "group", q: group } });
  });
  return entries;
}

// Case-insensitive; names that start with the query come first, then the
// rest in index order (pages, assets, BARCAs, groups).
export function searchIndex(index, query) {
  const q = String(query || "").trim().toLowerCase();
  if (!q) return [];
  const matches = index.filter((e) => e.label.toLowerCase().includes(q));
  const starts = matches.filter((e) => e.label.toLowerCase().startsWith(q));
  const rest = matches.filter((e) => !e.label.toLowerCase().startsWith(q));
  return [...starts, ...rest].slice(0, MAX_RESULTS);
}
