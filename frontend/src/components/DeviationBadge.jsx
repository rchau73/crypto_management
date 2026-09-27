import { Box } from "@mui/material";
import { STATUS_COLORS } from "../theme";

const SEVERITY_COLOR = {
  high: STATUS_COLORS.critical,
  medium: STATUS_COLORS.warning,
  normal: null,
};

// A deviation reading never relies on color alone (status colors are reserved
// and must ship with a visible cue, not just a tinted number) — a small dot
// sits beside the text so the signal survives for colorblind readers too.
export function DeviationBadge({ severity, children }) {
  const color = SEVERITY_COLOR[severity];
  return (
    <Box component="span" sx={{ display: "inline-flex", alignItems: "center", gap: 0.75, fontWeight: color ? 700 : 400 }}>
      {color && <Box component="span" sx={{ width: 6, height: 6, borderRadius: "50%", bgcolor: color, flexShrink: 0 }} />}
      <Box component="span" sx={{ color: color || "inherit" }}>
        {children}
      </Box>
    </Box>
  );
}
