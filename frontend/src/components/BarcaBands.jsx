import { Box, Paper, Stack, Typography } from "@mui/material";
import { formatNumber } from "../utils/formatters";
import { BAND_COLORS } from "../theme";

const STATUS_LABEL = { above: "Above band", below: "Below band", in: "In band" };

function signedPoints(n) {
  const sign = n > 0.005 ? "+" : n < -0.005 ? "−" : "";
  return `${sign}${formatNumber(Math.abs(n))} pts`;
}

function signedMoney(n) {
  const sign = n > 0.005 ? "+" : n < -0.005 ? "−" : "";
  return `${sign}$${formatNumber(Math.abs(n))}`;
}

// One bar per BARCA, each scaled to its own target (0 → 2× target): the
// tick is the target, the shaded zone its ±20% band, the dot where it is
// now. Relative on purpose, like the rule: a 10% bucket's band is as wide
// on screen as a 50% one's. Plain boxes, not a chart library: four bars.
function BandBar({ row }) {
  const hasTarget = row.targetPercent > 0;
  const position = hasTarget ? Math.min(100, Math.max(0, (row.currentPercent / (2 * row.targetPercent)) * 100)) : 100;
  return (
    <Box
      role="img"
      aria-label={`${row.name}: ${formatNumber(row.currentPercent)}% against a target of ${formatNumber(row.targetPercent)}%, ${STATUS_LABEL[row.status]}`}
      sx={{ position: "relative", height: 18, borderRadius: 1, bgcolor: "rgba(255,255,255,0.06)" }}
    >
      {hasTarget && (
        <>
          <Box sx={{ position: "absolute", top: 0, bottom: 0, left: "40%", width: "20%", bgcolor: "rgba(57,135,229,0.25)" }} />
          <Box sx={{ position: "absolute", top: -3, bottom: -3, left: "50%", width: 2, bgcolor: "text.secondary" }} />
        </>
      )}
      <Box
        sx={{
          position: "absolute",
          top: 2,
          left: `${position}%`,
          width: 14,
          height: 14,
          ml: "-7px",
          borderRadius: "50%",
          border: "2px solid",
          borderColor: "background.default",
          bgcolor: BAND_COLORS[row.status] ?? "text.primary",
          "@media (prefers-reduced-motion: no-preference)": { transition: "left .3s ease" },
        }}
      />
    </Box>
  );
}

export function BarcaBands({ rows }) {
  return (
    <Paper variant="outlined" sx={{ p: 2 }}>
      <Typography variant="h6" component="h2" sx={{ fontWeight: 700 }}>
        BARCA vs. band
      </Typography>
      <Typography variant="body2" sx={{ color: "text.secondary", mb: 2 }}>
        Each bar is scaled to its own target: the tick is the target, the shaded zone its ±20% band.
      </Typography>
      <Stack spacing={2.5}>
        {rows.map((row) => {
          const off = row.status !== "in";
          return (
            <Box key={row.id}>
              <Stack direction="row" useFlexGap sx={{ justifyContent: "space-between", flexWrap: "wrap", gap: 1, mb: 0.75 }}>
                <Typography sx={{ fontWeight: 700 }}>{row.name}</Typography>
                <Typography variant="body2" sx={{ color: "text.secondary", fontVariantNumeric: "tabular-nums" }}>
                  {formatNumber(row.currentPercent)}% now · target {formatNumber(row.targetPercent)}%
                  {row.targetPercent > 0 &&
                    ` · band ${formatNumber(row.targetPercent * 0.8)}–${formatNumber(row.targetPercent * 1.2)}%`}
                </Typography>
              </Stack>
              <BandBar row={row} />
              <Stack direction="row" useFlexGap sx={{ justifyContent: "space-between", flexWrap: "wrap", gap: 1, mt: 0.75 }}>
                <Typography variant="body2" sx={{ fontWeight: off ? 700 : 400, color: BAND_COLORS[row.status] ?? "text.secondary" }}>
                  {off ? "● " : ""}
                  {STATUS_LABEL[row.status]} ({signedPoints(row.deviation)})
                </Typography>
                <Typography variant="body2" sx={{ color: "text.secondary", fontVariantNumeric: "tabular-nums" }}>
                  {signedMoney(row.gap)} vs target
                </Typography>
              </Stack>
            </Box>
          );
        })}
      </Stack>
    </Paper>
  );
}
