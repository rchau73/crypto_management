import React, { useState, useEffect } from "react";
import {
  CssBaseline, // <-- Add this import
  Container,
  Typography,
  Button,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Paper,
  CircularProgress,
  Box,  // ...existing code...
  Grid,
  Tabs,
  Tab,
} from "@mui/material";
import { ThemeProvider, createTheme } from "@mui/material/styles";
import { PieChart, Pie, Cell, Legend, Tooltip, ResponsiveContainer, LineChart, Line, XAxis, YAxis, CartesianGrid } from "recharts";
import dayjs from "dayjs"; // Make sure to install dayjs: npm install dayjs

const darkTheme = createTheme({
  palette: {
    mode: "dark",
  },
});

const COLORS = ["#8884d8", "#82ca9d", "#ffc658", "#ff8042", "#00C49F", "#FFBB28", "#d72660", "#3f88c5", "#f49d37", "#140f2d"];

// Helper for formatting numbers as 999,999.99
function formatNumber(n) {
  if (typeof n !== "number" || isNaN(n)) return "-";
  return n.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
}

// Helper to sum target_percent for a given table's data
function getTotalTargetPercent(rows) {
  return rows.reduce((sum, row) => {
    const val = typeof row.target_percent === "number" ? row.target_percent : parseFloat(row.target_percent);
    return sum + (isNaN(val) ? 0 : val);
  }, 0);
}

function App() {
  const [allocations, setAllocations] = useState([]);
  const [groupAllocations, setGroupAllocations] = useState([]);
  const [barcaAllocations, setBarcaAllocations] = useState([]);
  const [barcaActualAllocations, setBarcaActualAllocations] = useState([]);
  const [loading, setLoading] = useState(false);

  // New filter states
  const [assetFilter, setAssetFilter] = useState("");
  const [groupFilter, setGroupFilter] = useState("");
  const [barcaFilter, setBarcaFilter] = useState("");

  const [page, setPage] = useState(1);
  const pageSize = 10;

  // Sorting state for Per-Asset Table only
  const [sortConfig, setSortConfig] = useState({ key: "value", direction: "desc" });

  // Sorting handler
  const handleSort = (key) => {
    setSortConfig((prev) => {
      if (prev.key === key) {
        // Toggle direction
        return { key, direction: prev.direction === "asc" ? "desc" : "asc" };
      }
      return { key, direction: "asc" };
    });
  };

  // Sort function for table rows
  function getSortedRows(rows) {
    if (!sortConfig.key) return rows;
    return [...rows].sort((a, b) => {
      let aValue, bValue;
      
      // Handle calculated fields that don't exist in raw data
      if (sortConfig.key === "value_deviation") {
        // Calculate value deviation for both rows
        const isFiltered = assetFilter !== "" || groupFilter !== "" || barcaFilter !== "";
        const relevantTotalValue = isFiltered ? filteredTotalValue : totalWalletValue;
        
        const aTargetValue = relevantTotalValue * (a.target_percent / 100);
        const bTargetValue = relevantTotalValue * (b.target_percent / 100);
        aValue = a.value - aTargetValue;
        bValue = b.value - bTargetValue;
      } else if (sortConfig.key === "dca") {
        // Calculate DCA for both rows (30% of absolute value deviation)
        const isFiltered = assetFilter !== "" || groupFilter !== "" || barcaFilter !== "";
        const relevantTotalValue = isFiltered ? filteredTotalValue : totalWalletValue;
        
        const aTargetValue = relevantTotalValue * (a.target_percent / 100);
        const bTargetValue = relevantTotalValue * (b.target_percent / 100);
        const aValueDeviation = a.value - aTargetValue;
        const bValueDeviation = b.value - bTargetValue;
        aValue = Math.abs(aValueDeviation) * 0.3;
        bValue = Math.abs(bValueDeviation) * 0.3;
      } else if (sortConfig.key === "current_percent" && (assetFilter !== "" || groupFilter !== "" || barcaFilter !== "")) {
        // Use recalculated current percent for filtered views
        const relevantTotalValue = filteredTotalValue;
        aValue = relevantTotalValue > 0 ? (a.value / relevantTotalValue) * 100 : 0;
        bValue = relevantTotalValue > 0 ? (b.value / relevantTotalValue) * 100 : 0;
      } else if (sortConfig.key === "deviation" && (assetFilter !== "" || groupFilter !== "" || barcaFilter !== "")) {
        // Use recalculated deviation for filtered views
        const relevantTotalValue = filteredTotalValue;
        const aCurrentPercent = relevantTotalValue > 0 ? (a.value / relevantTotalValue) * 100 : 0;
        const bCurrentPercent = relevantTotalValue > 0 ? (b.value / relevantTotalValue) * 100 : 0;
        aValue = aCurrentPercent - a.target_percent;
        bValue = bCurrentPercent - b.target_percent;
      } else {
        // Use regular property access for other fields
        aValue = a[sortConfig.key];
        bValue = b[sortConfig.key];
      }
      
      if (aValue === undefined || bValue === undefined) return 0;
      if (typeof aValue === "number" && typeof bValue === "number") {
        return sortConfig.direction === "asc" ? aValue - bValue : bValue - aValue;
      }
      // String comparison
      return sortConfig.direction === "asc"
        ? String(aValue).localeCompare(String(bValue))
        : String(bValue).localeCompare(String(aValue));
    });
  }

  const fetchAllocations = async () => {
    setLoading(true);
    try {
      const res = await fetch("http://localhost:3001/api/allocations");
      if (!res.ok) throw new Error("Network response was not ok");
      const data = await res.json();
      setAllocations(data.per_asset || []);
      setGroupAllocations(data.per_group || []);
      setBarcaAllocations(data.per_barca || []);
      setBarcaActualAllocations(data.per_barca_actual || []);
      setLastUpdate(new Date());
    } catch (err) {
      alert("Failed to fetch allocations: " + err.message);
    }
    setLoading(false);
  };

  // Calculate total wallet value (sum of all per-asset values)
  const totalWalletValue = allocations.reduce((sum, row) => sum + (typeof row.value === "number" ? row.value : 0), 0);

  // Find USDTBRL price from allocations
  // const usdtBrlRow = allocations.find(row => row.symbol === "USDTBRL");
  const usdtBrlRate = 5.6;
  const totalWalletValueBRL = usdtBrlRate ? totalWalletValue * usdtBrlRate : null;
  // const totalWalletValueBRL = totalWalletValue * 5.6;

  // Pie data for group and barca
  const groupPieData = groupAllocations
    .filter(g => g.value > 0)
    .map(g => ({
      name: g.group,
      value: g.value, // Use the actual value, not target_percent
    }));

  const barcaPieData = barcaAllocations
    .filter(b => b.target_percent > 0)
    .map(b => ({
      name: b.barca,
      value: b.target_percent,
    }));

  // Get all unique BARCA names from both target and actual data
  const allBarcaNames = Array.from(
    new Set([
      ...barcaAllocations.map(b => b.barca),
      ...allocations.map(a => a.barca)
    ])
  );

  // Map BARCA name to color index
  const barcaNameToColorIdx = Object.fromEntries(
    allBarcaNames.map((name, idx) => [name, idx % COLORS.length])
  );

  // Actual BARCA allocation (sum value per BARCA from per-asset)
  const barcaActualPieData = barcaActualAllocations
    .filter(b => b.value > 0)
    .map(b => ({
      name: b.barca,
      value: b.value,
    }));

  // Get unique values for dropdowns
  const assetOptions = Array.from(new Set(allocations.map(a => a.symbol))).sort();
  const groupOptions = Array.from(new Set(allocations.map(a => a.group))).sort();
  const barcaOptions = Array.from(new Set(allocations.map(a => a.barca))).sort();

  // Apply filters
  const filteredAllocations = allocations.filter(row =>
    (assetFilter === "" || row.symbol === assetFilter) &&
    (groupFilter === "" || row.group === groupFilter) &&
    (barcaFilter === "" || row.barca === barcaFilter)
  );

  // Calculate filtered total value (sum of only filtered assets)  
  const filteredTotalValue = filteredAllocations.reduce((sum, row) => sum + (typeof row.value === "number" ? row.value : 0), 0);

  // Only sort Per-Asset Table
  const sortedAllocations = getSortedRows(filteredAllocations);
  const totalPages = Math.ceil(sortedAllocations.length / pageSize);
  const paginatedAllocations = sortedAllocations.slice((page - 1) * pageSize, page * pageSize);

  // Reset to page 1 whenever a filter changes
  React.useEffect(() => {
    setPage(1);
  }, [assetFilter, groupFilter, barcaFilter]);

  // Tab state
  const [tab, setTab] = useState(0);
  const [lastUpdate, setLastUpdate] = useState(null);
  // Dashboard states
  const [dashboardData, setDashboardData] = useState([]);
  const [dashboardLevel, setDashboardLevel] = useState('totals'); // 'totals' | 'assets' | 'barca'
  const [granularity, setGranularity] = useState('daily'); // daily, weekly, monthly, quarterly, yearly
  const [dashboardSeries, setDashboardSeries] = useState([]);
  const [selectedSeries, setSelectedSeries] = useState([]);
  const [dashboardLoading, setDashboardLoading] = useState(false);
  const [dashboardError, setDashboardError] = useState(null);
  const [dashboardRaw, setDashboardRaw] = useState(null);

  // Fetch historical data for dashboard
  const fetchHistory = async (level = dashboardLevel) => {
    setDashboardLoading(true);
    setDashboardError(null);
    try {
      const res = await fetch(`http://localhost:3001/api/history?level=${level}`);
      if (!res.ok) {
        const txt = await res.text().catch(() => '');
        throw new Error(`Failed to fetch history: ${res.status} ${res.statusText} ${txt}`);
      }
      const data = await res.json();
      setDashboardRaw(data);
      const rows = data.rows || [];
      // Transform rows depending on level
      let parsed = [];
      if (level === 'assets') {
        parsed = rows.map(r => {
          const rawVal = r.value !== undefined ? r.value : r['value'];
          const value = typeof rawVal === 'number' ? rawVal : Number(rawVal || 0);
          return { ts: String(r.timestamp || r['timestamp'] || ''), symbol: r.symbol || r['symbol'], value: isNaN(value) ? 0 : value };
        }).filter(x => x.ts && x.symbol);
      } else if (level === 'barca') {
        parsed = rows.map(r => {
          const rawVal = r.value !== undefined ? r.value : r['value'];
          const value = typeof rawVal === 'number' ? rawVal : Number(rawVal || 0);
          return { ts: String(r.timestamp || r['timestamp'] || ''), barca: r.barca || r['barca'], value: isNaN(value) ? 0 : value };
        }).filter(x => x.ts && x.barca);
      } else {
        parsed = rows.map(r => {
          const rawVal = r.total_value !== undefined ? r.total_value : r['total_value'];
          const value = typeof rawVal === 'number' ? rawVal : Number(rawVal || 0);
          return { ts: String(r.timestamp || r['timestamp'] || ''), value: isNaN(value) ? 0 : value };
        }).filter(x => x.ts);
      }

      // Group by granularity and pick last value per period
      const groups = {};
      parsed.forEach(item => {
        // Parse timestamp and set to BRT (UTC-3) for bucketing/period labels
        const _d0 = dayjs(String(item.ts));
        if (!_d0.isValid()) return; // skip invalid timestamps
        const d = _d0.subtract(3, 'hour');
        let key = '';
        if (granularity === '5min') {
          const minuteBucket = Math.floor(d.minute() / 5) * 5;
          key = `${d.format('YYYY-MM-DD HH')}:${String(minuteBucket).padStart(2, '0')}`;
        } else if (granularity === '30min') {
          const minuteBucket = Math.floor(d.minute() / 30) * 30;
          key = `${d.format('YYYY-MM-DD HH')}:${String(minuteBucket).padStart(2, '0')}`;
        } else if (granularity === '1h') {
          key = `${d.format('YYYY-MM-DD HH')}:00`;
        } else if (granularity === '4h') {
          const hourBucket = Math.floor(d.hour() / 4) * 4;
          key = `${d.format('YYYY-MM-DD')} ${String(hourBucket).padStart(2, '0')}:00`;
        } else if (granularity === 'daily') key = d.format('YYYY-MM-DD');
        else if (granularity === 'weekly') key = d.startOf('week').format('YYYY-MM-DD');
        else if (granularity === 'monthly') key = d.format('YYYY-MM');
        else if (granularity === 'quarterly') {
          const q = Math.floor(d.month() / 3) + 1;
          key = `${d.year()}-Q${q}`;
        } else if (granularity === 'yearly') key = `${d.year()}`;

        // For assets/barca we want the latest entry per symbol in the bucket (not accumulation)
        if (level === 'assets') {
          const symbol = item.symbol;
          groups[key] = groups[key] || { ts: null };
          // initialize container for symbol if missing
          if (!groups[key][symbol]) {
            groups[key][symbol] = { ts: item.ts, value: item.value };
          } else {
            // keep the latest timestamp/value
            if (dayjs(item.ts).isAfter(dayjs(groups[key][symbol].ts))) {
              groups[key][symbol] = { ts: item.ts, value: item.value };
            }
          }
          // ensure period ts is the latest among entries
          if (!groups[key].ts || dayjs(item.ts).isAfter(dayjs(groups[key].ts))) {
            groups[key].ts = item.ts;
          }
        } else if (level === 'barca') {
          const name = item.barca;
          groups[key] = groups[key] || { ts: null };
          if (!groups[key][name]) {
            groups[key][name] = { ts: item.ts, value: item.value };
          } else {
            if (dayjs(item.ts).isAfter(dayjs(groups[key][name].ts))) {
              groups[key][name] = { ts: item.ts, value: item.value };
            }
          }
          if (!groups[key].ts || dayjs(item.ts).isAfter(dayjs(groups[key].ts))) {
            groups[key].ts = item.ts;
          }
        } else {
          // totals: keep latest entry per period (most recent snapshot)
          if (!groups[key] || dayjs(item.ts).isAfter(dayjs(groups[key].ts))) {
            groups[key] = { ts: item.ts, value: item.value };
          }
        }
      });

      // Convert groups to ordered array
      let out = [];
      if (level === 'assets' || level === 'barca') {
        // determine available series
        const seriesSet = new Set();
        Object.values(groups).forEach(g => {
          Object.keys(g).forEach(k => { if (k !== 'ts') seriesSet.add(k); });
        });
        const series = Array.from(seriesSet);
        setDashboardSeries(series);
        if (selectedSeries.length === 0 && series.length > 0) setSelectedSeries(series);

        out = Object.keys(groups).map(k => {
          const entry = { period: k, ts: String(groups[k].ts) };
          series.forEach(s => {
            const cell = groups[k][s];
            entry[s] = cell && typeof cell === 'object' ? (cell.value || 0) : (cell || 0);
          });
          return entry;
        });
        out.sort((a,b) => (dayjs(a.ts).isBefore(dayjs(b.ts)) ? -1 : 1));
        setDashboardData(out);
      } else {
        out = Object.keys(groups).map(k => ({ period: k, value: groups[k].value, ts: groups[k].ts }));
        out.sort((a,b) => (dayjs(a.ts).isBefore(dayjs(b.ts)) ? -1 : 1));
        setDashboardData(out);
      }
    } catch (err) {
      console.error('Failed to fetch history', err);
      setDashboardError(String(err));
      setDashboardData([]);
    }
    setDashboardLoading(false);
  };

  // Custom tooltip for LineChart: show timestamp header (dark) and currency-formatted values
  const CustomTooltip = ({ active, payload, label }) => {
    if (!active || !payload || payload.length === 0) return null;
    // Prefer original timestamp if present on payload objects
    const ts = (payload[0] && payload[0].payload && (payload[0].payload.ts || payload[0].payload.timestamp)) || label;
    // Display timestamp in BRT (UTC-3)
    const _ts0 = dayjs(String(ts));
    const displayTs = _ts0.isValid() ? _ts0.subtract(3, 'hour').format('YYYY-MM-DD HH:mm:ss') : String(ts);
    return (
      <div style={{ background: '#ffffff', color: '#000000', padding: 8, borderRadius: 6, boxShadow: '0 4px 12px rgba(0,0,0,0.12)', minWidth: 140 }}>
            <div style={{ fontWeight: 700, marginBottom: 6, color: '#000' }}>{displayTs}</div>
        {payload.map((p, i) => (
          <div key={i} style={{ display: 'flex', justifyContent: 'space-between', gap: 8, alignItems: 'center' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <div style={{ width: 10, height: 10, background: p.color || '#000', borderRadius: 2 }} />
              <div style={{ color: '#111' }}>{p.name || p.dataKey}</div>
            </div>
            <div style={{ color: '#111', fontWeight: 600 }}>{typeof p.value === 'number' ? `$${formatNumber(p.value)}` : p.value}</div>
          </div>
        ))}
      </div>
    );
  };

  useEffect(() => {
    // Only fetch history when Dashboard tab is active to avoid early crashes on load
    if (tab === 3) {
      fetchHistory(dashboardLevel);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [dashboardLevel, granularity, tab]);

  return (
    <ThemeProvider theme={darkTheme}>
      <CssBaseline />
      <Container
        maxWidth="md"
        sx={{
          mt: 2, // smaller margin-top
          minHeight: "100vh",
          display: "flex",
          flexDirection: "column",
          justifyContent: "flex-start", // align to top
        }}
      >
        <Typography variant="h4" gutterBottom>
          Wallet Allocations (Live)
        </Typography>
        <Box sx={{ display: "flex", alignItems: "center", mb: 2 }}>
          <Typography variant="h6" sx={{ mr: 3 }}>
            Total Wallet Value: <span style={{ color: "#00C49F" }}>${formatNumber(totalWalletValue)}</span>
          </Typography>
          <Typography variant="h6">
            {usdtBrlRate
              ? <>Total in R$: <span style={{ color: "#FFBB28" }}>R$ {formatNumber(totalWalletValueBRL)}</span></>
              : <span style={{ color: "#FFBB28" }}>R$ --</span>
            }
          </Typography>
        </Box>
        <Button
          variant="contained"
          color="primary"
          onClick={fetchAllocations}
          disabled={loading}
          sx={{ mb: 3, mr: 2 }}
        >
          {loading ? <CircularProgress size={24} /> : "Update Prices & Show Distribution"}
        </Button>
        {lastUpdate && (
          <Typography variant="body2" sx={{ color: "#aaa", alignSelf: "center" }}>
                Last update: {dayjs(lastUpdate).subtract(3, 'hour').format("YYYY-MM-DD HH:mm:ss")}
              </Typography>
        )}

        {/* Filters */}
        <Box sx={{ display: "flex", gap: 2, mb: 2, alignItems: "center", flexWrap: "wrap" }}>
          <select value={assetFilter} onChange={e => setAssetFilter(e.target.value)}>
            <option value="">All Assets</option>
            {assetOptions.map(opt => (
              <option key={opt} value={opt}>{opt}</option>
            ))}
          </select>
          <select value={groupFilter} onChange={e => setGroupFilter(e.target.value)}>
            <option value="">All Groups</option>
            {groupOptions.map(opt => (
              <option key={opt} value={opt}>{opt}</option>
            ))}
          </select>
          <select value={barcaFilter} onChange={e => setBarcaFilter(e.target.value)}>
            <option value="">All BARCA</option>
            {barcaOptions.map(opt => (
              <option key={opt} value={opt}>{opt}</option>
            ))}
          </select>
          {(assetFilter !== "" || groupFilter !== "" || barcaFilter !== "") && (
            <Typography variant="body2" sx={{ color: "#FFBB28", fontStyle: "italic" }}>
              ⚡ Filtered View: Percentages calculated relative to filtered assets only
            </Typography>
          )}
        </Box>

        {/* Tabs */}
        <Tabs value={tab} onChange={(_, v) => setTab(v)} sx={{ mb: 2 }}>
          <Tab label="Per-Asset Table" />
          <Tab label="Per-Group Table" />
          <Tab label="BARCA Actual Table" />
          <Tab label="Dashboard" />
        </Tabs>

        {/* Per-Asset Table */}
        {tab === 0 && allocations.length > 0 && (
          <Box>
            <Typography variant="h6" sx={{ mt: 2, mb: 2 }}>
              Per-Asset Allocation (Current Prices)
              <span style={{ color: "#00C49F", fontWeight: "normal", marginLeft: 16, fontSize: 14 }}>
                Total Target %: {formatNumber(getTotalTargetPercent(filteredAllocations))}%
                {(assetFilter !== "" || groupFilter !== "" || barcaFilter !== "") && (
                  <span style={{ color: "#FFBB28", marginLeft: 8 }}>
                    (Filtered Total: ${formatNumber(filteredTotalValue)})
                  </span>
                )}
              </span>
            </Typography>
            <TableContainer component={Paper} sx={{ width: "100%" }}>
              <Table sx={{ width: "100%", tableLayout: "auto" }}>
                <TableHead>
                  <TableRow>
                    {[
                      { key: "symbol", label: "Symbol" },
                      { key: "group", label: "Group" },
                      { key: "barca", label: "BARCA" },
                      { key: "price", label: "Price", align: "right" },
                      { key: "current_quantity", label: "Qty", align: "right" },
                      { key: "value", label: "Value", align: "right" },
                      { key: "target_percent", label: "Target %", align: "right" },
                      { key: "current_percent", label: "Current %", align: "right" },
                      { key: "deviation", label: "Deviation", align: "right" },
                      { key: "value_deviation", label: "Value Deviation", align: "right" },
                      { key: "dca", label: "DCA", align: "right" },
                    ].map((col) => (
                      <TableCell
                        key={col.key}
                        align={col.align || "left"}
                        sx={{ 
                          cursor: "pointer", 
                          fontWeight: "bold", 
                          fontSize: 12,
                          width: col.key === "symbol" ? "8%" : 
                                 col.key === "group" ? "12%" : 
                                 col.key === "barca" ? "12%" : 
                                 col.key === "price" ? "10%" : 
                                 col.key === "current_quantity" ? "8%" : 
                                 col.key === "value" ? "12%" : 
                                 col.key === "target_percent" ? "8%" : 
                                 col.key === "current_percent" ? "8%" : 
                                 col.key === "deviation" ? "10%" : 
                                 col.key === "value_deviation" ? "10%" : 
                                 col.key === "dca" ? "8%" : "auto",
                          padding: "8px 4px",
                          whiteSpace: "nowrap",
                          overflow: "hidden",
                          textOverflow: "ellipsis"
                        }}
                        onClick={() => handleSort(col.key)}
                      >
                        <Button
                          size="small"
                          variant="text"
                          sx={{
                            color: sortConfig.key === col.key ? "#00C49F" : "inherit",
                            minWidth: 0,
                            fontWeight: "bold",
                            fontSize: 12,
                            textTransform: "none",
                            p: 0,
                          }}
                        >
                          {col.label}
                          {sortConfig.key === col.key ? (
                            sortConfig.direction === "asc" ? " ▲" : " ▼"
                          ) : ""}
                        </Button>
                      </TableCell>
                    ))}
                  </TableRow>
                </TableHead>
                <TableBody>
                  {paginatedAllocations.map((row, idx) => {
                    // Use filtered total if any filters are active, otherwise use total wallet value
                    const isFiltered = assetFilter !== "" || groupFilter !== "" || barcaFilter !== "";
                    const relevantTotalValue = isFiltered ? filteredTotalValue : totalWalletValue;
                    
                    // Recalculate current percent based on the relevant total
                    const recalculatedCurrentPercent = relevantTotalValue > 0 ? (row.value / relevantTotalValue) * 100 : 0;
                    
                    // Recalculate deviation based on the filtered context
                    const recalculatedDeviation = recalculatedCurrentPercent - row.target_percent;
                    
                    // Calculate target dollar value for this asset based on relevant total
                    const targetValue = relevantTotalValue * (row.target_percent / 100);
                    // Value deviation is simply current value minus target value
                    const valueDeviation = row.value - targetValue;
                    // DCA is 30% of the absolute value deviation to help with investment adjustments
                    const dca = Math.abs(valueDeviation) * 0.3;
                    return (
                      <TableRow key={idx + (page - 1) * pageSize}>
                        <TableCell sx={{ fontSize: 10, padding: "8px 4px", whiteSpace: "nowrap" }}>{row.symbol}</TableCell>
                        <TableCell sx={{ fontSize: 10, padding: "8px 4px", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis", maxWidth: "120px" }} title={row.group}>{row.group}</TableCell>
                        <TableCell sx={{ fontSize: 10, padding: "8px 4px", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis", maxWidth: "120px" }} title={row.barca}>{row.barca}</TableCell>
                        <TableCell align="right" sx={{ fontSize: 10, padding: "8px 4px" }}>{formatNumber(row.price)}</TableCell>
                        <TableCell align="right" sx={{ fontSize: 10, padding: "8px 4px" }}>{formatNumber(row.current_quantity)}</TableCell>
                        <TableCell align="right" sx={{ fontSize: 10, padding: "8px 4px" }}>${formatNumber(row.value)}</TableCell>
                        <TableCell align="right" sx={{ fontSize: 10, padding: "8px 4px" }}>{formatNumber(row.target_percent)}%</TableCell>
                        <TableCell align="right" sx={{ fontSize: 10, padding: "8px 4px" }}>{formatNumber(recalculatedCurrentPercent)}%</TableCell>
                        <TableCell
                          align="right"
                          sx={{
                            color: Math.abs(recalculatedDeviation) > 1 ? "error.main" : "inherit",
                            fontWeight: Math.abs(recalculatedDeviation) > 1 ? "bold" : "normal",
                            fontSize: 10,
                            padding: "8px 4px",
                            whiteSpace: "nowrap",
                          }}
                        >
                          {recalculatedDeviation > 0 ? "+" : ""}
                          {formatNumber(recalculatedDeviation)}%
                        </TableCell>
                        <TableCell align="right" sx={{ fontSize: 10, padding: "8px 4px" }}>
                          {valueDeviation > 0 ? "+$" : valueDeviation < 0 ? "-$" : "$"}
                          {formatNumber(Math.abs(valueDeviation))}
                        </TableCell>
                        <TableCell align="right" sx={{ fontSize: 10, padding: "8px 4px" }}>
                          ${formatNumber(dca)}
                        </TableCell>
                      </TableRow>
                    );
                  })}
                </TableBody>
              </Table>
            </TableContainer>
            {/* Pagination controls */}
            <Box sx={{ display: "flex", justifyContent: "center", alignItems: "center", mt: 2 }}>
              <Button
                variant="outlined"
                size="small"
                onClick={() => setPage(page - 1)}
                disabled={page === 1}
                sx={{ mr: 1 }}
              >
                Prev
              </Button>
              <Typography sx={{ fontSize: 12 }}>
                Page {page} of {totalPages}
              </Typography>
              <Button
                variant="outlined"
                size="small"
                onClick={() => setPage(page + 1)}
                disabled={page === totalPages}
                sx={{ ml: 1 }}
              >
                Next
              </Button>
            </Box>
          </Box>
        )}

        {/* Per-Group Table */}
        {tab === 1 && groupAllocations.length > 0 && (
          <Box>
            <Typography variant="h6" sx={{ mt: 2, mb: 2 }}>
              Per-Group Allocation
              <span style={{ color: "#00C49F", fontWeight: "normal", marginLeft: 16, fontSize: 14 }}>
                Total Target %: {formatNumber(getTotalTargetPercent(groupAllocations))}%
              </span>
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
                    const deviationValue = g.value * (g.deviation / 100); // Calculate deviation value
                    return (
                      <TableRow key={idx}>
                        <TableCell sx={{ fontSize: 10 }}>{g.group}</TableCell>
                        <TableCell align="right" sx={{ fontSize: 10 }}>${formatNumber(g.value)}</TableCell>
                        <TableCell align="right" sx={{ fontSize: 10 }}>{formatNumber(g.current_percent)}%</TableCell>
                        <TableCell
                          align="right"
                          sx={{
                            color: Math.abs(g.deviation) > 1 ? "error.main" : "inherit",
                            fontWeight: Math.abs(g.deviation) > 1 ? "bold" : "normal",
                            fontSize: 10,
                          }}
                        >
                          {g.deviation > 0 ? "+" : ""}
                          {formatNumber(g.deviation)}%
                        </TableCell>
                        <TableCell align="right" sx={{ fontSize: 10 }}>
                          ${formatNumber(deviationValue)} {/* Display deviation value */}
                        </TableCell>
                      </TableRow>
                    );
                  })}
                </TableBody>
              </Table>
            </TableContainer>
            {/* Group Pie Chart directly below the table */}
            {groupPieData.length > 0 && (
              <Box sx={{ mt: 4 }}>
                <Typography variant="h6" sx={{ mb: 2 }}>
                  Group Target Allocation (Pie Chart)
                </Typography>
                <ResponsiveContainer width={350} height={300}>
                  <PieChart>
                    <Pie
                      data={groupPieData}
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
                      {groupPieData.map((entry, index) => (
                        <Cell key={`cell-group-${index}`} fill={COLORS[index % COLORS.length]} />
                      ))}
                    </Pie>
                    <Tooltip content={<CustomTooltip />} />
                    <Legend />
                  </PieChart>
                </ResponsiveContainer>
              </Box>
            )}
          </Box>
        )}

        {/* BARCA Actual Allocation Table */}
        {tab === 2 && barcaActualAllocations.length > 0 && (
          <Box>
            <Typography variant="h6" sx={{ mt: 2, mb: 2 }}>
              Per-BARCA Actual Allocation
              <span style={{ color: "#00C49F", fontWeight: "normal", marginLeft: 16, fontSize: 14 }}>
                Total Target %: {formatNumber(getTotalTargetPercent(barcaAllocations))}%
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
                  {barcaActualAllocations.map((b, idx) => (
                    <TableRow key={idx}>
                      <TableCell sx={{ fontSize: 10 }}>{b.barca}</TableCell>
                      <TableCell align="right" sx={{ fontSize: 10 }}>${formatNumber(b.value)}</TableCell>
                      <TableCell align="right" sx={{ fontSize: 10 }}>{formatNumber(b.current_percent)}%</TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </TableContainer>
            {/* BARCA Pie Charts below the table, side by side */}
            {(barcaPieData.length > 0 || barcaActualPieData.length > 0) && (
              <Box sx={{ display: "flex", gap: 4, justifyContent: "center", mt: 4 }}>
                {barcaPieData.length > 0 && (
                  <Box sx={{ flex: 1, maxWidth: 400 }}>
                    <Typography variant="h6" sx={{ mb: 2 }}>
                      BARCA Target Allocation
                    </Typography>
                    <ResponsiveContainer width={350} height={300}>
                      <PieChart>
                        <Pie
                          data={barcaPieData}
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
                          {barcaPieData.map((entry) => (
                            <Cell
                              key={`cell-barca-target-${entry.name}`}
                              fill={COLORS[barcaNameToColorIdx[entry.name]]}
                            />
                          ))}
                        </Pie>
                        <Tooltip />
                        <Legend />
                      </PieChart>
                    </ResponsiveContainer>
                  </Box>
                )}
                {barcaActualPieData.length > 0 && (
                  <Box sx={{ flex: 1, maxWidth: 400 }}>
                    <Typography variant="h6" sx={{ mb: 2 }}>
                      BARCA Actual Allocation
                    </Typography>
                    <ResponsiveContainer width={350} height={300}>
                      <PieChart>
                        <Pie
                          data={barcaActualPieData}
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
                          {barcaActualPieData.map((entry) => (
                            <Cell
                              key={`cell-barca-actual-${entry.name}`}
                              fill={COLORS[barcaNameToColorIdx[entry.name]]}
                            />
                          ))}
                        </Pie>
                        <Tooltip />
                        <Legend />
                      </PieChart>
                    </ResponsiveContainer>
                  </Box>
                )}
              </Box>
            )}
          </Box>
        )}

        {/* Dashboard Tab */}
        {tab === 3 && (
          <Box>
            <Typography variant="h6" sx={{ mt: 2, mb: 2 }}>
              Historical Dashboard
            </Typography>
            <Box sx={{ display: 'flex', gap: 2, mb: 2, alignItems: 'center' }}>
              <select value={dashboardLevel} onChange={e => { const v = e.target.value; setDashboardLevel(v); fetchHistory(v); }}>
                <option value="totals">Totals</option>
                <option value="barca">BARCA</option>
                <option value="assets">Assets</option>
              </select>
              {(dashboardLevel === 'assets' || dashboardLevel === 'barca') && (
                <select multiple value={selectedSeries} onChange={e => setSelectedSeries(Array.from(e.target.selectedOptions, o => o.value))} style={{ minWidth: 200 }}>
                  {dashboardSeries.map(s => <option key={s} value={s}>{s}</option>)}
                </select>
              )}
              <select value={granularity} onChange={e => { const g = e.target.value; setGranularity(g); fetchHistory(dashboardLevel); }}>
                <option value="5min">5 min</option>
                <option value="30min">30 min</option>
                <option value="1h">1 hour</option>
                <option value="4h">4 hours</option>
                <option value="daily">Daily</option>
                <option value="weekly">Weekly</option>
                <option value="monthly">Monthly</option>
                <option value="quarterly">Quarterly</option>
                <option value="yearly">Yearly</option>
              </select>
              <Button variant="outlined" size="small" onClick={() => fetchHistory(dashboardLevel)}>Refresh</Button>
            </Box>
            <Box sx={{ height: 400 }}>
              {dashboardData.length === 0 ? (
                <Box>
                  <Typography variant="body2">No data available. Click "Refresh" after updating prices.</Typography>
                  {dashboardLoading && <CircularProgress size={20} sx={{ mt: 1 }} />}
                  {dashboardError && (
                    <Typography variant="body2" sx={{ color: 'error.main', mt: 1 }}>{dashboardError}</Typography>
                  )}
                  {dashboardRaw && (
                    <Box sx={{ mt: 2, bgcolor: '#0b0b0b', color: '#fff', p: 1, borderRadius: 1, maxHeight: 240, overflow: 'auto' }}>
                      <Typography variant="caption" sx={{ fontWeight: 700 }}>Raw /api/history response</Typography>
                      <pre style={{ whiteSpace: 'pre-wrap', fontSize: 11 }}>{JSON.stringify(dashboardRaw, null, 2)}</pre>
                    </Box>
                  )}
                </Box>
              ) : (
                <ResponsiveContainer width="100%" height={400}>
                  <LineChart data={dashboardData} margin={{ top: 5, right: 20, left: 10, bottom: 5 }}>
                    <CartesianGrid stroke="#333" strokeDasharray="3 3" />
                    <XAxis dataKey="period" />
                    <YAxis />
                    <Tooltip content={<CustomTooltip />} />
                    <Legend />
                    {dashboardLevel === 'totals' ? (
                      <Line type="monotone" dataKey="value" stroke="#8884d8" dot={false} />
                    ) : (
                      selectedSeries.map((s, idx) => (
                        <Line key={s} type="monotone" dataKey={s} stroke={COLORS[idx % COLORS.length]} dot={false} />
                      ))
                    )}
                  </LineChart>
                </ResponsiveContainer>
              )}
            </Box>
          </Box>
        )}
      </Container>
    </ThemeProvider>
  );
}

export default App;
