import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useHistoryDashboard } from "./useHistoryDashboard";
import * as api from "../api/client";

vi.mock("../api/client", () => ({ fetchHistory: vi.fn() }));

const ts = "2026-01-01T12:00:00Z";
const responses = {
  totals: { level: "totals", rows: [{ timestamp: ts, total_value: 100 }] },
  assets: {
    level: "assets",
    rows: [
      { timestamp: ts, symbol: "BTC", value: 80 },
      { timestamp: ts, symbol: "ETH", value: 20 },
    ],
  },
  groups: { level: "groups", rows: [{ timestamp: ts, group: "Core", value: 100 }] },
};

describe("useHistoryDashboard", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    api.fetchHistory.mockImplementation(async (level) => responses[level]);
  });

  it("fetches the totals history on mount", async () => {
    const { result } = renderHook(() => useHistoryDashboard());
    await waitFor(() => expect(result.current.data).toHaveLength(1));
    expect(api.fetchHistory).toHaveBeenCalledWith("totals");
  });

  it("resets the selected series when the level changes", async () => {
    const { result } = renderHook(() => useHistoryDashboard());
    act(() => result.current.setLevel("assets"));
    await waitFor(() => expect(result.current.selectedSeries).toEqual(expect.arrayContaining(["BTC", "ETH"])));

    act(() => result.current.setLevel("groups"));
    // Regression: asset names used to stay selected, leaving the chart empty.
    await waitFor(() => expect(result.current.selectedSeries).toEqual(["Core"]));
  });

  it("changing granularity re-buckets without fetching again", async () => {
    const { result } = renderHook(() => useHistoryDashboard());
    await waitFor(() => expect(result.current.data).toHaveLength(1));
    const calls = api.fetchHistory.mock.calls.length;

    act(() => result.current.setGranularity("weekly"));

    expect(result.current.granularity).toBe("weekly");
    expect(api.fetchHistory).toHaveBeenCalledTimes(calls);
  });

  it("surfaces a fetch failure as an error message", async () => {
    api.fetchHistory.mockRejectedValue(new Error("network down"));
    const { result } = renderHook(() => useHistoryDashboard());
    await waitFor(() => expect(result.current.error).toBe("network down"));
    expect(result.current.data).toEqual([]);
  });
});
