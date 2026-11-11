import { useState } from "react";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { HoldingsTab } from "./HoldingsTab";

const report = {
  allocations: [
    { symbol: "BTC", group: "Core", barca: "Base", price: 100, current_quantity: 3, value: 300, target_percent: 30 },
    { symbol: "BTC", group: "Trading", barca: "Base", price: 100, current_quantity: 1, value: 100, target_percent: 20 },
    { symbol: "ETH", group: "Core", barca: "Altcoins", price: 10, current_quantity: 50, value: 500, target_percent: 30 },
    { symbol: "USDT", group: "Cash", barca: "Caixa", price: 1, current_quantity: 100, value: 100, target_percent: 20 },
  ],
  groupAllocations: [
    { group: "Core", value: 800, target_percent: 60 },
    { group: "Trading", value: 100, target_percent: 20 },
    { group: "Cash", value: 100, target_percent: 20 },
  ],
  barcaAllocations: [
    { barca: "Base", value: 400, target_percent: 50 },
    { barca: "Altcoins", value: 500, target_percent: 30 },
    { barca: "Caixa", value: 100, target_percent: 20 },
  ],
  barcaActualAllocations: [],
};

// Holds the URL params like App does, so clicks round-trip.
function Harness({ initial = {}, onChange = () => {}, unpriced }) {
  const [params, setParams] = useState(initial);
  return (
    <HoldingsTab
      report={unpriced ? { ...report, unpriced } : report}
      params={params}
      onParamsChange={(p, o) => {
        onChange(p, o);
        setParams(p);
      }}
    />
  );
}

const viewButton = (name) => within(screen.getByRole("group", { name: "View by" })).getByRole("button", { name });

const firstColumn = () =>
  within(screen.getByRole("table"))
    .getAllByRole("rowheader")
    .map((c) => c.textContent);

const fund = { symbol: "FUND A", group: "Holding", barca: "RendaFixa", asset_class: "br-equities", current_quantity: 1000 };

describe("HoldingsTab", () => {
  it("lists holdings with no price, and how to fix them, above the table", () => {
    render(<Harness unpriced={[fund]} />);
    const alert = screen.getByRole("alert");
    expect(alert).toHaveTextContent("1 holding has no price and is left out of every total");
    expect(alert).toHaveTextContent("FUND A · RendaFixa · br-equities · qty 1,000");
    expect(alert).toHaveTextContent(/brl-manual/);
  });

  it("shows no warning when everything is priced", () => {
    render(<Harness unpriced={[]} />);
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("says a filtered-for holding is held but unpriced instead of just 'no rows'", () => {
    render(<Harness unpriced={[fund]} initial={{ q: "fund a" }} />);
    expect(screen.getByText(/FUND A is held but has no price \(see the warning above\)/)).toBeInTheDocument();
  });

  it("opens on the Asset view, biggest $ gap first, with the step's direction", () => {
    render(<Harness />);
    expect(viewButton("Asset")).toHaveAttribute("aria-pressed", "true");
    expect(firstColumn()).toEqual(["ETH", "BTC", "USDT"]); // gaps +200, -100, -100
    const eth = screen.getByRole("rowheader", { name: "ETH" }).closest("tr");
    expect(within(eth).getByText("Sell $60.00")).toBeInTheDocument();
    expect(within(eth).getByText("Above band")).toBeInTheDocument();
    expect(within(eth).getByText("+$200.00")).toBeInTheDocument();
  });

  it("switches view and keeps the URL params in step", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<Harness onChange={onChange} />);

    await user.click(viewButton("BARCA"));

    expect(onChange).toHaveBeenLastCalledWith({ view: "barca", q: "", out: "" }, undefined);
    expect(firstColumn()).toEqual(["Altcoins", "Base", "Caixa"]);
    expect(screen.getByRole("columnheader", { name: "BARCA" })).toBeInTheDocument();
  });

  it("filters as you type, replacing the URL instead of adding history", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<Harness onChange={onChange} />);

    await user.type(screen.getByRole("textbox", { name: "Filter rows" }), "core");

    expect(firstColumn()).toEqual(["ETH", "BTC"]); // assets held in group Core
    expect(onChange).toHaveBeenLastCalledWith({ view: "asset", q: "core", out: "" }, { replace: true });
  });

  it("can show only rows outside their band", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.click(screen.getByRole("button", { name: "Only outside band (2)" }));
    expect(firstColumn()).toEqual(["ETH", "USDT"]);
  });

  it("offers a way back when nothing matches", async () => {
    const user = userEvent.setup();
    render(<Harness initial={{ q: "doge" }} />);
    expect(screen.getByText(/No rows match “doge”/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Show all rows" }));
    expect(firstColumn()).toHaveLength(3);
  });

  it("opens straight on a view and filter from the URL (the header search's links)", () => {
    render(<Harness initial={{ view: "group", q: "Trading" }} />);
    expect(firstColumn()).toEqual(["Trading"]);
  });

  it("warns when the targets of a view don't add up to 100%", () => {
    render(<HoldingsTab report={{ ...report, barcaAllocations: report.barcaAllocations.slice(0, 2) }} params={{ view: "barca" }} onParamsChange={() => {}} />);
    expect(screen.getByText("Targets in this view add up to 80.00%, not 100%.")).toBeInTheDocument();
  });
});
