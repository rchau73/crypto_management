import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ActionsBar } from "./ActionsBar";

describe("ActionsBar", () => {
  it("hides Import Wallet CSV for a role that can't import", () => {
    render(<ActionsBar loading={false} onRefresh={vi.fn()} importing={false} onImport={vi.fn()} canImport={false} />);
    expect(screen.queryByRole("button", { name: "Import Wallet CSV" })).not.toBeInTheDocument();
  });

  it("shows Import Wallet CSV for a role that can import", () => {
    render(<ActionsBar loading={false} onRefresh={vi.fn()} importing={false} onImport={vi.fn()} canImport={true} />);
    expect(screen.getByRole("button", { name: "Import Wallet CSV" })).toBeInTheDocument();
  });

  it("calls onImport with the chosen File, not a path string", async () => {
    const user = userEvent.setup();
    const onImport = vi.fn();
    render(<ActionsBar loading={false} onRefresh={vi.fn()} importing={false} onImport={onImport} canImport={true} />);

    const file = new File(["symbol,group\nBTC,Core\n"], "wallet.csv", { type: "text/csv" });
    // The button just opens the native picker; the hidden <input type="file"> is what
    // actually carries the selection in a real browser and in Testing Library alike.
    const fileInput = document.querySelector('input[type="file"]');
    await user.upload(fileInput, file);

    expect(onImport).toHaveBeenCalledWith(file);
  });

  it("does not call onImport when the file input is cleared without a selection", () => {
    const onImport = vi.fn();
    render(<ActionsBar loading={false} onRefresh={vi.fn()} importing={false} onImport={onImport} canImport={true} />);

    const fileInput = document.querySelector('input[type="file"]');
    fileInput.dispatchEvent(new Event("change", { bubbles: true }));

    expect(onImport).not.toHaveBeenCalled();
  });

  it("disables Update Prices while loading, and shows a spinner in place of the label", () => {
    render(<ActionsBar loading={true} onRefresh={vi.fn()} importing={false} onImport={vi.fn()} canImport={false} />);
    const button = screen.getByRole("button");
    expect(button).toBeDisabled();
    expect(screen.queryByText("Update Prices")).not.toBeInTheDocument();
  });

  it("disables Import Wallet CSV while an import is in flight", () => {
    render(<ActionsBar loading={false} onRefresh={vi.fn()} importing={true} onImport={vi.fn()} canImport={true} />);
    // Two buttons render (Update Prices + Import Wallet CSV); the second is disabled and shows a spinner.
    const buttons = screen.getAllByRole("button");
    expect(buttons[1]).toBeDisabled();
  });
});
