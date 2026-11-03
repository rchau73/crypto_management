import { useMemo } from "react";
import { Alert, Box, Button, Stack, Typography } from "@mui/material";
import { SentimentPicker } from "../SentimentPicker";
import { BarcaBands } from "../BarcaBands";
import { RoundSteps } from "../RoundSteps";
import { planRound } from "../../utils/rebalance";
import { describePricesAge } from "../../utils/pricesAge";

// Rebalance: the home page, built for coming back after a big BTC move.
// Where each BARCA stands against its band, the market sentiment (asked on
// every visit), and the round's buy/sell steps with the reason for any
// bucket that waits. `sentiment` and `done` live in App, so they last for
// this visit and account only.
export function RebalanceTab({ report, sentiment, onSentimentChange, done, onToggleDone, pricesAsOf, refreshIntervalSecs, onOpenHoldings }) {
  const plan = useMemo(() => planRound(report, sentiment), [report, sentiment]);
  const { label: pricesLabel, stale } = describePricesAge(pricesAsOf, refreshIntervalSecs);

  return (
    <Box>
      <Stack direction="row" useFlexGap spacing={2} sx={{ mb: 2, alignItems: "baseline", flexWrap: "wrap" }}>
        <Typography variant="h5" component="h1" sx={{ fontWeight: 700 }}>
          Rebalance
        </Typography>
        <Typography sx={{ color: "text.secondary" }}>
          {plan.outOfBandCount} of {plan.barcas.length} BARCA outside their band
        </Typography>
        <Button size="small" sx={{ ml: "auto" }} onClick={onOpenHoldings}>
          See every asset in Holdings
        </Button>
      </Stack>

      {stale && (
        <Alert severity="warning" sx={{ mb: 2 }}>
          {pricesLabel}: the steps below use those prices. Check them before placing any order.
        </Alert>
      )}

      <Stack spacing={2}>
        <SentimentPicker value={sentiment} onChange={onSentimentChange} />
        <Box sx={{ display: "grid", gap: 2, gridTemplateColumns: { xs: "1fr", lg: "1fr 1fr" }, alignItems: "start" }}>
          <BarcaBands rows={plan.barcas} />
          <RoundSteps plan={plan} sentiment={sentiment} done={done} onToggle={onToggleDone} />
        </Box>
      </Stack>
    </Box>
  );
}
