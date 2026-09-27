import { Cell, Legend, Pie, PieChart, ResponsiveContainer, Tooltip } from "recharts";
import { ChartTooltip } from "./ChartTooltip";

function renderSliceLabel({ percent, name, x, y }) {
  return (
    <text x={x} y={y} textAnchor="middle" dominantBaseline="central" fontSize={12} fill="#fff">
      {`${name}: ${(percent * 100).toFixed(1)}%`}
    </text>
  );
}

// Shared pie chart for group/BARCA target and actual allocation breakdowns.
// `colorForEntry` lets callers key color by index or by a stable name->color
// map (BARCA needs the latter so "target" and "actual" charts agree on color).
export function AllocationPieChart({ data, colorForEntry, outerRadius = 80, width = 350, height = 300 }) {
  return (
    <ResponsiveContainer width={width} height={height}>
      <PieChart>
        <Pie data={data} dataKey="value" nameKey="name" cx="50%" cy="50%" outerRadius={outerRadius} label={renderSliceLabel}>
          {data.map((entry, index) => (
            <Cell key={`cell-${entry.name}-${index}`} fill={colorForEntry(entry, index)} />
          ))}
        </Pie>
        <Tooltip content={<ChartTooltip />} />
        <Legend />
      </PieChart>
    </ResponsiveContainer>
  );
}
