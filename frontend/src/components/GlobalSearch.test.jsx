import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { GlobalSearch } from "./GlobalSearch";

const pages = [
  { key: "holdings", label: "Holdings" },
  { key: "history", label: "History" },
];
const allocations = [
  { symbol: "ETH", group: "Core", barca: "Altcoins" },
  { symbol: "BTC", group: "Core", barca: "Base" },
];

describe("GlobalSearch", () => {
  it("goes to the asset's row when a result is picked", async () => {
    const user = userEvent.setup();
    const onGo = vi.fn();
    render(<GlobalSearch pages={pages} allocations={allocations} onGo={onGo} />);

    await user.type(screen.getByRole("combobox", { name: /Find an asset/ }), "eth");
    await user.click(await screen.findByRole("option", { name: /ETH/ }));

    expect(onGo).toHaveBeenCalledWith("holdings", { view: "asset", q: "ETH" });
  });

  it("Enter picks the first result", async () => {
    const user = userEvent.setup();
    const onGo = vi.fn();
    render(<GlobalSearch pages={pages} allocations={allocations} onGo={onGo} />);

    await user.type(screen.getByRole("combobox", { name: /Find an asset/ }), "hist{Enter}");

    expect(onGo).toHaveBeenCalledWith("history", {});
  });

  it("says when nothing matches", async () => {
    const user = userEvent.setup();
    render(<GlobalSearch pages={pages} allocations={allocations} onGo={vi.fn()} />);
    await user.type(screen.getByRole("combobox", { name: /Find an asset/ }), "doge");
    expect(await screen.findByText("Nothing matches “doge”")).toBeInTheDocument();
  });

  it("Ctrl/⌘+K focuses the box from anywhere", () => {
    render(<GlobalSearch pages={pages} allocations={allocations} onGo={vi.fn()} />);
    fireEvent.keyDown(window, { key: "k", ctrlKey: true });
    expect(screen.getByRole("combobox", { name: /Find an asset/ })).toHaveFocus();
  });
});
