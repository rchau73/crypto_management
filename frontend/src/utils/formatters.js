// Number/currency formatting shared across tables and charts.

export function formatNumber(n) {
  if (typeof n !== "number" || isNaN(n)) return "-";
  return n.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
}

export function formatPrice(n) {
  if (typeof n !== "number" || isNaN(n)) return "-";
  return n.toLocaleString("en-US", { minimumFractionDigits: 4, maximumFractionDigits: 4 });
}

const USD_AXIS_FORMATTER = new Intl.NumberFormat("en-US", {
  style: "currency",
  currency: "USD",
  maximumFractionDigits: 0,
  notation: "compact",
  compactDisplay: "short",
});

export function formatUsdAxisTick(value) {
  if (typeof value !== "number" || isNaN(value)) return "";
  return USD_AXIS_FORMATTER.format(value);
}
