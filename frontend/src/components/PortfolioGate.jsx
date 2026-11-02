import { Box, Button, CircularProgress, Container } from "@mui/material";
import { usePortfolios } from "../hooks/usePortfolios";
import { CurrentPortfolioContext } from "../hooks/useCurrentPortfolio";
import { PortfolioListContext } from "../hooks/usePortfolioList";
import { EmptyState } from "./EmptyState";

// Loads the user's portfolios and renders `children(portfolio)` for the
// selected one, inside CurrentPortfolioContext (and PortfolioListContext,
// for the switcher and the Accounts tab).
//
// The children are keyed by portfolio id: picking another portfolio
// unmounts every tab and starts it fresh, so nothing fetched for the
// previous portfolio (rows, filters, unsaved edits) can be shown or saved
// under the new one.
export function PortfolioGate({ onLogout, children }) {
  const { portfolios, current, select, reload, loading, error } = usePortfolios();

  if (loading) {
    return (
      <Box sx={{ display: "flex", justifyContent: "center", py: 8 }}>
        <CircularProgress />
      </Box>
    );
  }
  if (!current) {
    return (
      <Container maxWidth="sm" sx={{ py: 6 }}>
        <EmptyState
          title={error ? "Could not load your portfolios" : "No portfolio yet"}
          description={error || "You don't have access to any portfolio. Ask a manager to add you to one."}
        />
        <Box sx={{ textAlign: "center" }}>
          <Button variant="outlined" onClick={onLogout}>
            Log out
          </Button>
        </Box>
      </Container>
    );
  }

  return (
    <PortfolioListContext value={{ portfolios, select, reload }}>
      <CurrentPortfolioContext value={current}>
        <Box key={current.id}>{children(current)}</Box>
      </CurrentPortfolioContext>
    </PortfolioListContext>
  );
}
