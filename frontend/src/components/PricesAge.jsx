import { useEffect, useState } from "react";
import { Box, Typography } from "@mui/material";
import { describePricesAge } from "../utils/pricesAge";
import { STATUS_COLORS } from "../theme";

const TICK_MS = 10_000;

// "Prices from 4 min ago" in the header, replacing the old Update Prices
// button: prices now refresh by themselves, so what the user needs is how
// fresh the numbers are. Turns amber (with a dot and a tooltip, never
// color alone) when refreshes seem to be failing.
export function PricesAge({ pricesAsOf, refreshIntervalSecs }) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), TICK_MS);
    return () => clearInterval(id);
  }, []);

  const { label, stale } = describePricesAge(pricesAsOf, refreshIntervalSecs, now);
  const minutes = Math.round((refreshIntervalSecs || 0) / 60);
  const title = stale
    ? "These prices are older than expected; the price service may be failing. Check before acting on them."
    : minutes > 0
      ? `Prices refresh every ${minutes} min while the app is open.`
      : undefined;

  return (
    <Box
      role="status"
      title={title}
      sx={{ display: "inline-flex", alignItems: "center", gap: 0.75, color: stale ? STATUS_COLORS.warning : "text.secondary" }}
    >
      {stale && <Box component="span" sx={{ width: 8, height: 8, borderRadius: "50%", bgcolor: STATUS_COLORS.warning }} />}
      <Typography variant="body2" sx={{ fontWeight: stale ? 700 : 400, color: "inherit" }}>
        {label}
        {stale && " — may be outdated"}
      </Typography>
    </Box>
  );
}
