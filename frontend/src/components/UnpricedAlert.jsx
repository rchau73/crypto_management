import { Alert, AlertTitle, Box, Button } from "@mui/material";
import { formatNumber } from "../utils/formatters";
import { unpricedFixes, unpricedTitle } from "../utils/unpriced";

// Warns that some holdings have no price and are left out of every total.
// Full (Holdings): lists each one and how to fix it. Compact (Rebalance):
// one line and a link to Holdings, so the round stays the focus.
export function UnpricedAlert({ unpriced, compact = false, onOpenHoldings }) {
  if (!unpriced?.length) return null;
  const title = unpricedTitle(unpriced.length);

  if (compact) {
    return (
      <Alert
        severity="warning"
        sx={{ mb: 2 }}
        action={
          onOpenHoldings && (
            <Button color="inherit" size="small" onClick={onOpenHoldings}>
              See which in Holdings
            </Button>
          )
        }
      >
        {title}: the bands below don't count them.
      </Alert>
    );
  }

  return (
    <Alert severity="warning" sx={{ mb: 1.5 }}>
      <AlertTitle>{title}</AlertTitle>
      <Box component="ul" sx={{ m: 0, pl: 2.5 }}>
        {unpriced.map((u) => (
          <li key={`${u.symbol}|${u.group}|${u.barca}|${u.asset_class}`}>
            <strong>{u.symbol}</strong> · {u.barca || "no BARCA"} · {u.asset_class} · qty{" "}
            <Box component="span" sx={{ fontVariantNumeric: "tabular-nums" }}>
              {formatNumber(u.current_quantity)}
            </Box>
          </li>
        ))}
      </Box>
      {unpricedFixes(unpriced).map((fix) => (
        <Box key={fix} sx={{ mt: 1 }}>
          {fix}
        </Box>
      ))}
    </Alert>
  );
}
