import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { MoveBalanceDialog } from "./MoveBalanceDialog";
import * as api from "../api/client";

vi.mock("../hooks/useCurrentPortfolio", () => ({ useCurrentPortfolio: () => ({ id: 7, name: "Account A", access: "manage" }) }));
vi.mock("../api/client", () => ({ fetchPositionSources: vi.fn(), recordTransfer: vi.fn() }));

const report = {
  allocations: [{ symbol: "BTC", barca: "Base", price: 100 }],
  barcaAllocations: [{ barca: "Base" }, { barca: "RendaPassiva" }],
  barcaActualAllocations: [
    { barca: "Base", value: 100 },
    { barca: "RendaPassiva", value: 100 },
  ],
};
const sources = [
  { symbol: "BTC", group_name: "Base", barca: "Base", asset_class: "crypto", current_quantity: 1, notes: "Binance" },
  { symbol: "ETH", group_name: "Yield", barca: "RendaPassiva", asset_class: "crypto", current_quantity: 2, notes: "Deribit" },
];

function open() {
  const onMoved = vi.fn();
  render(<MoveBalanceDialog report={report} accountName="Account A" onClose={() => {}} onMoved={onMoved} />);
  return { onMoved };
}

describe("MoveBalanceDialog", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    api.fetchPositionSources.mockResolvedValue(sources);
    api.recordTransfer.mockResolvedValue({ from_quantity: 0.85, to_quantity: 0.15, new_position: true });
  });

  it("moves 15% to another BARCA, shows the BARCA shift and names the new position", async () => {
    const user = userEvent.setup();
    const { onMoved } = open();
    const preview = await screen.findByTestId("move-preview");

    await user.click(screen.getByRole("button", { name: "15%" }));
    expect(screen.getByLabelText("Quantity to move (BTC)")).toHaveValue("0.15");
    await user.click(screen.getByLabelText("To BARCA"));
    await user.click(screen.getByRole("option", { name: "RendaPassiva" }));

    // The group defaults to RendaPassiva's existing group; the exchange stays.
    expect(within(preview).getByText(/BTC RendaPassiva · Yield · Binance: 0 → 0.15 \(new position, 0% target/)).toBeInTheDocument();
    expect(within(preview).getByText("Base 50.00% → 42.50% · RendaPassiva 50.00% → 57.50%")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Move 0.15 BTC in Account A" }));
    await waitFor(() => expect(onMoved).toHaveBeenCalled());
    expect(api.recordTransfer).toHaveBeenCalledWith(7, {
      from: { symbol: "BTC", group_name: "Base", barca: "Base", asset_class: "crypto", source: "Binance" },
      to: { symbol: "BTC", group_name: "Yield", barca: "RendaPassiva", asset_class: "crypto", source: "Binance" },
      quantity: 0.15,
      received: null,
    });
    expect(onMoved.mock.calls[0][0].message).toMatch(/0% target; set it in Targets/);
  });

  it("moves to a new exchange with a fee", async () => {
    const user = userEvent.setup();
    open();
    await screen.findByTestId("move-preview");
    await user.type(screen.getByLabelText("Quantity to move (BTC)"), "0.5");
    const exchange = screen.getByLabelText("To exchange");
    await user.clear(exchange);
    await user.type(exchange, "Deribit Crypto Options");
    const received = screen.getByLabelText("Quantity received (BTC)");
    await user.clear(received);
    await user.type(received, "0.4995");
    expect(screen.getByText("Fee: 0.0005 BTC")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Move 0.5 BTC in Account A" }));
    await waitFor(() => expect(api.recordTransfer).toHaveBeenCalled());
    const body = api.recordTransfer.mock.calls[0][1];
    expect(body.to).toMatchObject({ barca: "Base", group_name: "Base", source: "Deribit Crypto Options" });
    expect(body.received).toBe(0.4995);
  });

  it("won't move more than the origin holds, or to the same place", async () => {
    const user = userEvent.setup();
    open();
    await screen.findByTestId("move-preview");
    await user.type(screen.getByLabelText("Quantity to move (BTC)"), "2");
    expect(screen.getByText("The origin holds only 1.")).toBeInTheDocument();
    expect(screen.getByText("The destination is the same as the origin.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Move 2 BTC in Account A" })).toBeDisabled();
  });
});
