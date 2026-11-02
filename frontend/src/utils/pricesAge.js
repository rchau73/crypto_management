// How old the shared prices are, in words, and whether that is old enough
// to warn about. Pure (takes `now`) so it can be tested without timers.
import { formatBrt } from "./time";

// The app asks the backend this often. That only reads the database: the
// backend calls the price APIs when its prices are older than the refresh
// interval (PRICE_REFRESH_MINUTES).
export const POLL_INTERVAL_MS = 10_000;

const MINUTE_MS = 60_000;
const HOUR_MS = 60 * MINUTE_MS;

// `pricesAsOf`: RFC 3339 from the API, or null before the first answer.
// Stale = more than two refresh intervals old: one missed refresh can be a
// slow API, two means refreshes are failing and the numbers are drifting.
export function describePricesAge(pricesAsOf, refreshIntervalSecs, now = Date.now()) {
  if (!pricesAsOf) return { label: "Loading prices…", stale: false };
  const fetchedAt = new Date(pricesAsOf).getTime();
  if (Number.isNaN(fetchedAt)) return { label: "Prices: unknown time", stale: true };

  const ageMs = Math.max(0, now - fetchedAt);
  const staleAfterMs = 2 * (refreshIntervalSecs || 0) * 1000;
  const stale = staleAfterMs > 0 && ageMs > staleAfterMs;

  let label;
  if (ageMs < MINUTE_MS) label = "Prices from just now";
  else if (ageMs < HOUR_MS) label = `Prices from ${Math.floor(ageMs / MINUTE_MS)} min ago`;
  else if (ageMs < 24 * HOUR_MS) label = `Prices from ${Math.floor(ageMs / HOUR_MS)} h ago`;
  else label = `Prices from ${formatBrt(pricesAsOf).slice(0, 16)}`;
  return { label, stale };
}
