import { toBrt } from "../utils/historyBucketing";
import { formatNumber } from "../utils/formatters";
import "../styles/chartTooltip.css";

// Recharts tooltip for both the pie charts and the history line chart.
// Always renders on a light card (by design, not a theme leak) so the
// series-color swatches stay legible regardless of the app's dark theme.
// Pie charts have no timestamp to show, so the header is omitted for them.
export function ChartTooltip({ active, payload, label }) {
  if (!active || !payload || payload.length === 0) return null;

  const ts = payload[0]?.payload?.ts || payload[0]?.payload?.timestamp || label;
  const brt = ts ? toBrt(String(ts)) : null;

  return (
    <div className="chart-tooltip">
      {brt?.isValid() && <div className="chart-tooltip__header">{brt.format("YYYY-MM-DD HH:mm:ss")}</div>}
      {payload.map((entry) => (
        <div className="chart-tooltip__row" key={entry.dataKey ?? entry.name}>
          <div className="chart-tooltip__series">
            <span className="chart-tooltip__swatch" style={{ background: entry.color || "#000" }} />
            <span className="chart-tooltip__label">{entry.name || entry.dataKey}</span>
          </div>
          <span className="chart-tooltip__value">
            {typeof entry.value === "number" ? `$${formatNumber(entry.value)}` : entry.value}
          </span>
        </div>
      ))}
    </div>
  );
}
