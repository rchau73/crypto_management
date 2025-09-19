import { useMemo } from "react";
import {
  Box,
  Paper,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Typography,
} from "@mui/material";
import { Legend, Pie, PieChart, ResponsiveContainer, Tooltip, Cell } from "recharts";

import { formatNumber, sumTargetPercent } from "../utils/format";
import { CHART_COLORS } from "../utils/colors";

export function BarcaSection({ targets, actual }) {
  const targetPieData = useMemo(
    () =>
      targets
        .filter((entry) => entry.target_percent > 0)
        .map((entry) => ({ name: entry.barca, value: entry.target_percent })),
    [targets]
  );

  const actualPieData = useMemo(
    () =>
      actual
        .filter((entry) => entry.value > 0)
        .map((entry) => ({ name: entry.barca, value: entry.value })),
    [actual]
  );

  const barcaNameToColor = useMemo(() => {
    const uniqueNames = new Set([
      ...targets.map((entry) => entry.barca),
      ...actual.map((entry) => entry.barca),
    ]);
    return Array.from(uniqueNames).reduce((acc, name, index) => {
      acc[name] = CHART_COLORS[index % CHART_COLORS.length];
      return acc;
    }, {});
  }, [targets, actual]);

  return (
    <Box>
      <Typography variant="h6" sx={{ mt: 2, mb: 2 }}>
        Per-BARCA Actual Allocation
        <span
          style={{
            color: "#00C49F",
            fontWeight: "normal",
            marginLeft: 16,
            fontSize: 14,
          }}
        >
          Total Target %: {formatNumber(sumTargetPercent(targets))}%
        </span>
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
            {actual.map((entry) => (
              <TableRow key={entry.barca}>
                <TableCell sx={{ fontSize: 10 }}>{entry.barca}</TableCell>
                <TableCell align="right" sx={{ fontSize: 10 }}>
                  ${formatNumber(entry.value)}
                </TableCell>
                <TableCell align="right" sx={{ fontSize: 10 }}>
                  {formatNumber(entry.current_percent)}%
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </TableContainer>

      {(targetPieData.length > 0 || actualPieData.length > 0) && (
        <Box sx={{ display: "flex", flexWrap: "wrap", gap: 4, justifyContent: "center", mt: 4 }}>
          {targetPieData.length > 0 && (
            <PiePanel title="BARCA Target Allocation">
              <Pie
                data={targetPieData}
                dataKey="value"
                nameKey="name"
                cx="50%"
                cy="50%"
                outerRadius={100}
                label={({ percent, name, x, y }) => (
                  <text
                    x={x}
                    y={y}
                    textAnchor="middle"
                    dominantBaseline="central"
                    fontSize={12}
                    fill="#fff"
                  >
                    {`${name}: ${(percent * 100).toFixed(1)}%`}
                  </text>
                )}
              >
                {targetPieData.map((entry) => (
                  <Cell key={`target-${entry.name}`} fill={barcaNameToColor[entry.name]} />
                ))}
              </Pie>
              <Tooltip />
              <Legend />
            </PiePanel>
          )}

          {actualPieData.length > 0 && (
            <PiePanel title="BARCA Actual Allocation">
              <Pie
                data={actualPieData}
                dataKey="value"
                nameKey="name"
                cx="50%"
                cy="50%"
                outerRadius={100}
                label={({ percent, name, x, y }) => (
                  <text
                    x={x}
                    y={y}
                    textAnchor="middle"
                    dominantBaseline="central"
                    fontSize={12}
                    fill="#fff"
                  >
                    {`${name}: ${(percent * 100).toFixed(1)}%`}
                  </text>
                )}
              >
                {actualPieData.map((entry) => (
                  <Cell key={`actual-${entry.name}`} fill={barcaNameToColor[entry.name]} />
                ))}
              </Pie>
              <Tooltip />
              <Legend />
            </PiePanel>
          )}
        </Box>
      )}
    </Box>
  );
}

function PiePanel({ title, children }) {
  return (
    <Box sx={{ flex: 1, maxWidth: 400 }}>
      <Typography variant="h6" sx={{ mb: 2 }}>
        {title}
      </Typography>
      <ResponsiveContainer width={350} height={300}>
        <PieChart>{children}</PieChart>
      </ResponsiveContainer>
    </Box>
  );
}
