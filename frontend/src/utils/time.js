// This app always displays times in BRT (UTC-3, no daylight saving since
// 2019), whatever time zone the browser runs in.
import dayjs from "dayjs";
import utc from "dayjs/plugin/utc";

dayjs.extend(utc);

const BRT_OFFSET_MINUTES = -180;

// `ts` is anything dayjs parses: an ISO string with an offset (the API's
// "2026-11-02T22:48:21+00:00"), or a Date. Converting from UTC instead of
// subtracting 3 hours from a local time keeps it right on a machine that
// is already on BRT.
export function toBrt(ts) {
  return dayjs.utc(ts).utcOffset(BRT_OFFSET_MINUTES);
}

export function formatBrt(ts) {
  return toBrt(ts).format("YYYY-MM-DD HH:mm:ss");
}
