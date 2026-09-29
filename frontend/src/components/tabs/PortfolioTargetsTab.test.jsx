import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { PortfolioTargetsTab } from "./PortfolioTargetsTab";
import { usePortfolioTargets } from "../../hooks/usePortfolioTargets";
import * as api from "../../api/client";

vi.mock("../../hooks/usePortfolioTargets");
vi.mock("../../api/client", () => ({
  fetchBarcaTargets: vi.fn(),
}));

function setupHook(overrides = {}) {
  const hookValue = {
    rows: [
      { symbol: "BTC", group_name: "Core", barca: "Base", asset_class: "crypto", target_percent: 60, current_quantity: 1, last_price: 10, notes: "" },
      { symbol: "ETH", group_name: "Core", barca: "Base", asset_class: "crypto", target_percent: 40, current_quantity: 2, last_price: 5, notes: "" },
    ],
    loading: false,
    error: "",
    save: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  };
  usePortfolioTargets.mockReturnValue(hookValue);
  return hookValue;
}

describe("PortfolioTargetsTab", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    api.fetchBarcaTargets.mockResolvedValue([]);
  });

  it("lists existing rows", () => {
    setupHook();
    render(<PortfolioTargetsTab active />);
    expect(screen.getByDisplayValue("BTC")).toBeInTheDocument();
    expect(screen.getByDisplayValue("ETH")).toBeInTheDocument();
  });

  it("shows a valid 100% sum and enables Save", () => {
    setupHook();
    render(<PortfolioTargetsTab active />);
    expect(screen.getByText("Sum: 100.00%")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Save All" })).toBeEnabled();
  });

  it("disables Save when the sum is not 100%", async () => {
    const user = userEvent.setup();
    setupHook();
    render(<PortfolioTargetsTab active />);

    const btcRow = screen.getByDisplayValue("BTC").closest("tr");
    const targetInput = within(btcRow).getAllByRole("spinbutton")[0];
    await user.clear(targetInput);
    await user.type(targetInput, "50");

    expect(screen.getByText("Sum: 90.00%")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Save All" })).toBeDisabled();
  });

  it("saves existing rows with quantity/notes zeroed out, never the fetched value", async () => {
    // Regression coverage: current_quantity/notes on a fetched row can be an
    // aggregate across several distinct ledger entries (see the component's
    // doc comment). Resubmitting that aggregate as-is would silently double
    // it, so existing rows must always save quantity=0/notes=null.
    const user = userEvent.setup();
    const hook = setupHook();
    render(<PortfolioTargetsTab active />);

    await user.click(screen.getByRole("button", { name: "Save All" }));

    expect(hook.save).toHaveBeenCalledWith([
      expect.objectContaining({ symbol: "BTC", target_percent: 60, current_quantity: 0, notes: null }),
      expect.objectContaining({ symbol: "ETH", target_percent: 40, current_quantity: 0, notes: null }),
    ]);
  });

  it("quantity and notes are disabled for existing rows", () => {
    setupHook();
    render(<PortfolioTargetsTab active />);

    const btcRow = screen.getByDisplayValue("BTC").closest("tr");
    const [quantityInput] = within(btcRow).getAllByRole("spinbutton").slice(1); // [target%, quantity]
    expect(quantityInput).toBeDisabled();
  });

  it("removing a row drops it from the table, the sum, and the saved payload", async () => {
    const user = userEvent.setup();
    const hook = setupHook();
    render(<PortfolioTargetsTab active />);

    const btcRow = screen.getByDisplayValue("BTC").closest("tr");
    await user.click(within(btcRow).getByRole("button", { name: "Remove" }));

    expect(screen.queryByDisplayValue("BTC")).not.toBeInTheDocument();
    expect(screen.getByText("Sum: 40.00%")).toBeInTheDocument(); // only ETH's 40 left
    expect(screen.getByRole("button", { name: "Save All" })).toBeDisabled(); // no longer sums to 100

    // Re-add ETH's missing 60% some other way isn't the point here — just
    // confirm a save reflects the removal (single remaining row at 100%).
    const ethRow = screen.getByDisplayValue("ETH").closest("tr");
    const ethTarget = within(ethRow).getAllByRole("spinbutton")[0];
    await user.clear(ethTarget);
    await user.type(ethTarget, "100");
    await user.click(screen.getByRole("button", { name: "Save All" }));

    expect(hook.save).toHaveBeenCalledWith([expect.objectContaining({ symbol: "ETH", target_percent: 100 })]);
  });

  it("adding a new row requires a symbol before Save is enabled", async () => {
    const user = userEvent.setup();
    setupHook();
    render(<PortfolioTargetsTab active />);

    await user.click(screen.getByRole("button", { name: "+ Add Asset" }));
    expect(screen.getByRole("button", { name: "Save All" })).toBeDisabled();

    const symbolInputs = screen.getAllByDisplayValue("");
    await user.type(symbolInputs[0], "HGRU11");
    // still invalid: new row's target defaults to 0, breaking the 100% sum
    expect(screen.getByRole("button", { name: "Save All" })).toBeDisabled();
  });

  it("a brand-new row's quantity and notes are editable and saved as typed", async () => {
    const user = userEvent.setup();
    const hook = setupHook({
      rows: [{ symbol: "BTC", group_name: "Core", barca: "Base", asset_class: "crypto", target_percent: 0, current_quantity: 1, last_price: 10, notes: "" }],
    });
    render(<PortfolioTargetsTab active />);

    await user.click(screen.getByRole("button", { name: "+ Add Asset" }));
    const dataRows = screen.getAllByRole("row").slice(1); // drop the header row
    const newRow = dataRows[dataRows.length - 1];
    const newRowSymbol = within(newRow).getAllByRole("textbox")[0];
    await user.type(newRowSymbol, "HGRU11");
    const quantityInput = within(newRow).getAllByRole("spinbutton")[1];
    expect(quantityInput).toBeEnabled();
    await user.clear(quantityInput);
    await user.type(quantityInput, "10");

    const targetInput = within(newRow).getAllByRole("spinbutton")[0];
    await user.clear(targetInput);
    await user.type(targetInput, "100");

    const btcRow = screen.getByDisplayValue("BTC").closest("tr");
    const btcTarget = within(btcRow).getAllByRole("spinbutton")[0];
    await user.clear(btcTarget);
    await user.type(btcTarget, "0");

    await user.click(screen.getByRole("button", { name: "Save All" }));

    expect(hook.save).toHaveBeenCalledWith(
      expect.arrayContaining([expect.objectContaining({ symbol: "HGRU11", current_quantity: 10, target_percent: 100 })])
    );
  });

  it("suggests existing Group and BARCA values to avoid typos, while still allowing a new one", async () => {
    const user = userEvent.setup();
    setupHook();
    render(<PortfolioTargetsTab active />);

    const btcRow = screen.getByDisplayValue("BTC").closest("tr");
    const groupInput = within(btcRow).getAllByRole("combobox")[0];
    await user.click(groupInput);
    expect(screen.getByRole("option", { name: "Core" })).toBeInTheDocument();

    const barcaInput = within(btcRow).getAllByRole("combobox")[1];
    await user.click(barcaInput);
    expect(screen.getByRole("option", { name: "Base" })).toBeInTheDocument();

    // freeSolo: typing something new is still accepted, not restricted to suggestions.
    await user.clear(barcaInput);
    await user.type(barcaInput, "Renda Variavel");
    expect(barcaInput).toHaveValue("Renda Variavel");
  });

  it("fetches BARCA names from both markets for the barca suggestions", async () => {
    setupHook();
    render(<PortfolioTargetsTab active />);
    await waitFor(() => {
      expect(api.fetchBarcaTargets).toHaveBeenCalledWith("BullMarket");
      expect(api.fetchBarcaTargets).toHaveBeenCalledWith("BearMarket");
    });
  });

  it("surfaces a fetch error from the hook", () => {
    setupHook({ error: "network down" });
    render(<PortfolioTargetsTab active />);
    expect(screen.getByText("network down")).toBeInTheDocument();
  });

  it("shows an empty-state row when there are no rows", () => {
    setupHook({ rows: [] });
    render(<PortfolioTargetsTab active />);
    expect(screen.getByText("No portfolio targets yet.")).toBeInTheDocument();
  });
});
