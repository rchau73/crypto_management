import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { BarcaTargetsTab } from "./BarcaTargetsTab";
import { useBarcaTargets } from "../../hooks/useBarcaTargets";

vi.mock("../../hooks/useBarcaTargets");

function setupHook(overrides = {}) {
  const hookValue = {
    targets: [
      { barca: "Base", target_percent: 60 },
      { barca: "Altcoins", target_percent: 40 },
    ],
    loading: false,
    error: "",
    save: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  };
  useBarcaTargets.mockReturnValue(hookValue);
  return hookValue;
}

describe("BarcaTargetsTab", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("lists existing targets for the default market", () => {
    setupHook();
    render(<BarcaTargetsTab active />);
    expect(useBarcaTargets).toHaveBeenCalledWith(true, "BullMarket");
    expect(screen.getByDisplayValue("Base")).toBeInTheDocument();
    expect(screen.getByDisplayValue("Altcoins")).toBeInTheDocument();
  });

  it("re-queries the hook with the newly selected market", async () => {
    const user = userEvent.setup();
    setupHook();
    render(<BarcaTargetsTab active />);

    await user.click(screen.getByRole("combobox"));
    await user.click(screen.getByRole("option", { name: "BearMarket" }));

    expect(useBarcaTargets).toHaveBeenLastCalledWith(true, "BearMarket");
  });

  it("shows a valid 100% sum and enables Save", () => {
    setupHook();
    render(<BarcaTargetsTab active />);
    expect(screen.getByText("Sum: 100.00%")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Save All" })).toBeEnabled();
  });

  it("disables Save when the sum is not 100%", async () => {
    const user = userEvent.setup();
    setupHook();
    render(<BarcaTargetsTab active />);

    const baseRow = screen.getByDisplayValue("Base").closest("tr");
    const targetInput = within(baseRow).getByRole("spinbutton");
    await user.clear(targetInput);
    await user.type(targetInput, "50");

    expect(screen.getByText("Sum: 90.00%")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Save All" })).toBeDisabled();
  });

  it("saves the edited, trimmed set of targets", async () => {
    const user = userEvent.setup();
    const hook = setupHook();
    render(<BarcaTargetsTab active />);

    await user.click(screen.getByRole("button", { name: "Save All" }));

    expect(hook.save).toHaveBeenCalledWith([
      { barca: "Base", target_percent: 60 },
      { barca: "Altcoins", target_percent: 40 },
    ]);
  });

  it("adding a new row starts at 0% and breaks the sum until edited", async () => {
    const user = userEvent.setup();
    setupHook();
    render(<BarcaTargetsTab active />);

    await user.click(screen.getByRole("button", { name: "+ Add BARCA" }));
    expect(screen.getByText("Sum: 100.00%")).toBeInTheDocument(); // new row is 0%, sum unchanged
    expect(screen.getByRole("button", { name: "Save All" })).toBeDisabled(); // blank barca name

    const newRowNameInputs = screen.getAllByRole("textbox");
    const blankInput = newRowNameInputs.find((el) => el.value === "");
    await user.type(blankInput, "IBOVE");
    expect(screen.getByRole("button", { name: "Save All" })).toBeEnabled();
  });

  it("removing a row updates the sum", async () => {
    const user = userEvent.setup();
    setupHook();
    render(<BarcaTargetsTab active />);

    const baseRow = screen.getByDisplayValue("Base").closest("tr");
    await user.click(within(baseRow).getByRole("button", { name: "Remove" }));

    expect(screen.getByText("Sum: 40.00%")).toBeInTheDocument();
  });

  it("surfaces a fetch error from the hook", () => {
    setupHook({ error: "network down" });
    render(<BarcaTargetsTab active />);
    expect(screen.getByText("network down")).toBeInTheDocument();
  });

  it("shows an empty-state row when there are no targets for this market", () => {
    setupHook({ targets: [] });
    render(<BarcaTargetsTab active />);
    expect(screen.getByText("No BARCA targets for BullMarket yet.")).toBeInTheDocument();
  });
});
