import { describe, expect, it } from "vitest";
import { formatBrt, toBrt } from "./time";

describe("toBrt", () => {
  it("converts a UTC timestamp to BRT (UTC-3) once", () => {
    expect(formatBrt("2026-11-02T22:48:21+00:00")).toBe("2026-11-02 19:48:21");
  });

  it("does the same for a Date, so a 'now' stamp is not shifted twice", () => {
    // Regression: StatusLine subtracted 3 hours from a local Date, showing
    // 16:00 for an update made at 19:00 on a machine already on BRT.
    expect(formatBrt(new Date("2026-11-02T22:00:00Z"))).toBe("2026-11-02 19:00:00");
  });

  it("crosses midnight into the previous BRT day", () => {
    expect(toBrt("2026-11-03T01:30:00Z").format("YYYY-MM-DD HH:mm")).toBe("2026-11-02 22:30");
  });

  it("is invalid (not a throw) for an unparsable timestamp", () => {
    expect(toBrt("not-a-date").isValid()).toBe(false);
  });
});
