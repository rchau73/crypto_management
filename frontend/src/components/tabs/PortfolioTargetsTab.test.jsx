import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { PortfolioTargetsTab } from "./PortfolioTargetsTab";
import { usePortfolioTargets } from "../../hooks/usePortfolioTargets";
import * as api from "../../api/client";

vi.mock("../../hooks/usePortfolioTargets");
vi.mock("../../api/client", () => ({
  fetchBarcaTargets: vi.fn(),
  correctWalletQuantity: vi.fn(),
}));

// The row that "+ Add Asset" appended (the last row of the table body).
function lastRow() {
  const dataRows = screen.getAllByRole("row").slice(1); // drop the header row
  return dataRows[dataRows.length - 1];
}

function setupHook(overrides = {}) {
  const hookValue = {
    rows: [
      { symbol: "BTC", group_name: "Core", barca: "Base", asset_class: "crypto", target_percent: 60, current_quantity: 1, last_price: 10, notes: "", source_count: 1 },
      { symbol: "ETH", group_name: "Core", barca: "Base", asset_class: "crypto", target_percent: 40, current_quantity: 2, last_price: 5, notes: "", source_count: 1 },
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
    api.correctWalletQuantity.mockResolvedValue({ ok: true });
  });

  it("lists existing rows", () => {
    setupHook();
    render(<PortfolioTargetsTab />);
    expect(screen.getByDisplayValue("BTC")).toBeInTheDocument();
    expect(screen.getByDisplayValue("ETH")).toBeInTheDocument();
  });

  it("shows a valid 100% sum and enables Save", () => {
    setupHook();
    render(<PortfolioTargetsTab />);
    expect(screen.getByText("Sum: 100.00%")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Save All" })).toBeEnabled();
  });

  it("disables Save when the sum is not 100%", async () => {
    const user = userEvent.setup();
    setupHook();
    render(<PortfolioTargetsTab />);

    const btcRow = screen.getByDisplayValue("BTC").closest("tr");
    const targetInput = within(btcRow).getAllByRole("spinbutton")[0];
    await user.clear(targetInput);
    await user.type(targetInput, "50");

    expect(screen.getByText("Sum: 90.00%")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Save All" })).toBeDisabled();
  });

  it("never sends quantity or notes for existing rows", async () => {
    // Regression coverage: a fetched row's quantity/notes can be a SUM over
    // several ledger sources; sending it back would double the holding.
    const user = userEvent.setup();
    const hook = setupHook();
    render(<PortfolioTargetsTab />);

    await user.click(screen.getByRole("button", { name: "Save All" }));

    const [btc, eth] = hook.save.mock.calls[0][0];
    expect(btc).toEqual({ symbol: "BTC", group_name: "Core", barca: "Base", asset_class: "crypto", target_percent: 60 });
    expect(eth).not.toHaveProperty("current_quantity");
    expect(eth).not.toHaveProperty("notes");
  });

  it("only the target is editable on an existing row", () => {
    // Renaming symbol/group/BARCA/class would move the target away from the
    // holdings it belongs to, so those are read-only once saved.
    setupHook();
    render(<PortfolioTargetsTab />);

    const btcRow = screen.getByDisplayValue("BTC").closest("tr");
    const [targetInput, quantityInput] = within(btcRow).getAllByRole("spinbutton");
    expect(targetInput).toBeEnabled();
    expect(quantityInput).toBeDisabled();
    expect(screen.getByDisplayValue("BTC")).toBeDisabled();
    // Group/BARCA are Autocomplete inputs (disabled attribute); Asset Class
    // is a Select (aria-disabled).
    for (const combobox of within(btcRow).getAllByRole("combobox")) {
      const disabled = combobox.hasAttribute("disabled") || combobox.getAttribute("aria-disabled") === "true";
      expect(disabled).toBe(true);
    }
  });

  it("removing a row drops it from the table, the sum, and the saved payload", async () => {
    const user = userEvent.setup();
    const hook = setupHook();
    render(<PortfolioTargetsTab />);

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

  it("corrects a single-source row's quantity via a dedicated endpoint, updating only that row", async () => {
    const user = userEvent.setup();
    setupHook();
    render(<PortfolioTargetsTab />);

    const btcRow = screen.getByDisplayValue("BTC").closest("tr");
    await user.click(within(btcRow).getByRole("button", { name: "Corrigir" }));

    const correctionInput = within(btcRow).getAllByRole("spinbutton")[1];
    await user.clear(correctionInput);
    await user.type(correctionInput, "42");
    await user.click(within(btcRow).getByRole("button", { name: "Salvar" }));

    await waitFor(() => {
      expect(api.correctWalletQuantity).toHaveBeenCalledWith({
        symbol: "BTC",
        group_name: "Core",
        barca: "Base",
        asset_class: "crypto",
        current_quantity: 42,
      });
    });
    // Back to the normal (non-editing) display, now showing the corrected value.
    expect(within(btcRow).getByRole("button", { name: "Corrigir" })).toBeInTheDocument();
    expect(within(btcRow).getByDisplayValue("42")).toBeInTheDocument();
  });

  it("correcting one row's quantity never discards an unsaved Target % edit on another row", async () => {
    // Regression coverage: this used to call the hook's `refresh()` after a
    // successful correction, which re-fetched and replaced the ENTIRE
    // `rows` state from the server — silently reverting any in-progress
    // batch edit (Target %, Group, BARCA, ...) on every other row that
    // hadn't been submitted via Save All yet.
    const user = userEvent.setup();
    setupHook();
    render(<PortfolioTargetsTab />);

    const ethRow = screen.getByDisplayValue("ETH").closest("tr");
    const ethTarget = within(ethRow).getAllByRole("spinbutton")[0];
    await user.clear(ethTarget);
    await user.type(ethTarget, "77");

    const btcRow = screen.getByDisplayValue("BTC").closest("tr");
    await user.click(within(btcRow).getByRole("button", { name: "Corrigir" }));
    const correctionInput = within(btcRow).getAllByRole("spinbutton")[1];
    await user.clear(correctionInput);
    await user.type(correctionInput, "42");
    await user.click(within(btcRow).getByRole("button", { name: "Salvar" }));

    await waitFor(() => {
      expect(api.correctWalletQuantity).toHaveBeenCalled();
    });
    expect(ethTarget).toHaveValue(77);
  });

  it("Cancelar closes the correction control without calling the API", async () => {
    const user = userEvent.setup();
    setupHook();
    render(<PortfolioTargetsTab />);

    const btcRow = screen.getByDisplayValue("BTC").closest("tr");
    await user.click(within(btcRow).getByRole("button", { name: "Corrigir" }));
    await user.click(within(btcRow).getByRole("button", { name: "Cancelar" }));

    expect(api.correctWalletQuantity).not.toHaveBeenCalled();
    expect(within(btcRow).getByRole("button", { name: "Corrigir" })).toBeInTheDocument();
  });

  it("an invalid correction value disables Salvar instead of saving 0", async () => {
    const user = userEvent.setup();
    setupHook();
    render(<PortfolioTargetsTab />);

    const btcRow = screen.getByDisplayValue("BTC").closest("tr");
    await user.click(within(btcRow).getByRole("button", { name: "Corrigir" }));
    const correctionInput = within(btcRow).getAllByRole("spinbutton")[1];
    await user.clear(correctionInput);
    await user.type(correctionInput, "-5");

    expect(within(btcRow).getByRole("button", { name: "Salvar" })).toBeDisabled();
    await user.clear(correctionInput);
    expect(within(btcRow).getByRole("button", { name: "Salvar" })).toBeDisabled();
  });

  it("shows a server error from a refused correction inline", async () => {
    const user = userEvent.setup();
    api.correctWalletQuantity.mockRejectedValue(new Error("BTC is held in 2 sources"));
    setupHook();
    render(<PortfolioTargetsTab />);

    const btcRow = screen.getByDisplayValue("BTC").closest("tr");
    await user.click(within(btcRow).getByRole("button", { name: "Corrigir" }));
    await user.click(within(btcRow).getByRole("button", { name: "Salvar" }));

    expect(await within(btcRow).findByText("BTC is held in 2 sources")).toBeInTheDocument();
  });

  it("a multi-source row shows 'multi-fonte' even when its notes have no ' | '", () => {
    // Regression: a NULL-notes source is invisible in the joined notes, so
    // the old " | " check offered Corrigir on 2-source rows.
    setupHook({
      rows: [
        {
          symbol: "BTC",
          group_name: "Core",
          barca: "Base",
          asset_class: "crypto",
          target_percent: 100,
          current_quantity: 3,
          last_price: 10,
          notes: "Binance",
          source_count: 2,
        },
      ],
    });
    render(<PortfolioTargetsTab />);

    const btcRow = screen.getByDisplayValue("BTC").closest("tr");
    expect(within(btcRow).queryByRole("button", { name: "Corrigir" })).not.toBeInTheDocument();
    expect(within(btcRow).getByText("multi-fonte")).toBeInTheDocument();
  });

  it("adding a new row requires a symbol before Save is enabled", async () => {
    const user = userEvent.setup();
    setupHook();
    render(<PortfolioTargetsTab />);

    await user.click(screen.getByRole("button", { name: "+ Add Asset" }));
    expect(screen.getByRole("button", { name: "Save All" })).toBeDisabled();

    await user.type(within(lastRow()).getAllByRole("textbox")[0], "HGRU11");
    // Valid now: 60 + 40 + the new row's default 0% still sums to 100.
    expect(screen.getByRole("button", { name: "Save All" })).toBeEnabled();
  });

  it("a brand-new row's quantity and notes are editable and saved as typed", async () => {
    const user = userEvent.setup();
    const hook = setupHook({
      rows: [{ symbol: "BTC", group_name: "Core", barca: "Base", asset_class: "crypto", target_percent: 0, current_quantity: 1, last_price: 10, notes: "" }],
    });
    render(<PortfolioTargetsTab />);

    await user.click(screen.getByRole("button", { name: "+ Add Asset" }));
    const newRow = lastRow();
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
    render(<PortfolioTargetsTab />);
    await user.click(screen.getByRole("button", { name: "+ Add Asset" }));
    const newRow = lastRow();

    const groupInput = within(newRow).getAllByRole("combobox")[0];
    await user.click(groupInput);
    expect(screen.getByRole("option", { name: "Core" })).toBeInTheDocument();

    const barcaInput = within(newRow).getAllByRole("combobox")[1];
    await user.click(barcaInput);
    expect(screen.getByRole("option", { name: "Base" })).toBeInTheDocument();

    // freeSolo: typing something new is still accepted, not restricted to suggestions.
    await user.clear(barcaInput);
    await user.type(barcaInput, "Renda Variavel");
    expect(barcaInput).toHaveValue("Renda Variavel");
  });

  it("fetches BARCA names from both markets for the barca suggestions", async () => {
    setupHook();
    render(<PortfolioTargetsTab />);
    await waitFor(() => {
      expect(api.fetchBarcaTargets).toHaveBeenCalledWith("BullMarket");
      expect(api.fetchBarcaTargets).toHaveBeenCalledWith("BearMarket");
    });
  });

  it("surfaces a fetch error from the hook", () => {
    setupHook({ error: "network down" });
    render(<PortfolioTargetsTab />);
    expect(screen.getByText("network down")).toBeInTheDocument();
  });

  it("shows an empty-state row when there are no rows", () => {
    setupHook({ rows: [] });
    render(<PortfolioTargetsTab />);
    expect(screen.getByText("No portfolio targets yet.")).toBeInTheDocument();
  });
});
