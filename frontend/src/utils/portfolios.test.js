import { afterEach, describe, expect, it, vi } from "vitest";
import { loadRememberedPortfolioId, pickPortfolio, rememberPortfolioId } from "./portfolios";

const mine = { id: 1, name: "Mine" };
const spouse = { id: 2, name: "Spouse" };

describe("pickPortfolio", () => {
  it("picks the remembered portfolio when it is still in the list", () => {
    expect(pickPortfolio([mine, spouse], 2)).toBe(spouse);
  });

  it("falls back to the first when the remembered one is gone or unset", () => {
    expect(pickPortfolio([mine, spouse], 99)).toBe(mine);
    expect(pickPortfolio([mine, spouse], null)).toBe(mine);
  });

  it("is null for an empty list", () => {
    expect(pickPortfolio([], 1)).toBeNull();
  });
});

describe("remembered portfolio id", () => {
  afterEach(() => {
    vi.restoreAllMocks();
    window.localStorage.clear();
  });

  it("round-trips through localStorage as a number", () => {
    rememberPortfolioId(2);
    expect(loadRememberedPortfolioId()).toBe(2);
  });

  it("is null when nothing was remembered", () => {
    expect(loadRememberedPortfolioId()).toBeNull();
  });

  it("treats blocked storage as no memory instead of throwing", () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    expect(() => rememberPortfolioId(2)).not.toThrow();
    expect(loadRememberedPortfolioId()).toBeNull();
  });
});
