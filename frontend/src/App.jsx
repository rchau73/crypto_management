import React, { useState } from "react";
import {
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
  Box,
  Grid,
  Tabs,
  Tab,
} from "@mui/material";
import { ThemeProvider, createTheme } from "@mui/material/styles";
import { PieChart, Pie, Cell, Legend, Tooltip, ResponsiveContainer } from "recharts";

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
    } catch (err) {
      alert("Failed to fetch allocations: " + err.message);
    }
    setLoading(false);
  };

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

  // Sort by Value (descending)
  const sortedAllocations = [...filteredAllocations].sort((a, b) => b.value - a.value);

  // Pagination logic (apply to sortedAllocations)
  const totalPages = Math.ceil(sortedAllocations.length / pageSize);
  const paginatedAllocations = sortedAllocations.slice((page - 1) * pageSize, page * pageSize);

  // Reset to page 1 whenever a filter changes
  React.useEffect(() => {
    setPage(1);
  }, [assetFilter, groupFilter, barcaFilter]);

  // Tab state
  const [tab, setTab] = useState(0);

  return (
    <ThemeProvider theme={darkTheme}>
      <Container maxWidth="md" sx={{ mt: 4 }}>
        <Typography variant="h4" gutterBottom>
          Wallet Allocations (Live)
        </Typography>
        <Button
          variant="contained"
          color="primary"
          onClick={fetchAllocations}
          disabled={loading}
          sx={{ mb: 3 }}
        >
          {loading ? <CircularProgress size={24} /> : "Update Prices & Show Distribution"}
        </Button>

        {/* Filters */}
        <Box sx={{ display: "flex", gap: 2, mb: 2 }}>
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
        </Box>

        {/* Tabs */}
        <Tabs value={tab} onChange={(_, v) => setTab(v)} sx={{ mb: 2 }}>
          <Tab label="Per-Asset Table" />
          <Tab label="Per-Group Table" />
          <Tab label="BARCA Actual Table" />
          <Tab label="Pie Charts" />
        </Tabs>

        {/* Per-Asset Table */}
        {tab === 0 && allocations.length > 0 && (
          <Box>
            <Typography variant="h6" sx={{ mt: 2, mb: 2 }}>
              Per-Asset Allocation (Current Prices)
            </Typography>
            <TableContainer component={Paper} sx={{ maxWidth: "100%", overflowX: "auto" }}>
              <Table sx={{ minWidth: 600 }}>
                <TableHead>
                  <TableRow>
                    <TableCell>Symbol</TableCell>
                    <TableCell>Group</TableCell>
                    <TableCell>BARCA</TableCell>
                    <TableCell align="right">Price</TableCell>
                    <TableCell align="right">Qty</TableCell>
                    <TableCell align="right">Value</TableCell>
                    <TableCell align="right">Target %</TableCell>
                    <TableCell align="right">Current %</TableCell>
                    <TableCell align="right">Deviation</TableCell>
                  </TableRow>
                </TableHead>
                <TableBody>
                  {paginatedAllocations.map((row, idx) => (
                    <TableRow key={idx + (page - 1) * pageSize}>
                      <TableCell sx={{ fontSize: 10 }}>{row.symbol}</TableCell>
                      <TableCell sx={{ fontSize: 10 }}>{row.group}</TableCell>
                      <TableCell sx={{ fontSize: 10 }}>{row.barca}</TableCell>
                      <TableCell align="right" sx={{ fontSize: 10 }}>{formatNumber(row.price)}</TableCell>
                      <TableCell align="right" sx={{ fontSize: 10 }}>{formatNumber(row.current_quantity)}</TableCell>
                      <TableCell align="right" sx={{ fontSize: 10 }}>${formatNumber(row.value)}</TableCell>
                      <TableCell align="right" sx={{ fontSize: 10 }}>{formatNumber(row.target_percent)}%</TableCell>
                      <TableCell align="right" sx={{ fontSize: 10 }}>{formatNumber(row.current_percent)}%</TableCell>
                      <TableCell
                        align="right"
                        sx={{
                          color: Math.abs(row.deviation) > 1 ? "error.main" : "inherit",
                          fontWeight: Math.abs(row.deviation) > 1 ? "bold" : "normal",
                          fontSize: 10, // Smaller font size for table data
                          whiteSpace: "nowrap",
                        }}
                      >
                        {row.deviation > 0 ? "+" : ""}
                        {formatNumber(row.deviation)}%
                      </TableCell>
                    </TableRow>
                  ))}
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
            </Typography>
            <TableContainer component={Paper} sx={{ maxWidth: "100%", overflowX: "auto" }}>
              <Table sx={{ minWidth: 600 }}>
                <TableHead>
                  <TableRow>
                    <TableCell>Group</TableCell>
                    <TableCell align="right">Current Value&nbsp;($)</TableCell>
                    <TableCell align="right">Current %</TableCell>
                    <TableCell align="right">Deviation</TableCell>
                  </TableRow>
                </TableHead>
                <TableBody>
                  {groupAllocations.map((g, idx) => (
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
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </TableContainer>
          </Box>
        )}

        {/* BARCA Actual Allocation Table */}
        {tab === 2 && barcaActualAllocations.length > 0 && (
          <Box>
            <Typography variant="h6" sx={{ mt: 2, mb: 2 }}>
              Per-BARCA Actual Allocation
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
          </Box>
        )}

        {/* Pie Charts */}
        {tab === 3 && (
          <Grid container spacing={4} sx={{ mt: 2 }}>
            <Grid item xs={12} md={6}>
              {groupPieData.length > 0 && (
                <Box>
                  <Typography variant="h6" sx={{ mb: 2 }}>
                    Group Target Allocation (Pie Chart)
                  </Typography>
                  <ResponsiveContainer width="100%" height={300}>
                    <PieChart>
                      <Pie
                        data={groupPieData}
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
                            fontSize={8} // Even smaller font size for better fit
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
                      <Tooltip />
                      <Legend />
                    </PieChart>
                  </ResponsiveContainer>
                </Box>
              )}
            </Grid>
            <Grid item xs={12} md={6}>
              {barcaPieData.length > 0 && (
                <Box>
                  <Typography variant="h6" sx={{ mb: 2 }}>
                    BullMarket BARCA Target Allocation (Pie Chart)
                  </Typography>
                  <ResponsiveContainer width="100%" height={300}>
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
                            fontSize={8}
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
            </Grid>
            <Grid item xs={12} md={6}>
              {barcaActualPieData.length > 0 && (
                <Box>
                  <Typography variant="h6" sx={{ mb: 2 }}>
                    BARCA Actual Allocation (Pie Chart)
                  </Typography>
                  <ResponsiveContainer width="100%" height={300}>
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
                            fontSize={8}
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
            </Grid>
          </Grid>
        )}
      </Container>
    </ThemeProvider>
  );
}

export default App;
