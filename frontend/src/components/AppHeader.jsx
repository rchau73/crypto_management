import { AppBar, Toolbar, Typography } from "@mui/material";
import { ActionsBar } from "./ActionsBar";

// Sticky top bar: title + the two primary actions stay reachable while
// scrolling a long table, instead of scrolling away with the rest of the page.
export function AppHeader({ loading, onRefresh, importing, onImport }) {
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
        <Typography variant="h6" sx={{ fontWeight: 700, mr: "auto" }}>
          Wallet Allocations
        </Typography>
        <ActionsBar loading={loading} onRefresh={onRefresh} importing={importing} onImport={onImport} />
      </Toolbar>
    </AppBar>
  );
}
