import { Box, Paper, Typography } from "@mui/material";
import { formatNumber } from "../utils/formatters";

// USDT/BRL is not part of the API response yet; hardcoded here until the
// backend exposes a real rate.
const USDT_BRL_RATE = 5.6;

function StatTile({ label, value, accent }) {
  return (
    <Paper
      variant="outlined"
      sx={{ px: 2.5, py: 1.5, minWidth: 220, borderColor: "divider", bgcolor: "background.paper" }}
    >
      <Typography variant="caption" sx={{ color: "text.secondary", textTransform: "uppercase", letterSpacing: 0.5 }}>
        {label}
      </Typography>
      <Typography variant="h5" sx={{ fontWeight: 700, color: accent, fontVariantNumeric: "tabular-nums" }}>
        {value}
      </Typography>
    </Paper>
  );
}

export function StatTiles({ totalWalletValue }) {
  const totalWalletValueBrl = totalWalletValue * USDT_BRL_RATE;

  return (
    <Box sx={{ display: "flex", gap: 2, mb: 3, flexWrap: "wrap" }}>
      <StatTile label="Total Wallet Value" value={`$${formatNumber(totalWalletValue)}`} accent="primary.main" />
      <StatTile label="Total in R$" value={`R$ ${formatNumber(totalWalletValueBrl)}`} accent="secondary.main" />
    </Box>
  );
}
