import { describe, expect, it } from "vitest";
import { resolvePage, selectedTab, visiblePages } from "./navigation";

const keys = (user, portfolio) => visiblePages(user, portfolio).map((p) => p.key);
const manage = { access: "manage" };
const view = { access: "view" };

describe("visiblePages", () => {
  it("shows a user only the read-only pages", () => {
    expect(keys({ role: "user" }, manage)).toEqual(["rebalance", "holdings", "history"]);
  });

  it("shows Targets only with manage access to this account", () => {
    expect(keys({ role: "manager" }, manage)).toEqual(["rebalance", "holdings", "history", "targets", "settings"]);
    expect(keys({ role: "manager" }, view)).toEqual(["rebalance", "holdings", "history", "settings"]);
  });

  it("gives an admin the same pages as a manager", () => {
    expect(keys({ role: "admin" }, manage)).toEqual(keys({ role: "manager" }, manage));
  });
});

describe("resolvePage", () => {
  it("keeps a visible page and falls back to Rebalance otherwise", () => {
    expect(resolvePage("history", { role: "user" }, view)).toBe("history");
    expect(resolvePage("targets", { role: "user" }, manage)).toBe("rebalance");
    expect(resolvePage("nonsense", { role: "admin" }, manage)).toBe("rebalance");
    expect(resolvePage("", { role: "admin" }, manage)).toBe("rebalance");
  });
});

describe("selectedTab", () => {
  const tabs = [{ key: "asset" }, { key: "barca" }];
  it("keeps a known tab and falls back to the first", () => {
    expect(selectedTab(tabs, "barca")).toBe("barca");
    expect(selectedTab(tabs, "users")).toBe("asset");
    expect(selectedTab(tabs, undefined)).toBe("asset");
  });
});
