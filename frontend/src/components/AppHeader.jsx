import { AppBar, Button, Stack, Toolbar, Typography } from "@mui/material";
import { ActionsBar } from "./ActionsBar";
import { PortfolioSwitcher } from "./PortfolioSwitcher";
import { PricesAge } from "./PricesAge";

// Sticky top bar: title, account, how fresh the prices are, and the wallet
// file actions stay reachable while scrolling a long table.
export function AppHeader({
  pricesAsOf,
  refreshIntervalSecs,
  importing,
  onImport,
  exporting,
  onExport,
  canImport,
  username,
  role,
  onLogout,
}) {
  return (
    <AppBar
      position="sticky"
      color="transparent"
      elevation={0}
      sx={{
        backdropFilter: "blur(8px)",
        bgcolor: "rgba(13, 13, 13, 0.85)",
        borderBottom: "1px solid",
        borderColor: "divider",
      }}
    >
      <Toolbar sx={{ flexWrap: "wrap", gap: 2, py: 1.5 }}>
        <Stack direction="row" spacing={2} alignItems="center" sx={{ mr: "auto" }}>
          <Typography variant="h6" sx={{ fontWeight: 700 }}>
            Wallet Allocations
          </Typography>
          <PortfolioSwitcher />
        </Stack>
        <PricesAge pricesAsOf={pricesAsOf} refreshIntervalSecs={refreshIntervalSecs} />
        <ActionsBar
          importing={importing}
          onImport={onImport}
          exporting={exporting}
          onExport={onExport}
          canImport={canImport}
        />
        <Stack direction="row" spacing={1} alignItems="center">
          <Typography variant="body2" sx={{ color: "text.secondary" }}>
            {username} ({role})
          </Typography>
          <Button size="small" variant="text" onClick={onLogout}>
            Log Out
          </Button>
        </Stack>
      </Toolbar>
    </AppBar>
  );
}
