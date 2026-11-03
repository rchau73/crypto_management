import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { StatTiles } from "./StatTiles";

describe("StatTiles", () => {
  it("converts the total to reais at the PTAX rate and says which one", () => {
    render(<StatTiles totalWalletValue={1000} usdBrl={{ brl_per_usd: 5.2, quoted_at: "2026-10-30 13:09:00" }} />);
    expect(screen.getByText("R$ 5,200.00")).toBeInTheDocument();
    expect(screen.getByText("At R$ 5.2000 per US$ (PTAX, 2026-10-30 13:09:00)")).toBeInTheDocument();
  });

  it("shows no R$ total without a rate instead of guessing one", () => {
    // Regression: it used a hard-coded 5.6.
    render(<StatTiles totalWalletValue={1000} usdBrl={null} />);
    expect(screen.getByText("—")).toBeInTheDocument();
    expect(screen.queryByText(/R\$ 5,600/)).not.toBeInTheDocument();
    expect(screen.getByText("USD/BRL rate unavailable right now")).toBeInTheDocument();
  });
});
