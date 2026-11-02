import { AppBar, Button, Stack, Toolbar, Typography } from "@mui/material";
import { ActionsBar } from "./ActionsBar";
import { PortfolioSwitcher } from "./PortfolioSwitcher";

// Sticky top bar: title + the two primary actions stay reachable while
// scrolling a long table, instead of scrolling away with the rest of the page.
export function AppHeader({
  loading,
  onRefresh,
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
        <ActionsBar
          loading={loading}
          onRefresh={onRefresh}
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
