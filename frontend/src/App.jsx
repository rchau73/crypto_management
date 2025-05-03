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
} from "@mui/material";
import { ThemeProvider, createTheme } from "@mui/material/styles";

const darkTheme = createTheme({
  palette: {
    mode: "dark",
  },
});

function App() {
  const [allocations, setAllocations] = useState([]);
  const [groupAllocations, setGroupAllocations] = useState([]);
  const [loading, setLoading] = useState(false);

  const fetchAllocations = async () => {
    setLoading(true);
    try {
      const res = await fetch("http://localhost:3001/api/allocations");
      if (!res.ok) throw new Error("Network response was not ok");
      const data = await res.json();
      setAllocations(data.per_asset || []);
      setGroupAllocations(data.per_group || []);
    } catch (err) {
      alert("Failed to fetch allocations: " + err.message);
    }
    setLoading(false);
  };

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

        {allocations.length > 0 && (
          <Box>
            <Typography variant="h6" sx={{ mt: 4, mb: 2 }}>
              Per-Asset Allocation (Current Prices)
            </Typography>
            <TableContainer component={Paper}>
              <Table>
                <TableHead>
                  <TableRow>
                    <TableCell>Symbol</TableCell>
                    <TableCell>Group</TableCell>
                    <TableCell align="right">Price</TableCell>
                    <TableCell align="right">Qty</TableCell>
                    <TableCell align="right">Value</TableCell>
                    <TableCell align="right">Target %</TableCell>
                    <TableCell align="right">Current %</TableCell>
                    <TableCell align="right">Deviation</TableCell>
                  </TableRow>
                </TableHead>
                <TableBody>
                  {allocations.map((row, idx) => (
                    <TableRow key={idx}>
                      <TableCell>{row.symbol}</TableCell>
                      <TableCell>{row.group}</TableCell>
                      <TableCell align="right">${row.price}</TableCell>
                      <TableCell align="right">{row.current_quantity}</TableCell>
                      <TableCell align="right">${row.value.toFixed(2)}</TableCell>
                      <TableCell align="right">{row.target_percent}%</TableCell>
                      <TableCell align="right">{row.current_percent.toFixed(2)}%</TableCell>
                      <TableCell
                        align="right"
                        sx={{
                          color: Math.abs(row.deviation) > 1 ? "error.main" : "inherit",
                          fontWeight: Math.abs(row.deviation) > 1 ? "bold" : "normal",
                        }}
                      >
                        {row.deviation > 0 ? "+" : ""}
                        {row.deviation.toFixed(2)}%
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </TableContainer>

            <Typography variant="h6" sx={{ mt: 4, mb: 2 }}>
              Per-Group Allocation
            </Typography>
            <TableContainer component={Paper}>
              <Table>
                <TableHead>
                  <TableRow>
                    <TableCell>Group</TableCell>
                    <TableCell align="right">Target %</TableCell>
                    <TableCell align="right">Current %</TableCell>
                    <TableCell align="right">Deviation</TableCell>
                  </TableRow>
                </TableHead>
                <TableBody>
                  {groupAllocations.map((g, idx) => (
                    <TableRow key={idx}>
                      <TableCell>{g.group}</TableCell>
                      <TableCell align="right">{g.target_percent}%</TableCell>
                      <TableCell align="right">{g.current_percent.toFixed(2)}%</TableCell>
                      <TableCell
                        align="right"
                        sx={{
                          color: Math.abs(g.deviation) > 1 ? "error.main" : "inherit",
                          fontWeight: Math.abs(g.deviation) > 1 ? "bold" : "normal",
                        }}
                      >
                        {g.deviation > 0 ? "+" : ""}
                        {g.deviation.toFixed(2)}%
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </TableContainer>
          </Box>
        )}
      </Container>
    </ThemeProvider>
  );
}

export default App;
