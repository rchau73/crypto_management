import { Box, Button, FormControl, InputLabel, MenuItem, Select } from "@mui/material";

const LEVEL_OPTIONS = [
  { value: "totals", label: "Totals" },
  { value: "barca", label: "BARCA" },
  { value: "groups", label: "Groups" },
  { value: "assets", label: "Assets" },
];

const GRANULARITY_OPTIONS = [
  { value: "5min", label: "5 min" },
  { value: "30min", label: "30 min" },
  { value: "1h", label: "1 hour" },
  { value: "4h", label: "4 hours" },
  { value: "daily", label: "Daily" },
  { value: "weekly", label: "Weekly" },
  { value: "monthly", label: "Monthly" },
  { value: "quarterly", label: "Quarterly" },
  { value: "yearly", label: "Yearly" },
];

// Level/series/granularity pickers plus a manual refresh button for the
// historical dashboard chart.
export function DashboardControls({
  level,
  onLevelChange,
  granularity,
  onGranularityChange,
  series,
  selectedSeries,
  onSelectedSeriesChange,
  onRefresh,
}) {
  const showSeriesPicker = level === "assets" || level === "barca";

  return (
    <Box sx={{ display: "flex", gap: 2, mb: 2, alignItems: "center", flexWrap: "wrap" }}>
      <FormControl size="small" sx={{ minWidth: 140 }}>
        <InputLabel>Level</InputLabel>
        <Select label="Level" value={level} onChange={(e) => onLevelChange(e.target.value)}>
          {LEVEL_OPTIONS.map((opt) => (
            <MenuItem key={opt.value} value={opt.value}>
              {opt.label}
            </MenuItem>
          ))}
        </Select>
      </FormControl>

      {showSeriesPicker && (
        <FormControl size="small" sx={{ minWidth: 220 }}>
          <InputLabel>Series</InputLabel>
          <Select
            multiple
            label="Series"
            value={selectedSeries}
            onChange={(e) => onSelectedSeriesChange(typeof e.target.value === "string" ? e.target.value.split(",") : e.target.value)}
            renderValue={(selected) => selected.join(", ")}
          >
            {series.map((s) => (
              <MenuItem key={s} value={s}>
                {s}
              </MenuItem>
            ))}
          </Select>
        </FormControl>
      )}

      <FormControl size="small" sx={{ minWidth: 140 }}>
        <InputLabel>Granularity</InputLabel>
        <Select label="Granularity" value={granularity} onChange={(e) => onGranularityChange(e.target.value)}>
          {GRANULARITY_OPTIONS.map((opt) => (
            <MenuItem key={opt.value} value={opt.value}>
              {opt.label}
            </MenuItem>
          ))}
        </Select>
      </FormControl>

      <Button variant="outlined" size="small" onClick={onRefresh}>
        Refresh
      </Button>
    </Box>
  );
}
