import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { PricesAge } from "./PricesAge";

describe("PricesAge", () => {
  it("shows the age and how often prices refresh", () => {
    render(<PricesAge pricesAsOf={new Date().toISOString()} refreshIntervalSecs={300} />);
    const status = screen.getByRole("status");
    expect(status).toHaveTextContent("Prices from just now");
    expect(status).toHaveAttribute("title", "Prices refresh every 5 min while the app is open.");
  });

  it("warns in words, not only color, when prices are too old", () => {
    const hourAgo = new Date(Date.now() - 60 * 60_000).toISOString();
    render(<PricesAge pricesAsOf={hourAgo} refreshIntervalSecs={300} />);
    expect(screen.getByRole("status")).toHaveTextContent("Prices from 1 h ago — may be outdated");
  });
});
