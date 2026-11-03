import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { RecordTradeDialog } from "./RecordTradeDialog";
import * as api from "../api/client";

vi.mock("../hooks/useCurrentPortfolio", () => ({ useCurrentPortfolio: () => ({ id: 7, name: "Account A", access: "manage" }) }));
vi.mock("../api/client", () => ({ fetchPositionSources: vi.fn(), recordTrade: vi.fn() }));

const allocations = [
  { symbol: "SYRUP", barca: "Altcoins", price: 0.5 },
  { symbol: "BTC", barca: "Base", price: 80000 },
  { symbol: "USDT", barca: "Caixa", price: 1 },
];
const sources = [
  { symbol: "SYRUP", group_name: "DeFi", barca: "Altcoins", asset_class: "crypto", current_quantity: 40, notes: "Binance" },
  { symbol: "BTC", group_name: "Base", barca: "Base", asset_class: "crypto", current_quantity: 0.1, notes: "Ledger" },
  { symbol: "USDT", group_name: "Stable", barca: "Caixa", asset_class: "crypto", current_quantity: 100, notes: "Binance" },
];

function open(props = {}) {
  const onRecorded = vi.fn();
  render(
    <RecordTradeDialog
      preset={{ side: "buy", symbol: "SYRUP", barca: "Altcoins", amount: 10, stepId: "Buy|Altcoins|SYRUP" }}
      allocations={allocations}
      accountName="Account A"
      onClose={() => {}}
      onRecorded={onRecorded}
      {...props}
    />
  );
  return { onRecorded };
}

describe("RecordTradeDialog", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    api.fetchPositionSources.mockResolvedValue(sources);
    api.recordTrade.mockResolvedValue({ asset_quantity: 60, cash_quantity: 90 });
  });

  it("pre-fills a step's buy and records it with the cash from the same exchange", async () => {
    const { onRecorded } = open();
    const preview = await screen.findByTestId("trade-preview");
    // $10 at the live $0.50 = 20 SYRUP, paid with 10 USDT on Binance.
    expect(screen.getByLabelText("Quantity (SYRUP)")).toHaveValue("20");
    expect(within(preview).getByText("SYRUP on Binance: 40 → 60")).toBeInTheDocument();
    expect(within(preview).getByText("USDT on Binance: 100 → 90")).toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: "Record buy in Account A" }));
    await waitFor(() => expect(onRecorded).toHaveBeenCalled());
    expect(api.recordTrade).toHaveBeenCalledWith(7, {
      side: "buy",
      asset: { symbol: "SYRUP", group_name: "DeFi", barca: "Altcoins", asset_class: "crypto", source: "Binance" },
      quantity: 20,
      price: 0.5,
      cash: { position: { symbol: "USDT", group_name: "Stable", barca: "Caixa", asset_class: "crypto", source: "Binance" }, quantity: 10 },
    });
    expect(onRecorded.mock.calls[0][0]).toEqual({
      stepId: "Buy|Altcoins|SYRUP",
      message: "Recorded in Account A: bought 20 SYRUP on Binance at $0.50, paid with 10 USDT.",
    });
  });

  it("won't send a sell bigger than the exchange holds", async () => {
    open();
    await screen.findByTestId("trade-preview");
    await userEvent.click(screen.getByRole("button", { name: "Sell" }));
    const quantity = screen.getByLabelText("Quantity (SYRUP)");
    await userEvent.clear(quantity);
    await userEvent.type(quantity, "50");
    expect(screen.getByText("That exchange holds only 40; you can't sell more.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Record sell in Account A" })).toBeDisabled();
  });

  it("can skip the cash and keeps the server's error in the dialog", async () => {
    api.recordTrade.mockRejectedValue(new Error("Binance holds 40 SYRUP, so it can't give 50"));
    const { onRecorded } = open();
    await screen.findByTestId("trade-preview");
    await userEvent.click(screen.getByLabelText("Also take the cash from Caixa"));
    await userEvent.click(screen.getByRole("button", { name: "Record buy in Account A" }));
    expect(await screen.findByText("Binance holds 40 SYRUP, so it can't give 50")).toBeInTheDocument();
    expect(api.recordTrade.mock.calls[0][1].cash).toBeNull();
    expect(onRecorded).not.toHaveBeenCalled();
  });

  it("without a step, asks which asset and leaves the quantity empty", async () => {
    open({ preset: null });
    await screen.findByTestId("trade-preview");
    expect(screen.getByLabelText("Asset")).toBeInTheDocument();
    expect(screen.getByLabelText("Quantity (BTC)")).toHaveValue("");
    expect(screen.getByRole("button", { name: "Record buy in Account A" })).toBeDisabled();
  });
});
