import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AccountsTab } from "./AccountsTab";
import { CurrentPortfolioContext } from "../../hooks/useCurrentPortfolio";
import { PortfolioListContext } from "../../hooks/usePortfolioList";
import * as api from "../../api/client";

vi.mock("../../api/client", () => ({ createPortfolio: vi.fn(), renamePortfolio: vi.fn() }));

const portfolios = [
  { id: 1, name: "Mine", access: "manage" },
  { id: 2, name: "Shared with me", access: "view" },
];

function renderTab() {
  const list = { portfolios, select: vi.fn(), reload: vi.fn() };
  render(
    <PortfolioListContext value={list}>
      <CurrentPortfolioContext value={portfolios[0]}>
        <AccountsTab />
      </CurrentPortfolioContext>
    </PortfolioListContext>
  );
  return list;
}

const row = (name) => screen.getByText(name).closest("tr");

describe("AccountsTab", () => {
  beforeEach(() => vi.clearAllMocks());

  it("lists every account with its access, and only offers Rename where you can manage", () => {
    renderTab();
    expect(within(row("Mine")).getByText("Manage")).toBeInTheDocument();
    expect(within(row("Mine")).getByRole("button", { name: "Rename" })).toBeInTheDocument();
    expect(within(row("Shared with me")).getByText("View only")).toBeInTheDocument();
    expect(within(row("Shared with me")).queryByRole("button", { name: "Rename" })).not.toBeInTheDocument();
  });

  it("creates an account with a copy of the current BARCA targets and opens it", async () => {
    const user = userEvent.setup();
    api.createPortfolio.mockResolvedValue({ id: 3, name: "Sister", access: "manage" });
    const list = renderTab();

    expect(screen.getByRole("button", { name: "Create" })).toBeDisabled();
    await user.type(screen.getByLabelText("Name"), "  Sister ");
    await user.click(screen.getByRole("button", { name: "Create" }));

    expect(api.createPortfolio).toHaveBeenCalledWith("Sister", 1);
    expect(list.reload).toHaveBeenCalledWith(3);
  });

  it("can create an account with empty BARCA targets", async () => {
    const user = userEvent.setup();
    api.createPortfolio.mockResolvedValue({ id: 3, name: "Sister", access: "manage" });
    renderTab();

    await user.type(screen.getByLabelText("Name"), "Sister");
    await user.click(screen.getByLabelText("Start BARCA targets from"));
    await user.click(within(screen.getByRole("listbox")).getByText("Empty"));
    await user.click(screen.getByRole("button", { name: "Create" }));

    expect(api.createPortfolio).toHaveBeenCalledWith("Sister", null);
  });

  it("shows a server error (e.g. a taken name) and doesn't switch", async () => {
    const user = userEvent.setup();
    api.createPortfolio.mockRejectedValue(new Error("a portfolio with this name already exists"));
    const list = renderTab();

    await user.type(screen.getByLabelText("Name"), "Mine");
    await user.click(screen.getByRole("button", { name: "Create" }));

    expect(await screen.findByText("a portfolio with this name already exists")).toBeInTheDocument();
    expect(list.reload).not.toHaveBeenCalled();
  });

  it("renames an account and reloads the list", async () => {
    const user = userEvent.setup();
    api.renamePortfolio.mockResolvedValue({ ok: true });
    const list = renderTab();

    await user.click(within(row("Mine")).getByRole("button", { name: "Rename" }));
    const input = screen.getByLabelText("New name for Mine");
    await user.clear(input);
    await user.type(input, "Roberto ");
    await user.click(screen.getByRole("button", { name: "Save" }));

    expect(api.renamePortfolio).toHaveBeenCalledWith(1, "Roberto");
    expect(list.reload).toHaveBeenCalledWith();
  });
});
