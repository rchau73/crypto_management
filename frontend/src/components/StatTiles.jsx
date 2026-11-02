import { Box, Paper, Typography } from "@mui/material";
import { formatNumber } from "../utils/formatters";

function StatTile({ label, value, accent, caption }) {
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
      {caption && (
        <Typography variant="caption" component="div" sx={{ color: "text.secondary" }}>
          {caption}
        </Typography>
      )}
    </Paper>
  );
}

// `usdBrl` is the Banco Central's PTAX rate of the current prices
// ({ brl_per_usd, quoted_at }), or null before they load or when the rate
// couldn't be fetched. Without it there is no R$ total: a guessed rate
// would show a wrong number with no warning.
export function StatTiles({ totalWalletValue, usdBrl }) {
  const rate = usdBrl?.brl_per_usd;
  const hasRate = typeof rate === "number" && rate > 0;

  return (
    <Box sx={{ display: "flex", gap: 2, mb: 3, flexWrap: "wrap" }}>
      <StatTile label="Total Wallet Value" value={`$${formatNumber(totalWalletValue)}`} accent="primary.main" />
      <StatTile
        label="Total in R$"
        value={hasRate ? `R$ ${formatNumber(totalWalletValue * rate)}` : "—"}
        accent="secondary.main"
        caption={
          hasRate
            ? `At R$ ${rate.toFixed(4)} per US$ (PTAX, ${usdBrl.quoted_at})`
            : "USD/BRL rate unavailable right now"
        }
      />
    </Box>
  );
}
