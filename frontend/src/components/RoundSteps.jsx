import { Box, Checkbox, FormControlLabel, Paper, Stack, Typography } from "@mui/material";
import { formatNumber } from "../utils/formatters";
import { CASH_BARCA } from "../utils/rebalance";

// "This round's steps": what to buy or sell now, one checkbox each. Ticking
// only tracks progress while you place the orders elsewhere — the app
// never trades. `done` is a set of step ids kept for this visit only.
export function RoundSteps({ plan, sentiment, done, onToggle }) {
  return (
    <Paper variant="outlined" sx={{ p: 2 }}>
      <Typography variant="h6" component="h2" sx={{ fontWeight: 700 }}>
        This round's steps
      </Typography>
      <Typography variant="body2" sx={{ color: "text.secondary", mb: 2 }}>
        Each step is 30% of the $ gap (DCA). Ticking a step only tracks your progress; the app never places orders.
      </Typography>

      {!sentiment ? (
        <Typography sx={{ color: "text.secondary" }}>Pick today's market sentiment above to see the steps.</Typography>
      ) : plan.steps.length === 0 ? (
        <Typography sx={{ color: "text.secondary" }}>
          No step this round.{" "}
          {plan.waiting.length > 0
            ? "The buckets outside their band are waiting for the right sentiment (below)."
            : "Every BARCA is inside its band; check again after the next big move."}
        </Typography>
      ) : (
        <Stack spacing={1}>
          {plan.steps.map((step) => {
            const isDone = done.has(step.id);
            return (
              <Box
                key={step.id}
                sx={{ border: 1, borderColor: "divider", borderRadius: 2, px: 1.5, py: 1, opacity: isDone ? 0.55 : 1 }}
              >
                <FormControlLabel
                  control={<Checkbox checked={isDone} onChange={() => onToggle(step.id)} />}
                  sx={{ alignItems: "flex-start", m: 0, width: "100%" }}
                  label={
                    <Box sx={{ pt: 1, textDecoration: isDone ? "line-through" : "none" }}>
                      <Typography sx={{ fontWeight: 700, fontSize: 17, fontVariantNumeric: "tabular-nums" }}>
                        <Box component="span" sx={{ color: step.verb === "Buy" ? "primary.light" : "warning.light" }}>
                          {step.verb}
                        </Box>{" "}
                        ${formatNumber(step.amount)} of {step.symbol}
                      </Typography>
                      <Typography variant="body2" sx={{ color: "text.secondary" }}>
                        {step.funding} · {step.barca} is ${formatNumber(Math.abs(step.barcaGap))}{" "}
                        {step.barcaGap > 0 ? "above" : "below"} target; this round moves 30% of it.
                      </Typography>
                    </Box>
                  }
                />
              </Box>
            );
          })}
        </Stack>
      )}

      {sentiment && plan.waiting.length > 0 && (
        <Box sx={{ mt: 2, pt: 1.5, borderTop: 1, borderColor: "divider" }}>
          <Typography variant="body2" sx={{ fontWeight: 700, color: "text.secondary", mb: 0.5 }}>
            Out of band, but waiting
          </Typography>
          {plan.waiting.map((w) => (
            <Typography key={w.barca} variant="body2" sx={{ color: "text.secondary" }}>
              <Box component="span" sx={{ fontWeight: 700, color: "text.primary" }}>
                {w.barca}
              </Box>{" "}
              — {w.reason}
            </Typography>
          ))}
        </Box>
      )}
      {plan.notes.length > 0 && (
        <Typography variant="body2" sx={{ color: "text.secondary", mt: 1.5 }} data-testid="cash-note">
          {plan.notes.join(" ")} {CASH_BARCA} itself is never a step: it funds the buys and receives the sells.
        </Typography>
      )}
    </Paper>
  );
}
