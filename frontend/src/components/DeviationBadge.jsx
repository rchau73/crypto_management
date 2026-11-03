import { Box } from "@mui/material";
import { BAND_COLORS } from "../theme";

// A band status ("above" / "below" / "in") as colored text with a dot. It
// never relies on color alone (the words say it, and the dot is a second
// cue), so the signal survives for colorblind readers too.
export function DeviationBadge({ status, children }) {
  const color = BAND_COLORS[status] ?? null;
  return (
    <Box component="span" sx={{ display: "inline-flex", alignItems: "center", gap: 0.75, fontWeight: color ? 700 : 400 }}>
      {color && <Box component="span" sx={{ width: 6, height: 6, borderRadius: "50%", bgcolor: color, flexShrink: 0 }} />}
      <Box component="span" sx={{ color: color || "inherit" }}>
        {children}
      </Box>
    </Box>
  );
}
