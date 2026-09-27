import { Box, Paper, Table, TableBody, TableCell, TableContainer, TableHead, TableRow, Typography } from "@mui/material";
import { AllocationPieChart } from "../AllocationPieChart";
import { getTotalTargetPercent } from "../../utils/allocationMath";
import { formatNumber } from "../../utils/formatters";
import { numericCellSx, zebraRowSx } from "../../styles/tableStyles";
import { colorForSeries } from "../../theme";

// Tab 3: actual BARCA allocation table, plus target vs. actual pie charts
// side by side. Both charts key color off the same BARCA name (via
// colorForSeries) so "Base" (say) is always the same color in both.
export function BarcaActualTab({ barcaAllocations, barcaActualAllocations }) {
  if (barcaActualAllocations.length === 0) return null;

  const allBarcaNames = Array.from(new Set([...barcaAllocations.map((b) => b.barca), ...barcaActualAllocations.map((b) => b.barca)]));
  const colorForName = (name) => colorForSeries(name, allBarcaNames);

  const targetPieData = barcaAllocations.filter((b) => b.target_percent > 0).map((b) => ({ name: b.barca, value: b.target_percent }));
  const actualPieData = barcaActualAllocations.filter((b) => b.value > 0).map((b) => ({ name: b.barca, value: b.value }));

  return (
    <Box>
      <Typography variant="h6" sx={{ mt: 2, mb: 2 }}>
        Per-BARCA Actual Allocation
        <Box component="span" sx={{ color: "primary.main", fontWeight: "normal", ml: 2, fontSize: 14 }}>
          Total Target %: {formatNumber(getTotalTargetPercent(barcaAllocations))}%
        </Box>
      </Typography>
      <TableContainer component={Paper} sx={{ maxWidth: "100%", overflowX: "auto" }}>
        <Table sx={{ minWidth: 600 }}>
          <TableHead>
            <TableRow>
              <TableCell>BARCA</TableCell>
              <TableCell align="right">Value</TableCell>
              <TableCell align="right">Current %</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {barcaActualAllocations.map((b, idx) => (
              <TableRow key={idx} hover sx={zebraRowSx}>
                <TableCell sx={{ fontSize: 10 }}>{b.barca}</TableCell>
                <TableCell align="right" sx={numericCellSx}>
                  ${formatNumber(b.value)}
                </TableCell>
                <TableCell align="right" sx={numericCellSx}>
                  {formatNumber(b.current_percent)}%
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </TableContainer>
      {(targetPieData.length > 0 || actualPieData.length > 0) && (
        <Box sx={{ display: "flex", gap: 4, justifyContent: "center", mt: 4 }}>
          {targetPieData.length > 0 && (
            <Box sx={{ flex: 1, maxWidth: 400 }}>
              <Typography variant="h6" sx={{ mb: 2 }}>
                BARCA Target Allocation
              </Typography>
              <AllocationPieChart
                data={targetPieData}
                colorForEntry={(entry) => colorForName(entry.name)}
                outerRadius={100}
              />
            </Box>
          )}
          {actualPieData.length > 0 && (
            <Box sx={{ flex: 1, maxWidth: 400 }}>
              <Typography variant="h6" sx={{ mb: 2 }}>
                BARCA Actual Allocation
              </Typography>
              <AllocationPieChart
                data={actualPieData}
                colorForEntry={(entry) => colorForName(entry.name)}
                outerRadius={100}
              />
            </Box>
          )}
        </Box>
      )}
    </Box>
  );
}
