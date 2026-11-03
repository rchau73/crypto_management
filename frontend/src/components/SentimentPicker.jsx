import { Box, Link, Paper, Stack, ToggleButton, ToggleButtonGroup, Typography } from "@mui/material";
import { RULE_TEXT, SENTIMENTS } from "../utils/rebalance";

const FEAR_GREED_URL = "https://alternative.me/crypto/fear-and-greed-index/";

// "Market sentiment today?" — BARCA rule 2. The app doesn't read the Fear &
// Greed index on purpose (BARCA.md: that judgement stays human), so the
// user picks it on every visit and every account switch; nothing is
// remembered, so one account's answer never carries over to another.
export function SentimentPicker({ value, onChange }) {
  return (
    <Paper variant="outlined" sx={{ p: 2, borderColor: value ? "divider" : "primary.main" }}>
      <Stack direction="row" useFlexGap spacing={2} sx={{ alignItems: "baseline", flexWrap: "wrap", mb: 1.5 }}>
        <Typography variant="h6" component="h2" id="sentiment-heading" sx={{ fontWeight: 700 }}>
          Market sentiment today
        </Typography>
        <Link href={FEAR_GREED_URL} target="_blank" rel="noopener noreferrer">
          Check the Fear &amp; Greed index
        </Link>
      </Stack>
      <ToggleButtonGroup
        exclusive
        value={value}
        onChange={(_, next) => next && onChange(next)}
        aria-labelledby="sentiment-heading"
        sx={{ flexWrap: "wrap" }}
      >
        {SENTIMENTS.map((s) => (
          <ToggleButton key={s.key} value={s.key} sx={{ minHeight: 44, px: 2 }}>
            {s.label}
          </ToggleButton>
        ))}
      </ToggleButtonGroup>
      <Box sx={{ mt: 1.5, color: "text.secondary" }}>
        <Typography variant="body2">
          {value ? RULE_TEXT[value] : "Pick today's reading to see this round's steps. It is asked again on every visit and account switch."}
        </Typography>
      </Box>
    </Paper>
  );
}
