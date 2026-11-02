import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { PortfolioGate } from "./PortfolioGate";
import { useCurrentPortfolio } from "../hooks/useCurrentPortfolio";
import * as api from "../api/client";

vi.mock("../api/client", () => ({ fetchPortfolios: vi.fn() }));

function ShowsCurrent() {
  const portfolio = useCurrentPortfolio();
  return <p>current: {portfolio.name}</p>;
}

describe("PortfolioGate", () => {
  beforeEach(() => vi.clearAllMocks());

  it("provides the first portfolio to everything inside it", async () => {
    api.fetchPortfolios.mockResolvedValue([
      { id: 1, name: "Roberto", access: "manage" },
      { id: 2, name: "Spouse", access: "manage" },
    ]);
    render(<PortfolioGate>{(p) => <ShowsCurrent key={p.id} />}</PortfolioGate>);
    expect(await screen.findByText("current: Roberto")).toBeInTheDocument();
  });

  it("explains an empty list and still lets the user log out", async () => {
    const user = userEvent.setup();
    const onLogout = vi.fn();
    api.fetchPortfolios.mockResolvedValue([]);
    render(<PortfolioGate onLogout={onLogout}>{() => <ShowsCurrent />}</PortfolioGate>);

    expect(await screen.findByText("No portfolio yet")).toBeInTheDocument();
    expect(screen.queryByText(/current:/)).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Log out" }));
    expect(onLogout).toHaveBeenCalled();
  });

  it("shows a load failure instead of an app with no portfolio", async () => {
    api.fetchPortfolios.mockRejectedValue(new Error("network down"));
    render(<PortfolioGate onLogout={vi.fn()}>{() => <ShowsCurrent />}</PortfolioGate>);
    expect(await screen.findByText("network down")).toBeInTheDocument();
  });
});

describe("useCurrentPortfolio", () => {
  it("fails loudly outside PortfolioGate instead of calling the API with no id", () => {
    vi.spyOn(console, "error").mockImplementation(() => {});
    expect(() => render(<ShowsCurrent />)).toThrow("inside PortfolioGate");
  });
});
