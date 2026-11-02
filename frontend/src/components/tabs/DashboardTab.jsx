import { Box, CircularProgress, Typography } from "@mui/material";
import { DashboardControls } from "../DashboardControls";
import { HistoryLineChart } from "../HistoryLineChart";
import { useHistoryDashboard } from "../../hooks/useHistoryDashboard";

// Historical dashboard. Fetches when the tab mounts.
export function DashboardTab() {
  const dashboard = useHistoryDashboard();

  return (
    <Box>
      <Typography variant="h6" sx={{ mt: 2, mb: 2 }}>
        Historical Dashboard
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
