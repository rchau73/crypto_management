/**
 * Turns the summary returned by POST /api/import_wallets/upload into the
 * one-line status shown after "Import CSV".
 *
 * Sources missing from the CSV and differing targets are left untouched by
 * the backend, so they're called out: they usually mean the CSV and the
 * wallet disagree and need a manual look.
 */
export function describeImportSummary(summary = {}) {
  const {
    positions_added = 0,
    sources_added = 0,
    sources_updated = 0,
    sources_unchanged = 0,
    missing_from_csv = [],
    target_conflicts = [],
  } = summary;

  const parts = [
    `${positions_added} new position(s)`,
    `${sources_added} source(s) added`,
    `${sources_updated} updated`,
    `${sources_unchanged} unchanged`,
  ];
  if (missing_from_csv.length > 0) {
    const names = missing_from_csv.map(describeSource).join(", ");
    parts.push(`not in CSV, left as is: ${names}`);
  }
  if (target_conflicts.length > 0) {
    const names = target_conflicts.map((c) => c.symbol).join(", ");
    parts.push(`CSV target ignored (database kept) for: ${names}`);
  }
  return `Imported CSV: ${parts.join("; ")}`;
}

function describeSource({ symbol, group_name, notes }) {
  return `${symbol} (${group_name}${notes ? ` / ${notes}` : ""})`;
}
