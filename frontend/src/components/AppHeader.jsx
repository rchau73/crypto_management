import { AppBar, Button, IconButton, Stack, Toolbar, Typography } from "@mui/material";
import { ActionsBar } from "./ActionsBar";
import { PortfolioSwitcher } from "./PortfolioSwitcher";
import { PricesAge } from "./PricesAge";

function MenuIcon() {
  return (
    <svg width="22" height="22" viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
      <path d="M4 6h16M4 12h16M4 18h16" />
    </svg>
  );
}

// Sticky top bar: the account you're in, the search box, how fresh the
// prices are, and the wallet file actions stay reachable from every page.
// `onOpenMenu` is set on narrow screens, where the side menu is a drawer.
export function AppHeader({
  onOpenMenu,
  search,
  pricesAsOf,
  refreshIntervalSecs,
  accountName,
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
        <Stack direction="row" spacing={1.5} sx={{ alignItems: "center" }}>
          {onOpenMenu && (
            <IconButton onClick={onOpenMenu} aria-label="Open menu" sx={{ color: "text.primary" }}>
              <MenuIcon />
            </IconButton>
          )}
          <Typography variant="h6" sx={{ fontWeight: 700, whiteSpace: "nowrap" }}>
            Wallet Allocations
          </Typography>
          <PortfolioSwitcher />
        </Stack>
        {search}
        <Stack direction="row" useFlexGap spacing={2} sx={{ alignItems: "center", flexWrap: "wrap", ml: "auto" }}>
          <PricesAge pricesAsOf={pricesAsOf} refreshIntervalSecs={refreshIntervalSecs} />
          <ActionsBar
            accountName={accountName}
            importing={importing}
            onImport={onImport}
            exporting={exporting}
            onExport={onExport}
            canImport={canImport}
          />
          <Stack direction="row" spacing={1} sx={{ alignItems: "center" }}>
            <Typography variant="body2" sx={{ color: "text.secondary" }}>
              {username} ({role})
            </Typography>
            <Button size="small" variant="text" onClick={onLogout}>
              Log Out
            </Button>
          </Stack>
        </Stack>
      </Toolbar>
    </AppBar>
  );
}
