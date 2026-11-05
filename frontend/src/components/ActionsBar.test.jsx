import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ActionsBar } from "./ActionsBar";
import { fetchRecordedActivity } from "../api/client";

vi.mock("../api/client", () => ({ fetchRecordedActivity: vi.fn() }));
vi.mock("../hooks/useCurrentPortfolio", () => ({ useCurrentPortfolio: () => ({ id: 2, name: "Spouse" }) }));

beforeEach(() => {
  fetchRecordedActivity.mockReset();
  fetchRecordedActivity.mockResolvedValue({ trades: 0, transfers: 0, latest_at: null });
});

const csvFile = (savedAt) =>
  new File(["symbol,group\nBTC,Core\n"], "wallet.csv", { type: "text/csv", lastModified: Date.parse(savedAt) });

describe("ActionsBar", () => {
  it("hides Import Wallet CSV for a role that can't import", () => {
    render(<ActionsBar importing={false} onImport={vi.fn()} canImport={false} accountName="Spouse" />);
    expect(screen.queryByRole("button", { name: "Import CSV into Spouse" })).not.toBeInTheDocument();
  });

  it("shows Import Wallet CSV for a role that can import", () => {
    render(<ActionsBar importing={false} onImport={vi.fn()} canImport={true} accountName="Spouse" />);
    expect(screen.getByRole("button", { name: "Import CSV into Spouse" })).toBeInTheDocument();
  });

  it("calls onImport with the chosen File, not a path string", async () => {
    const user = userEvent.setup();
    const onImport = vi.fn();
    render(<ActionsBar importing={false} onImport={onImport} canImport={true} accountName="Spouse" />);

    const file = new File(["symbol,group\nBTC,Core\n"], "wallet.csv", { type: "text/csv" });
    // The button just opens the native picker; the hidden <input type="file"> is what
    // actually carries the selection in a real browser and in Testing Library alike.
    const fileInput = document.querySelector('input[type="file"]');
    await user.upload(fileInput, file);

    await waitFor(() => expect(onImport).toHaveBeenCalledWith(file));
  });

  it("asks first when the file is older than the latest recorded trade", async () => {
    const user = userEvent.setup();
    const onImport = vi.fn();
    const onExport = vi.fn();
    fetchRecordedActivity.mockResolvedValue({ trades: 1, latest_at: "2026-11-03T17:20:00.000Z" });
    render(<ActionsBar importing={false} onImport={onImport} onExport={onExport} canImport={true} accountName="Spouse" />);

    await user.upload(document.querySelector('input[type="file"]'), csvFile("2026-11-01T10:00:00Z"));

    expect(await screen.findByRole("dialog", { name: "This file may undo trades or moves recorded in Spouse" })).toBeInTheDocument();
    expect(screen.getByText(/move was recorded on 2026-11-03 14:20:00 \(BRT\), after wallet.csv was saved/)).toBeInTheDocument();
    expect(fetchRecordedActivity).toHaveBeenCalledWith(2);
    expect(onImport).not.toHaveBeenCalled();

    await user.click(screen.getByRole("button", { name: "Export current CSV" }));
    expect(onExport).toHaveBeenCalledTimes(1);
    expect(onImport).not.toHaveBeenCalled();
  });

  it("imports an older file only when told to", async () => {
    const user = userEvent.setup();
    const onImport = vi.fn();
    fetchRecordedActivity.mockResolvedValue({ trades: 1, latest_at: "2026-11-03T17:20:00.000Z" });
    render(<ActionsBar importing={false} onImport={onImport} onExport={vi.fn()} canImport={true} accountName="Spouse" />);
    const file = csvFile("2026-11-01T10:00:00Z");

    await user.upload(document.querySelector('input[type="file"]'), file);
    await user.click(await screen.findByRole("button", { name: "Import anyway into Spouse" }));

    expect(onImport).toHaveBeenCalledWith(file);
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  });

  it("imports a file saved after the latest trade without asking", async () => {
    const user = userEvent.setup();
    const onImport = vi.fn();
    fetchRecordedActivity.mockResolvedValue({ trades: 1, latest_at: "2026-11-03T17:20:00.000Z" });
    render(<ActionsBar importing={false} onImport={onImport} canImport={true} accountName="Spouse" />);
    const file = csvFile("2026-11-04T09:00:00Z");

    await user.upload(document.querySelector('input[type="file"]'), file);

    await waitFor(() => expect(onImport).toHaveBeenCalledWith(file));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("asks when the recorded trades can't be checked", async () => {
    const user = userEvent.setup();
    fetchRecordedActivity.mockRejectedValue(new Error("offline"));
    render(<ActionsBar importing={false} onImport={vi.fn()} canImport={true} accountName="Spouse" />);

    await user.upload(document.querySelector('input[type="file"]'), csvFile("2026-11-04T09:00:00Z"));

    expect(await screen.findByText(/Couldn't check the trades and moves recorded in Spouse/)).toBeInTheDocument();
  });

  it("does not call onImport when the file input is cleared without a selection", () => {
    const onImport = vi.fn();
    render(<ActionsBar importing={false} onImport={onImport} canImport={true} accountName="Spouse" />);

    const fileInput = document.querySelector('input[type="file"]');
    fileInput.dispatchEvent(new Event("change", { bubbles: true }));

    expect(onImport).not.toHaveBeenCalled();
  });

  it("has no Update Prices button: prices refresh by themselves", () => {
    render(<ActionsBar importing={false} onImport={vi.fn()} canImport={true} accountName="Spouse" />);
    expect(screen.queryByRole("button", { name: /update prices/i })).not.toBeInTheDocument();
  });

  it("hides Export Wallet CSV for a role that can't export", () => {
    render(<ActionsBar importing={false} onImport={vi.fn()} canImport={false} accountName="Spouse" />);
    expect(screen.queryByRole("button", { name: "Export Wallet CSV" })).not.toBeInTheDocument();
  });

  it("calls onExport when Export Wallet CSV is clicked", async () => {
    const user = userEvent.setup();
    const onExport = vi.fn();
    render(
      <ActionsBar importing={false} onImport={vi.fn()} exporting={false} onExport={onExport} canImport={true} accountName="Spouse" />
    );

    await user.click(screen.getByRole("button", { name: "Export Wallet CSV" }));

    expect(onExport).toHaveBeenCalledTimes(1);
  });

  it("disables Export Wallet CSV while an export is in flight", () => {
    render(
      <ActionsBar importing={false} onImport={vi.fn()} exporting={true} onExport={vi.fn()} canImport={true} accountName="Spouse" />
    );
    // Import, Export — the export button is the second.
    expect(screen.getAllByRole("button")[1]).toBeDisabled();
  });

  it("disables Import Wallet CSV while an import is in flight", () => {
    render(<ActionsBar importing={true} onImport={vi.fn()} canImport={true} accountName="Spouse" />);
    // Import shows a spinner instead of its label, so find it by position.
    const buttons = screen.getAllByRole("button");
    expect(buttons[0]).toBeDisabled();
    expect(buttons[1]).toBeEnabled();
  });
});
