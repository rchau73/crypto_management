// Pure calculations shared by the allocation tables and the dashboard chart.
// Nothing here touches React state or the DOM, so it's all trivially testable.

export function computeYAxisDomain(rows, keys) {
  if (!Array.isArray(rows) || rows.length === 0) return ["dataMin", "dataMax"];
  if (!Array.isArray(keys) || keys.length === 0) return ["dataMin", "dataMax"];
  let min = Infinity;
  let max = -Infinity;
  rows.forEach((row) => {
    keys.forEach((key) => {
      const raw = row[key];
      const value = typeof raw === "number" ? raw : Number(raw);
      if (isNaN(value)) return;
      if (value < min) min = value;
      if (value > max) max = value;
    });
  });
  if (!isFinite(min) || !isFinite(max)) return ["dataMin", "dataMax"];
  if (min === max) {
    const pad = min === 0 ? 1 : Math.abs(min) * 0.05;
    return [min - pad, max + pad];
  }
  const pad = (max - min) * 0.05;
  return [min - pad, max + pad];
}

export function getTotalTargetPercent(rows) {
  return rows.reduce((sum, row) => {
    const val = typeof row.target_percent === "number" ? row.target_percent : parseFloat(row.target_percent);
    return sum + (isNaN(val) ? 0 : val);
  }, 0);
}

// BARCA rule 1 (BARCA.md): a row needs action only when its current share is
// more than 20% *of its own target* away from that target. Relative on
// purpose: 2 points off is noise for a 50% target but a 20% miss for 10%.
export const BAND_FRACTION = 0.2;

// "below" / "in" / "above" the band. A row holding value with no target
// at all (0%) is above any band; an empty untargeted row is in it.
export function bandStatus(currentPercent, targetPercent) {
  const current = Number(currentPercent) || 0;
  const target = Number(targetPercent) || 0;
  if (target <= 0) return current > 0 ? "above" : "in";
  const margin = target * BAND_FRACTION;
  if (current > target + margin) return "above";
  if (current < target - margin) return "below";
  return "in";
}

// Collapse per-asset rows (one per symbol+group+barca) into one row per
// symbol, summing quantity/value/target across whichever groups/wallets hold
// that asset.
export function aggregateBySymbol(rows) {
  const bySymbol = new Map();
  rows.forEach((row) => {
    const symbol = row.symbol;
    if (!symbol) return;
    const qty = typeof row.current_quantity === "number" ? row.current_quantity : Number(row.current_quantity || 0);
    const value = typeof row.value === "number" ? row.value : Number(row.value || 0);
    const target = typeof row.target_percent === "number" ? row.target_percent : Number(row.target_percent || 0);
    const price = typeof row.price === "number" ? row.price : Number(row.price || 0);

    let entry = bySymbol.get(symbol);
    if (!entry) {
      entry = { symbol, current_quantity: 0, value: 0, target_percent: 0, last_price: 0 };
      bySymbol.set(symbol, entry);
    }
    entry.current_quantity += isNaN(qty) ? 0 : qty;
    entry.value += isNaN(value) ? 0 : value;
    entry.target_percent += isNaN(target) ? 0 : target;
    if (!isNaN(price) && price > 0) entry.last_price = price;
  });

  return Array.from(bySymbol.values()).map((entry) => ({
    ...entry,
    price: entry.current_quantity > 0 ? entry.value / entry.current_quantity : entry.last_price || 0,
  }));
}
