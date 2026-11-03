import { Box, Paper, Table, TableBody, TableCell, TableContainer, TableHead, TableRow, Typography } from "@mui/material";
import { AllocationPieChart } from "../AllocationPieChart";
import { DeviationBadge } from "../DeviationBadge";
import { bandSeverity, getTotalTargetPercent, valueGapFromDeviation } from "../../utils/allocationMath";
import { formatNumber } from "../../utils/formatters";
import { compactCellSx, numericCellSx, zebraRowSx } from "../../styles/tableStyles";
import { colorForSeries } from "../../theme";

// Tab 2: per-group table (values already computed server-side, no client
// recalculation needed) plus the group target-allocation pie chart.
export function PerGroupTab({ groupAllocations }) {
  if (groupAllocations.length === 0) return null;

  const allGroupNames = groupAllocations.map((g) => g.group);
  // Group values cover every priced asset, so together they are the wallet total.
  const totalValue = groupAllocations.reduce((sum, g) => sum + (g.value || 0), 0);
  const pieData = groupAllocations.filter((g) => g.value > 0).map((g) => ({ name: g.group, value: g.value }));

  return (
    <Box>
      <Typography variant="h6" sx={{ mt: 2, mb: 2 }}>
        Per-Group Allocation
        <Box component="span" sx={{ color: "primary.main", fontWeight: "normal", ml: 2, fontSize: 14 }}>
          Total Target %: {formatNumber(getTotalTargetPercent(groupAllocations))}%
        </Box>
      </Typography>
      <TableContainer component={Paper} sx={{ maxWidth: "100%", overflowX: "auto" }}>
        <Table sx={{ minWidth: 600 }}>
          <TableHead>
            <TableRow>
              <TableCell>Group</TableCell>
              <TableCell align="right">Current Value&nbsp;($)</TableCell>
              <TableCell align="right">Current %</TableCell>
              <TableCell align="right">Deviation</TableCell>
              <TableCell align="right">Value Deviation</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {groupAllocations.map((g, idx) => {
              const deviationValue = valueGapFromDeviation(g.deviation, totalValue);
              const severity = bandSeverity(g.current_percent, g.target_percent);
              return (
                <TableRow key={idx} hover sx={zebraRowSx}>
                  <TableCell sx={compactCellSx}>{g.group}</TableCell>
                  <TableCell align="right" sx={numericCellSx}>
                    ${formatNumber(g.value)}
                  </TableCell>
                  <TableCell align="right" sx={numericCellSx}>
                    {formatNumber(g.current_percent)}%
                  </TableCell>
                  <TableCell align="right" sx={numericCellSx}>
                    <DeviationBadge severity={severity}>
                      {g.deviation > 0 ? "+" : ""}
                      {formatNumber(g.deviation)}%
                    </DeviationBadge>
                  </TableCell>
                  <TableCell align="right" sx={numericCellSx}>
                    {deviationValue > 0 ? "+$" : deviationValue < 0 ? "-$" : "$"}
                    {formatNumber(Math.abs(deviationValue))}
                  </TableCell>
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
      </TableContainer>
      {pieData.length > 0 && (
        <Box sx={{ mt: 4 }}>
          <Typography variant="h6" sx={{ mb: 2 }}>
            Group Target Allocation (Pie Chart)
          </Typography>
          <AllocationPieChart data={pieData} colorForEntry={(entry) => colorForSeries(entry.name, allGroupNames)} outerRadius={80} />
        </Box>
      )}
    </Box>
  );
}
