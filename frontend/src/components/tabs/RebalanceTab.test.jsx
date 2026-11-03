import { useState } from "react";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { RebalanceTab } from "./RebalanceTab";

// BARCA.md's crash example: Base below its band, Caixa and RendaPassiva above.
const positions = [
  ["BTC", "Base", 38000, 50],
  ["USDT", "Caixa", 40000, 30],
  ["ETH", "Altcoins", 9000, 10],
  ["FII-A", "RendaPassiva", 7000, 5],
  ["FII-B", "RendaPassiva", 6000, 5],
];
const targets = { Base: 50, Altcoins: 10, RendaPassiva: 10, Caixa: 30 };
const report = {
  allocations: positions.map(([symbol, barca, value, target_percent]) => ({ symbol, group: "G", barca, value, target_percent })),
  barcaAllocations: Object.entries(targets).map(([barca, target_percent]) => ({ barca, target_percent, value: 0 })),
  barcaActualAllocations: Object.keys(targets).map((barca) => ({
    barca,
    value: positions.filter((p) => p[1] === barca).reduce((s, p) => s + p[2], 0),
  })),
};

function Harness({ pricesAsOf = new Date().toISOString(), onOpenHoldings = vi.fn() }) {
  const [sentiment, setSentiment] = useState(null);
  const [done, setDone] = useState(() => new Set());
  return (
    <RebalanceTab
      report={report}
      sentiment={sentiment}
      onSentimentChange={setSentiment}
      done={done}
      onToggleDone={(id) => setDone((prev) => new Set(prev.has(id) ? [] : [...prev, id]))}
      pricesAsOf={pricesAsOf}
      refreshIntervalSecs={300}
      onOpenHoldings={onOpenHoldings}
    />
  );
}

const steps = () => within(screen.getByRole("heading", { name: "This round's steps" }).parentElement);

describe("RebalanceTab", () => {
  it("shows the bands right away but asks for the sentiment before any step", () => {
    render(<Harness />);
    expect(screen.getByText("3 of 4 BARCA outside their band")).toBeInTheDocument();
    expect(screen.getByRole("img", { name: /Base: 38\.00% against a target of 50\.00%, Below band/ })).toBeInTheDocument();
    expect(steps().getByText("Pick today's market sentiment above to see the steps.")).toBeInTheDocument();
    expect(screen.queryByRole("checkbox")).not.toBeInTheDocument();
  });

  it("in fear, lists the buy, explains why the sell waits, and states the rule", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.click(screen.getByRole("button", { name: "Fear / Extreme fear" }));

    expect(screen.getByText(/Never sell into fear/)).toBeInTheDocument();
    expect(steps().getByRole("checkbox", { name: /Buy \$3,600\.00 of BTC/ })).toBeInTheDocument();
    expect(steps().getByText(/sell when the fear passes, never into it/)).toBeInTheDocument();
  });

  it("ticking a step marks it done without removing it", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.click(screen.getByRole("button", { name: "Neutral" }));
    const buy = steps().getByRole("checkbox", { name: /Buy \$3,600\.00 of BTC/ });

    await user.click(buy);

    expect(buy).toBeChecked();
    expect(steps().getAllByRole("checkbox")).toHaveLength(3); // BTC, FII-A, FII-B
  });

  it("warns when the prices are old", () => {
    render(<Harness pricesAsOf={new Date(Date.now() - 60 * 60_000).toISOString()} />);
    expect(screen.getByRole("alert")).toHaveTextContent("Prices from 1 h ago: the steps below use those prices.");
  });

  it("links to Holdings for the asset-level detail", async () => {
    const user = userEvent.setup();
    const onOpenHoldings = vi.fn();
    render(<Harness onOpenHoldings={onOpenHoldings} />);
    await user.click(screen.getByRole("button", { name: "See every asset in Holdings" }));
    expect(onOpenHoldings).toHaveBeenCalled();
  });
});
