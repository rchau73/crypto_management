export function formatNumber(value) {
  if (typeof value !== "number" || Number.isNaN(value)) return "-";
  return value.toLocaleString("en-US", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
}

export function sumTargetPercent(rows) {
  return rows.reduce((sum, row) => {
    const numeric =
      typeof row.target_percent === "number"
        ? row.target_percent
        : Number.parseFloat(row.target_percent);
    return sum + (Number.isFinite(numeric) ? numeric : 0);
  }, 0);
}
