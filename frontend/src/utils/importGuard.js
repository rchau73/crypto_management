// Whether importing `file` could undo what was recorded in the app (trades
// and balance moves). The
// import sets every source the CSV lists to the CSV's quantity, so a file
// saved before the latest trade puts the old balances back, and a balance
// recorded on a new exchange is then counted twice (the CSV doesn't zero it).
//
// `activity` is GET /recorded_activity ({ trades, transfers, latest_at }), or null when
// it couldn't be loaded: then we can't tell, so we warn.
// Returns null (import right away) or { latestAt } (latestAt may be null).
export function staleImportWarning(file, activity) {
  if (activity === null) return { latestAt: null };
  if (!(activity.trades || activity.transfers) || !activity.latest_at) return null;
  const latest = Date.parse(activity.latest_at);
  if (Number.isNaN(latest)) return { latestAt: null };
  return file.lastModified < latest ? { latestAt: activity.latest_at } : null;
}
