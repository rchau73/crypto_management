import { createContext, useContext } from "react";

// The portfolio (separate wallet) every tab is showing: { id, name, access }.
// Provided once by PortfolioGate; data hooks read `id` from here and pass it
// to the API client, so no tab can fetch another portfolio's data.
export const CurrentPortfolioContext = createContext(null);

export function useCurrentPortfolio() {
  const portfolio = useContext(CurrentPortfolioContext);
  if (!portfolio) {
    // A component outside PortfolioGate would otherwise call the API with
    // an undefined portfolio id.
    throw new Error("useCurrentPortfolio must be used inside PortfolioGate");
  }
  return portfolio;
}
