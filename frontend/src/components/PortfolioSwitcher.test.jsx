import { useState } from "react";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { PortfolioGate } from "./PortfolioGate";
import { PortfolioSwitcher } from "./PortfolioSwitcher";
import * as api from "../api/client";

vi.mock("../api/client", () => ({ fetchPortfolios: vi.fn() }));

// Stands in for a tab: holds local state that must not survive a switch.
function FakeTab({ portfolio }) {
  const [draft, setDraft] = useState("");
  return (
    <div>
      <PortfolioSwitcher />
      <p>showing {portfolio.name}</p>
      <input aria-label="draft" value={draft} onChange={(e) => setDraft(e.target.value)} />
    </div>
  );
}

function renderApp() {
  return render(<PortfolioGate onLogout={vi.fn()}>{(p) => <FakeTab portfolio={p} />}</PortfolioGate>);
}

const TWO = [
  { id: 1, name: "Mine", access: "manage" },
  { id: 2, name: "Spouse", access: "view" },
];

describe("PortfolioSwitcher", () => {
  beforeEach(() => vi.clearAllMocks());
  afterEach(() => window.localStorage.clear());

  it("only shows the name when there is a single portfolio", async () => {
    api.fetchPortfolios.mockResolvedValue([TWO[0]]);
    renderApp();
    expect(await screen.findByText("showing Mine")).toBeInTheDocument();
    expect(screen.queryByRole("combobox", { name: "Portfolio" })).not.toBeInTheDocument();
  });

  it("switching starts the app fresh for the other portfolio", async () => {
    const user = userEvent.setup();
    api.fetchPortfolios.mockResolvedValue(TWO);
    renderApp();
    await screen.findByText("showing Mine");
    await user.type(screen.getByLabelText("draft"), "unsaved edit for Mine");

    await user.click(screen.getByRole("combobox", { name: "Portfolio" }));
    await user.click(within(screen.getByRole("listbox")).getByText("Spouse (view only)"));

    expect(screen.getByText("showing Spouse")).toBeInTheDocument();
    expect(screen.getByLabelText("draft")).toHaveValue("");
  });

  it("remembers the selection for the next visit", async () => {
    const user = userEvent.setup();
    api.fetchPortfolios.mockResolvedValue(TWO);
    const first = renderApp();
    await screen.findByText("showing Mine");
    await user.click(screen.getByRole("combobox", { name: "Portfolio" }));
    await user.click(within(screen.getByRole("listbox")).getByText("Spouse (view only)"));
    first.unmount();

    renderApp();
    expect(await screen.findByText("showing Spouse")).toBeInTheDocument();
  });

  it("a remembered portfolio you lost access to falls back to the first", async () => {
    window.localStorage.setItem("cm.selectedPortfolioId", "2");
    api.fetchPortfolios.mockResolvedValue([TWO[0]]);
    renderApp();
    expect(await screen.findByText("showing Mine")).toBeInTheDocument();
  });
});
