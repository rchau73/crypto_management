import { createTheme } from "@mui/material/styles";

// Dark-surface tokens (validated categorical/status palette — see the
// dataviz design method: fixed lightness band, CVD-safe adjacent pairs,
// status colors reserved and never reused as chart series colors).
const CHART_SURFACE = "#1a1a19";
const PAGE_PLANE = "#0d0d0d";
const TEXT_PRIMARY = "#ffffff";
const TEXT_SECONDARY = "#c3c2b7";
const MUTED = "#898781";
const GRID = "#2c2c2a";
const AXIS = "#383835";

export const darkTheme = createTheme({
  palette: {
    mode: "dark",
    background: { default: PAGE_PLANE, paper: CHART_SURFACE },
    primary: { main: "#3987e5" },
    secondary: { main: "#9085e9" },
    success: { main: "#0ca30c" },
    warning: { main: "#fab219" },
    error: { main: "#d03b3b" },
    text: { primary: TEXT_PRIMARY, secondary: TEXT_SECONDARY },
    divider: GRID,
  },
  shape: { borderRadius: 10 },
  typography: {
    fontFamily: 'system-ui, -apple-system, "Segoe UI", sans-serif',
  },
  components: {
    MuiButton: { styleOverrides: { root: { textTransform: "none", fontWeight: 600 } } },
    MuiPaper: { styleOverrides: { root: { backgroundImage: "none" } } },
    MuiAppBar: { styleOverrides: { root: { backgroundImage: "none" } } },
  },
});

// Fixed-order categorical hues. A chart never cycles an arbitrary index into
// this array to assign color — use colorForSeries() below so the same entity
// (e.g. "BTC", "Base") always gets the same slot regardless of which subset
// of series happens to be selected.
export const COLORS = [
  "#3987e5", // blue
  "#d95926", // orange
  "#199e70", // aqua
  "#c98500", // yellow
  "#d55181", // magenta
  "#008300", // green
  "#9085e9", // violet
  "#e66767", // red
];

// Single-hue line for a lone series (e.g. the "totals" dashboard level).
export const SEQUENTIAL_BLUE = COLORS[0];

// Reserved for deviation severity only — never reused as a chart series color.
export const STATUS_COLORS = {
  good: "#0ca30c",
  warning: "#fab219",
  critical: "#d03b3b",
};

export const CHART_GRID = GRID;
export const CHART_AXIS = AXIS;
export const CHART_MUTED_TEXT = MUTED;

// Stable color assignment: an entity's color depends on its position in the
// *full* set of known series (sorted), not on the current selection/filter,
// so removing or reordering a filter never repaints the survivors.
export function colorForSeries(name, allSeries) {
  const sorted = [...allSeries].sort();
  const idx = sorted.indexOf(name);
  return COLORS[(idx < 0 ? 0 : idx) % COLORS.length];
}
