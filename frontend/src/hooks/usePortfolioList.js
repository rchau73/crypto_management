import { createContext, useContext } from "react";

// The user's whole portfolio list plus `select(id)` and `reload(thenSelect)`,
// for the account switcher and the Accounts tab. Provided by PortfolioGate.
// Data hooks should use useCurrentPortfolio instead.
export const PortfolioListContext = createContext(null);

export function usePortfolioList() {
  const list = useContext(PortfolioListContext);
  if (!list) throw new Error("usePortfolioList must be used inside PortfolioGate");
  return list;
}
