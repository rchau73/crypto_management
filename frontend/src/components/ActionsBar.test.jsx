import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ActionsBar } from "./ActionsBar";

describe("ActionsBar", () => {
  it("hides Import Wallet CSV for a role that can't import", () => {
    render(<ActionsBar importing={false} onImport={vi.fn()} canImport={false} />);
    expect(screen.queryByRole("button", { name: "Import Wallet CSV" })).not.toBeInTheDocument();
  });

  it("shows Import Wallet CSV for a role that can import", () => {
    render(<ActionsBar importing={false} onImport={vi.fn()} canImport={true} />);
    expect(screen.getByRole("button", { name: "Import Wallet CSV" })).toBeInTheDocument();
  });

  it("calls onImport with the chosen File, not a path string", async () => {
    const user = userEvent.setup();
    const onImport = vi.fn();
    render(<ActionsBar importing={false} onImport={onImport} canImport={true} />);

    const file = new File(["symbol,group\nBTC,Core\n"], "wallet.csv", { type: "text/csv" });
    // The button just opens the native picker; the hidden <input type="file"> is what
    // actually carries the selection in a real browser and in Testing Library alike.
    const fileInput = document.querySelector('input[type="file"]');
    await user.upload(fileInput, file);

    expect(onImport).toHaveBeenCalledWith(file);
  });

  it("does not call onImport when the file input is cleared without a selection", () => {
    const onImport = vi.fn();
    render(<ActionsBar importing={false} onImport={onImport} canImport={true} />);

    const fileInput = document.querySelector('input[type="file"]');
    fileInput.dispatchEvent(new Event("change", { bubbles: true }));

    expect(onImport).not.toHaveBeenCalled();
  });

  it("has no Update Prices button: prices refresh by themselves", () => {
    render(<ActionsBar importing={false} onImport={vi.fn()} canImport={true} />);
    expect(screen.queryByRole("button", { name: /update prices/i })).not.toBeInTheDocument();
  });

  it("hides Export Wallet CSV for a role that can't export", () => {
    render(<ActionsBar importing={false} onImport={vi.fn()} canImport={false} />);
    expect(screen.queryByRole("button", { name: "Export Wallet CSV" })).not.toBeInTheDocument();
  });

  it("calls onExport when Export Wallet CSV is clicked", async () => {
    const user = userEvent.setup();
    const onExport = vi.fn();
    render(
      <ActionsBar importing={false} onImport={vi.fn()} exporting={false} onExport={onExport} canImport={true} />
    );

    await user.click(screen.getByRole("button", { name: "Export Wallet CSV" }));

    expect(onExport).toHaveBeenCalledTimes(1);
  });

  it("disables Export Wallet CSV while an export is in flight", () => {
    render(
      <ActionsBar importing={false} onImport={vi.fn()} exporting={true} onExport={vi.fn()} canImport={true} />
    );
    // Import, Export — the export button is the second.
    expect(screen.getAllByRole("button")[1]).toBeDisabled();
  });

  it("disables Import Wallet CSV while an import is in flight", () => {
    render(<ActionsBar importing={true} onImport={vi.fn()} canImport={true} />);
    // Import shows a spinner instead of its label, so find it by position.
    const buttons = screen.getAllByRole("button");
    expect(buttons[0]).toBeDisabled();
    expect(buttons[1]).toBeEnabled();
  });
});
