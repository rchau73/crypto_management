import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { useHashRoute } from "./useHashRoute";

describe("useHashRoute", () => {
  beforeEach(() => window.history.replaceState(null, "", "#/holdings?view=barca"));

  it("starts from the URL, so a reload keeps the page and view", () => {
    const { result } = renderHook(() => useHashRoute());
    expect(result.current.page).toBe("holdings");
    expect(result.current.params).toEqual({ view: "barca" });
  });

  it("navigate() updates the URL and the route", () => {
    const { result } = renderHook(() => useHashRoute());
    act(() => result.current.navigate("history"));
    expect(window.location.hash).toBe("#/history");
    expect(result.current.page).toBe("history");
  });

  it("replace updates the URL without adding a history entry", () => {
    const { result } = renderHook(() => useHashRoute());
    const before = window.history.length;
    act(() => result.current.navigate("holdings", { view: "asset", q: "et" }, { replace: true }));
    expect(window.location.hash).toBe("#/holdings?q=et&view=asset");
    expect(window.history.length).toBe(before);
    expect(result.current.params.q).toBe("et");
  });

  it("follows Back/Forward (hashchange)", () => {
    const { result } = renderHook(() => useHashRoute());
    act(() => {
      window.history.replaceState(null, "", "#/targets?tab=barca");
      window.dispatchEvent(new HashChangeEvent("hashchange"));
    });
    expect(result.current.page).toBe("targets");
    expect(result.current.params).toEqual({ tab: "barca" });
  });
});
