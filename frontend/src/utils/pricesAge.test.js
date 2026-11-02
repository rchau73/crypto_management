import { describe, expect, it } from "vitest";
import { describePricesAge } from "./pricesAge";

const AS_OF = "2026-11-02T22:00:00+00:00";
const at = (minutes) => new Date(AS_OF).getTime() + minutes * 60_000;
const FIVE_MIN = 300;

describe("describePricesAge", () => {
  it("says loading before the first answer", () => {
    expect(describePricesAge(null, FIVE_MIN)).toEqual({ label: "Loading prices…", stale: false });
  });

  it("words the age in minutes, then hours, then the BRT date", () => {
    expect(describePricesAge(AS_OF, FIVE_MIN, at(0.5)).label).toBe("Prices from just now");
    expect(describePricesAge(AS_OF, FIVE_MIN, at(4)).label).toBe("Prices from 4 min ago");
    expect(describePricesAge(AS_OF, FIVE_MIN, at(130)).label).toBe("Prices from 2 h ago");
    expect(describePricesAge(AS_OF, FIVE_MIN, at(60 * 30)).label).toBe("Prices from 2026-11-02 19:00");
  });

  it("is stale only past two refresh intervals", () => {
    expect(describePricesAge(AS_OF, FIVE_MIN, at(9)).stale).toBe(false);
    expect(describePricesAge(AS_OF, FIVE_MIN, at(11)).stale).toBe(true);
  });

  it("never calls prices stale without a known interval, and flags an unreadable time", () => {
    expect(describePricesAge(AS_OF, 0, at(600)).stale).toBe(false);
    expect(describePricesAge("garbage", FIVE_MIN, at(1))).toEqual({ label: "Prices: unknown time", stale: true });
  });

  it("treats a clock slightly behind the server as just now, not negative", () => {
    expect(describePricesAge(AS_OF, FIVE_MIN, at(-1)).label).toBe("Prices from just now");
  });
});
