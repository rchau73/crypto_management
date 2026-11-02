import { FormControl, MenuItem, Select, Typography } from "@mui/material";
import { useCurrentPortfolio } from "../hooks/useCurrentPortfolio";
import { usePortfolioList } from "../hooks/usePortfolioList";

// The account switcher in the header. Always shows which portfolio you're
// looking at; becomes a dropdown once there is more than one. Selecting
// another one remounts the whole app for it (see PortfolioGate).
export function PortfolioSwitcher() {
  const current = useCurrentPortfolio();
  const { portfolios, select } = usePortfolioList();

  if (portfolios.length < 2) {
    return (
      <Typography variant="body2" sx={{ fontWeight: 600 }}>
        {current.name}
      </Typography>
    );
  }
  return (
    <FormControl size="small" sx={{ minWidth: 160 }}>
      <Select
        value={current.id}
        onChange={(e) => select(e.target.value)}
        inputProps={{ "aria-label": "Portfolio" }}
      >
        {portfolios.map((p) => (
          <MenuItem key={p.id} value={p.id}>
            {p.name}
            {p.access === "view" ? " (view only)" : ""}
          </MenuItem>
        ))}
      </Select>
    </FormControl>
  );
}
