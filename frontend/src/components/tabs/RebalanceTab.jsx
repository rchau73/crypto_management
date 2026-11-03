import { useMemo, useState } from "react";
import { Alert, Box, Button, Stack, Typography } from "@mui/material";
import { SentimentPicker } from "../SentimentPicker";
import { BarcaBands } from "../BarcaBands";
import { RoundSteps } from "../RoundSteps";
import { RecordTradeDialog } from "../RecordTradeDialog";
import { MoveBalanceDialog } from "../MoveBalanceDialog";
import { planRound } from "../../utils/rebalance";
import { describePricesAge } from "../../utils/pricesAge";

// Rebalance: the home page, built for coming back after a big BTC move.
// Where each BARCA stands against its band, the market sentiment (asked on
// every visit), and the round's buy/sell steps with the reason for any
// bucket that waits. `sentiment` and `done` live in App, so they last for
// this visit and account only. With `canRecord`, a trade made on an
// exchange is recorded here (RecordTradeDialog) and a balance moved to
// another BARCA or exchange (MoveBalanceDialog), instead of through a CSV.
export function RebalanceTab({
  report,
  sentiment,
  onSentimentChange,
  done,
  onToggleDone,
  pricesAsOf,
  refreshIntervalSecs,
  onOpenHoldings,
  accountName,
  canRecord,
  onTradeRecorded,
}) {
  const plan = useMemo(() => planRound(report, sentiment), [report, sentiment]);
  const { label: pricesLabel, stale } = describePricesAge(pricesAsOf, refreshIntervalSecs);
  // undefined = closed; null = open for any position; a step = pre-filled.
  const [recording, setRecording] = useState(undefined);
  const [moving, setMoving] = useState(false);
  const [notice, setNotice] = useState("");

  const recorded = ({ stepId, message }) => {
    setRecording(undefined);
    setNotice(message);
    if (stepId && !done.has(stepId)) onToggleDone(stepId);
    onTradeRecorded();
  };
  const moved = ({ message }) => {
    setMoving(false);
    setNotice(message);
    onTradeRecorded();
  };

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

      {notice && (
        <Alert severity="success" sx={{ mb: 2 }} onClose={() => setNotice("")}>
          {notice}
        </Alert>
      )}

      <Stack spacing={2}>
        <SentimentPicker value={sentiment} onChange={onSentimentChange} />
        <Box sx={{ display: "grid", gap: 2, gridTemplateColumns: { xs: "1fr", lg: "1fr 1fr" }, alignItems: "start" }}>
          <BarcaBands rows={plan.barcas} />
          <RoundSteps
            plan={plan}
            sentiment={sentiment}
            done={done}
            onToggle={onToggleDone}
            onRecord={canRecord ? (step) => setRecording(step) : undefined}
            onMove={canRecord ? () => setMoving(true) : undefined}
          />
        </Box>
      </Stack>

      {recording !== undefined && (
        <RecordTradeDialog
          preset={
            recording && {
              side: recording.verb === "Buy" ? "buy" : "sell",
              symbol: recording.symbol,
              barca: recording.barca,
              amount: recording.amount,
              stepId: recording.id,
            }
          }
          allocations={report.allocations}
          accountName={accountName}
          onClose={() => setRecording(undefined)}
          onRecorded={recorded}
        />
      )}
      {moving && <MoveBalanceDialog report={report} accountName={accountName} onClose={() => setMoving(false)} onMoved={moved} />}
    </Box>
  );
}
