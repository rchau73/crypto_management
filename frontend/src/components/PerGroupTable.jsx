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

export function PerGroupTable({ rows }) {
  const pieData = useMemo(
    () =>
      rows
        .filter((group) => group.value > 0)
        .map((group) => ({ name: group.group, value: group.value })),
    [rows]
  );

  return (
    <Box>
      <Typography variant="h6" sx={{ mt: 2, mb: 2 }}>
        Per-Group Allocation
        <span
          style={{
            color: "#00C49F",
            fontWeight: "normal",
            marginLeft: 16,
            fontSize: 14,
          }}
        >
          Total Target %: {formatNumber(sumTargetPercent(rows))}%
        </span>
      </Typography>

      <TableContainer component={Paper} sx={{ maxWidth: "100%", overflowX: "auto" }}>
        <Table sx={{ minWidth: 900 }}>
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
            {rows.map((group) => {
              const deviationValue = group.value * (group.deviation / 100);
              return (
                <TableRow key={group.group}>
                  <TableCell sx={{ fontSize: 10 }}>{group.group}</TableCell>
                  <TableCell align="right" sx={{ fontSize: 10 }}>
                    ${formatNumber(group.value)}
                  </TableCell>
                  <TableCell align="right" sx={{ fontSize: 10 }}>
                    {formatNumber(group.current_percent)}%
                  </TableCell>
                  <TableCell
                    align="right"
                    sx={{
                      color: Math.abs(group.deviation) > 1 ? "error.main" : "inherit",
                      fontWeight: Math.abs(group.deviation) > 1 ? "bold" : "normal",
                      fontSize: 10,
                    }}
                  >
                    {group.deviation > 0 ? "+" : ""}
                    {formatNumber(group.deviation)}%
                  </TableCell>
                  <TableCell align="right" sx={{ fontSize: 10 }}>
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
          <ResponsiveContainer width={350} height={300}>
            <PieChart>
              <Pie
                data={pieData}
                dataKey="value"
                nameKey="name"
                cx="50%"
                cy="50%"
                outerRadius={80}
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
                {pieData.map((entry, index) => (
                  <Cell key={entry.name} fill={CHART_COLORS[index % CHART_COLORS.length]} />
                ))}
              </Pie>
              <Tooltip />
              <Legend />
            </PieChart>
          </ResponsiveContainer>
        </Box>
      )}
    </Box>
  );
}
