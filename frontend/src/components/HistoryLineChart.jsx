import { CartesianGrid, Legend, Line, LineChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from "recharts";
import { ChartTooltip } from "./ChartTooltip";
import { formatUsdAxisTick } from "../utils/formatters";
import { CHART_AXIS, CHART_GRID, CHART_MUTED_TEXT, SEQUENTIAL_BLUE, colorForSeries } from "../theme";

// The dashboard's historical value chart. Renders a single "value" line for
// the totals level, or one line per selected series (asset/BARCA/group)
// otherwise. `allSeries` (every series available, not just the selected
// ones) is what makes color stable — an entity keeps its color even as the
// selection is narrowed or widened.
export function HistoryLineChart({ data, level, selectedSeries, allSeries, yAxisDomain }) {
  return (
    <ResponsiveContainer width="100%" height={400}>
      <LineChart data={data} margin={{ top: 5, right: 20, left: 10, bottom: 5 }}>
        <CartesianGrid stroke={CHART_GRID} strokeDasharray="3 3" />
        <XAxis dataKey="period" stroke={CHART_AXIS} tick={{ fill: CHART_MUTED_TEXT, fontSize: 12 }} />
        <YAxis
          domain={yAxisDomain}
          tickFormatter={formatUsdAxisTick}
          stroke={CHART_AXIS}
          tick={{ fill: CHART_MUTED_TEXT, fontSize: 12 }}
        />
        <Tooltip content={<ChartTooltip />} />
        <Legend />
        {level === "totals" ? (
          <Line type="monotone" dataKey="value" stroke={SEQUENTIAL_BLUE} dot={false} strokeWidth={2} />
        ) : (
          selectedSeries.map((s) => (
            <Line key={s} type="monotone" dataKey={s} stroke={colorForSeries(s, allSeries)} dot={false} strokeWidth={2} />
          ))
        )}
      </LineChart>
    </ResponsiveContainer>
  );
}
