import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useBarcaTargets } from "./useBarcaTargets";
import * as api from "../api/client";

vi.mock("../api/client", () => ({
  fetchBarcaTargets: vi.fn(),
  saveBarcaTargets: vi.fn(),
}));

describe("useBarcaTargets", () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  it("fetches targets for the given market on mount", async () => {
    api.fetchBarcaTargets.mockResolvedValue([{ barca: "Base", target_percent: 100 }]);
    const { result } = renderHook(() => useBarcaTargets("BullMarket"));
    await waitFor(() => expect(result.current.targets).toHaveLength(1));
    expect(api.fetchBarcaTargets).toHaveBeenCalledWith("BullMarket");
  });

  it("re-fetches with the new market when the market prop changes", async () => {
    api.fetchBarcaTargets.mockResolvedValue([]);
    const { rerender } = renderHook(({ market }) => useBarcaTargets(market), {
      initialProps: { market: "BullMarket" },
    });
    await waitFor(() => expect(api.fetchBarcaTargets).toHaveBeenCalledWith("BullMarket"));

    rerender({ market: "BearMarket" });
    await waitFor(() => expect(api.fetchBarcaTargets).toHaveBeenCalledWith("BearMarket"));
  });

  it("surfaces a fetch failure as an error message, not a thrown exception", async () => {
    api.fetchBarcaTargets.mockRejectedValue(new Error("network down"));
    const { result } = renderHook(() => useBarcaTargets("BullMarket"));
    await waitFor(() => expect(result.current.error).toBe("network down"));
    expect(result.current.targets).toEqual([]);
  });

  it("save calls saveBarcaTargets with the market and re-fetches", async () => {
    api.fetchBarcaTargets
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([{ barca: "Base", target_percent: 100 }]);
    api.saveBarcaTargets.mockResolvedValue({ ok: true });

    const { result } = renderHook(() => useBarcaTargets("BullMarket"));
    await waitFor(() => expect(api.fetchBarcaTargets).toHaveBeenCalledTimes(1));

    const targets = [{ barca: "Base", target_percent: 100 }];
    await act(async () => {
      await result.current.save(targets);
    });

    expect(api.saveBarcaTargets).toHaveBeenCalledWith("BullMarket", targets);
    expect(api.fetchBarcaTargets).toHaveBeenCalledTimes(2);
  });
});
