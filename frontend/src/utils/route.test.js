import { describe, expect, it } from "vitest";
import { formatHash, parseHash } from "./route";

describe("parseHash", () => {
  it("reads the page and its params", () => {
    expect(parseHash("#/holdings?view=barca&q=Base")).toEqual({ page: "holdings", params: { view: "barca", q: "Base" } });
  });

  it("decodes spaces and symbols in params", () => {
    expect(parseHash("#/holdings?q=Renda+Passiva%2FFII").params.q).toBe("Renda Passiva/FII");
  });

  it("is an empty page for no hash or a bare #", () => {
    expect(parseHash("")).toEqual({ page: "", params: {} });
    expect(parseHash("#")).toEqual({ page: "", params: {} });
    expect(parseHash(undefined)).toEqual({ page: "", params: {} });
  });
});

describe("formatHash", () => {
  it("writes params in a stable order and drops empty ones", () => {
    expect(formatHash("holdings", { view: "asset", q: "", out: undefined, a: "1" })).toBe("#/holdings?a=1&view=asset");
    expect(formatHash("history")).toBe("#/history");
  });

  it("round-trips through parseHash", () => {
    const params = { view: "group", q: "Core & Alts" };
    expect(parseHash(formatHash("holdings", params))).toEqual({ page: "holdings", params });
  });
});
