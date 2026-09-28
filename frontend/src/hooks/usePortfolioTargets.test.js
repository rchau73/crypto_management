import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { usePortfolioTargets } from "./usePortfolioTargets";
import * as api from "../api/client";

vi.mock("../api/client", () => ({
  fetchPortfolioTargets: vi.fn(),
  savePortfolioTargets: vi.fn(),
}));

describe("usePortfolioTargets", () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  it("does not fetch while inactive", () => {
    renderHook(() => usePortfolioTargets(false));
    expect(api.fetchPortfolioTargets).not.toHaveBeenCalled();
  });

  it("fetches the current rows once active", async () => {
    api.fetchPortfolioTargets.mockResolvedValue([{ symbol: "BTC", target_percent: 100 }]);
    const { result } = renderHook(() => usePortfolioTargets(true));
    await waitFor(() => expect(result.current.rows).toHaveLength(1));
    expect(result.current.loading).toBe(false);
  });

  it("surfaces a fetch failure as an error message, not a thrown exception", async () => {
    api.fetchPortfolioTargets.mockRejectedValue(new Error("network down"));
    const { result } = renderHook(() => usePortfolioTargets(true));
    await waitFor(() => expect(result.current.error).toBe("network down"));
    expect(result.current.rows).toEqual([]);
  });

  it("save re-fetches the rows afterwards", async () => {
    api.fetchPortfolioTargets
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([{ symbol: "BTC", target_percent: 100 }]);
    api.savePortfolioTargets.mockResolvedValue({ ok: true });

    const { result } = renderHook(() => usePortfolioTargets(true));
    await waitFor(() => expect(api.fetchPortfolioTargets).toHaveBeenCalledTimes(1));

    const rows = [{ symbol: "BTC", target_percent: 100 }];
    await act(async () => {
      await result.current.save(rows);
    });

    expect(api.savePortfolioTargets).toHaveBeenCalledWith(rows);
    expect(api.fetchPortfolioTargets).toHaveBeenCalledTimes(2);
    expect(result.current.rows).toEqual(rows);
  });
});
