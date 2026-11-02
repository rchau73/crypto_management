import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useAllocations } from "./useAllocations";
import * as api from "../api/client";
import { POLL_INTERVAL_MS } from "../utils/pricesAge";

vi.mock("./useCurrentPortfolio", () => ({ useCurrentPortfolio: () => ({ id: 7, name: "Test", access: "manage" }) }));
vi.mock("../api/client", () => ({ fetchAllocations: vi.fn(), uploadWalletCsv: vi.fn(), downloadWalletCsv: vi.fn() }));

const report = (value, asOf = "2026-11-02T22:00:00+00:00") => ({
  per_asset: [{ symbol: "BTC", value }],
  per_group: [],
  per_barca: [],
  per_barca_actual: [],
  usd_brl: null,
  prices_as_of: asOf,
  refresh_interval_secs: 300,
});

// Lets the pending fetch promise resolve inside act().
const flush = () => act(async () => {});

function setHidden(hidden) {
  Object.defineProperty(document, "hidden", { configurable: true, get: () => hidden });
}

describe("useAllocations", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.resetAllMocks();
    setHidden(false);
  });
  afterEach(() => vi.useRealTimers());

  it("loads on mount, without any button", async () => {
    api.fetchAllocations.mockResolvedValue(report(100));
    const { result } = renderHook(() => useAllocations());
    await flush();

    expect(api.fetchAllocations).toHaveBeenCalledWith(7);
    expect(result.current.allocations).toEqual([{ symbol: "BTC", value: 100 }]);
    expect(result.current.pricesAsOf).toBe("2026-11-02T22:00:00+00:00");
    expect(result.current.refreshIntervalSecs).toBe(300);
    expect(result.current.loaded).toBe(true);
  });

  it("polls every interval and picks up new prices", async () => {
    api.fetchAllocations.mockResolvedValueOnce(report(100)).mockResolvedValue(report(120, "2026-11-02T22:05:00+00:00"));
    const { result } = renderHook(() => useAllocations());
    await flush();

    await act(async () => vi.advanceTimersByTime(POLL_INTERVAL_MS));

    expect(api.fetchAllocations).toHaveBeenCalledTimes(2);
    expect(result.current.allocations[0].value).toBe(120);
  });

  it("does not poll while the browser tab is hidden", async () => {
    api.fetchAllocations.mockResolvedValue(report(100));
    renderHook(() => useAllocations());
    await flush();
    setHidden(true);

    await act(async () => vi.advanceTimersByTime(3 * POLL_INTERVAL_MS));

    expect(api.fetchAllocations).toHaveBeenCalledTimes(1);
  });

  it("keeps the last numbers on screen when a poll fails, and says so", async () => {
    api.fetchAllocations.mockResolvedValueOnce(report(100)).mockRejectedValue(new Error("network down"));
    const { result } = renderHook(() => useAllocations());
    await flush();

    await act(async () => vi.advanceTimersByTime(POLL_INTERVAL_MS));

    expect(result.current.allocations[0].value).toBe(100);
    expect(result.current.loadError).toBe("Failed to fetch allocations: network down");
  });

  it("stops polling once unmounted (e.g. switching account)", async () => {
    api.fetchAllocations.mockResolvedValue(report(100));
    const { unmount } = renderHook(() => useAllocations());
    await flush();
    unmount();

    await act(async () => vi.advanceTimersByTime(3 * POLL_INTERVAL_MS));

    expect(api.fetchAllocations).toHaveBeenCalledTimes(1);
  });
});
