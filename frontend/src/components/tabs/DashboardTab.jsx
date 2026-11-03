import { Box, CircularProgress, Typography } from "@mui/material";
import { DashboardControls } from "../DashboardControls";
import { HistoryLineChart } from "../HistoryLineChart";
import { useHistoryDashboard } from "../../hooks/useHistoryDashboard";

// History: the account's value over time. Fetches when the page mounts.
export function DashboardTab() {
  const dashboard = useHistoryDashboard();

  return (
    <Box>
      <Typography variant="h5" component="h1" sx={{ fontWeight: 700, mb: 2 }}>
        History
      </Typography>
      <DashboardControls
        level={dashboard.level}
        onLevelChange={dashboard.setLevel}
        granularity={dashboard.granularity}
        onGranularityChange={dashboard.setGranularity}
        series={dashboard.series}
        selectedSeries={dashboard.selectedSeries}
        onSelectedSeriesChange={dashboard.setSelectedSeries}
        onRefresh={dashboard.refresh}
      />
      <Box sx={{ height: 400 }}>
        {dashboard.data.length === 0 ? (
          <Box>
            {dashboard.loading ? (
              <CircularProgress size={20} sx={{ mt: 1 }} />
            ) : (
              !dashboard.error && (
                <Typography variant="body2" sx={{ color: "text.secondary" }}>
                  No history for this account yet. A point is recorded each time prices refresh while the app is open; the chart appears after the first one.
                </Typography>
              )
            )}
            {dashboard.error && (
              <Typography variant="body2" sx={{ color: "error.main", mt: 1 }}>
                {dashboard.error}
              </Typography>
            )}
          </Box>
        ) : (
          <HistoryLineChart
            data={dashboard.data}
            level={dashboard.level}
            selectedSeries={dashboard.selectedSeries}
            allSeries={dashboard.series}
            yAxisDomain={dashboard.yAxisDomain}
          />
        )}
      </Box>
    </Box>
  );
}
