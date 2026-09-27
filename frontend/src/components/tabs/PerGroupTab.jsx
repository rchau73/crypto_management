import { Box, Paper, Table, TableBody, TableCell, TableContainer, TableHead, TableRow, Typography } from "@mui/material";
import { AllocationPieChart } from "../AllocationPieChart";
import { DeviationBadge } from "../DeviationBadge";
import { getTotalTargetPercent } from "../../utils/allocationMath";
import { formatNumber } from "../../utils/formatters";
import { numericCellSx, zebraRowSx } from "../../styles/tableStyles";
import { colorForSeries } from "../../theme";

// Tab 2: per-group table (values already computed server-side, no client
// recalculation needed) plus the group target-allocation pie chart.
export function PerGroupTab({ groupAllocations }) {
  if (groupAllocations.length === 0) return null;

  const allGroupNames = groupAllocations.map((g) => g.group);
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
              const deviationValue = g.value * (g.deviation / 100);
              const severity = Math.abs(g.deviation) > 1 ? "high" : "normal";
              return (
                <TableRow key={idx} hover sx={zebraRowSx}>
                  <TableCell sx={{ fontSize: 10 }}>{g.group}</TableCell>
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
                    ${formatNumber(deviationValue)}
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
