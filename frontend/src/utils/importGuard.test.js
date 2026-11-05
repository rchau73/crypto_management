import { describe, expect, it } from "vitest";
import { staleImportWarning } from "./importGuard";

const fileAt = (iso) => ({ lastModified: Date.parse(iso) });

describe("staleImportWarning", () => {
  const activity = { trades: 2, latest_at: "2026-11-03T17:20:00.000Z" };

  it("warns about a file saved before the latest recorded trade", () => {
    expect(staleImportWarning(fileAt("2026-11-01T10:00:00Z"), activity)).toEqual({
      latestAt: "2026-11-03T17:20:00.000Z",
    });
  });

  it("lets a file saved after it import right away", () => {
    expect(staleImportWarning(fileAt("2026-11-03T18:00:00Z"), activity)).toBeNull();
  });

  it("counts balance moves like trades", () => {
    const moves = { trades: 0, transfers: 1, latest_at: "2026-11-03T17:20:00.000Z" };
    expect(staleImportWarning(fileAt("2026-11-01T10:00:00Z"), moves)).toEqual({ latestAt: moves.latest_at });
  });

  it("lets anything import when nothing was recorded", () => {
    expect(staleImportWarning(fileAt("2020-01-01T00:00:00Z"), { trades: 0, latest_at: null })).toBeNull();
  });

  it("warns, without a date, when the activity couldn't be loaded", () => {
    expect(staleImportWarning(fileAt("2026-11-03T18:00:00Z"), null)).toEqual({ latestAt: null });
  });
});
