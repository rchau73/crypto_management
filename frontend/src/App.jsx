import React, { useMemo, useState } from "react";
import { ThemeProvider, createTheme } from "@mui/material/styles";
import { Box, Container, CssBaseline, Tab, Tabs, Typography } from "@mui/material";

import { SummaryHeader } from "./components/SummaryHeader";
import { Filters } from "./components/Filters";
import { PerAssetTable } from "./components/PerAssetTable";
import { PerGroupTable } from "./components/PerGroupTable";
import { BarcaSection } from "./components/BarcaSection";
import { useAllocationsData } from "./hooks/useAllocationsData";

const darkTheme = createTheme({
  palette: {
    mode: "dark",
  },
});

const DEFAULT_USDT_BRL_RATE = Number(import.meta.env.VITE_USDT_BRL_RATE ?? 5.45);

function App() {
  const {
    allocations,
    groupAllocations,
    barcaAllocations,
    barcaActualAllocations,
    loading,
    error,
    lastUpdate,
    refresh,
    clearError,
  } = useAllocationsData();

  const [assetFilter, setAssetFilter] = useState("");
  const [groupFilter, setGroupFilter] = useState("");
  const [barcaFilter, setBarcaFilter] = useState("");
  const [tab, setTab] = useState(0);

  const filteredAllocations = useMemo(
    () =>
      allocations.filter(
        (row) =>
          (assetFilter === "" || row.symbol === assetFilter) &&
          (groupFilter === "" || row.group === groupFilter) &&
          (barcaFilter === "" || row.barca === barcaFilter)
      ),
    [allocations, assetFilter, groupFilter, barcaFilter]
  );

  const assetOptions = useMemo(
    () => Array.from(new Set(allocations.map((row) => row.symbol))).sort(),
    [allocations]
  );
  const groupOptions = useMemo(
    () => Array.from(new Set(allocations.map((row) => row.group))).sort(),
    [allocations]
  );
  const barcaOptions = useMemo(
    () => Array.from(new Set(allocations.map((row) => row.barca))).sort(),
    [allocations]
  );

  const totalWalletValue = useMemo(
    () => allocations.reduce((sum, row) => sum + (typeof row.value === "number" ? row.value : 0), 0),
    [allocations]
  );

  const totalWalletValueBRL = useMemo(() => {
    return Number.isFinite(DEFAULT_USDT_BRL_RATE)
      ? totalWalletValue * DEFAULT_USDT_BRL_RATE
      : null;
  }, [totalWalletValue]);

  return (
    <ThemeProvider theme={darkTheme}>
      <CssBaseline />
      <Container
        maxWidth="md"
        sx={{
          mt: 2,
          minHeight: "100vh",
          display: "flex",
          flexDirection: "column",
        }}
      >
        <SummaryHeader
          totalWalletValue={totalWalletValue}
          totalWalletValueBRL={totalWalletValueBRL}
          usdtBrlRate={DEFAULT_USDT_BRL_RATE}
          loading={loading}
          lastUpdate={lastUpdate}
          onRefresh={refresh}
          error={error}
          onDismissError={clearError}
        />

        <Filters
          assetOptions={assetOptions}
          groupOptions={groupOptions}
          barcaOptions={barcaOptions}
          assetFilter={assetFilter}
          groupFilter={groupFilter}
          barcaFilter={barcaFilter}
          onAssetFilterChange={setAssetFilter}
          onGroupFilterChange={setGroupFilter}
          onBarcaFilterChange={setBarcaFilter}
        />

        <Tabs value={tab} onChange={(_, value) => setTab(value)} sx={{ mb: 2 }}>
          <Tab label="Per-Asset Table" />
          <Tab label="Per-Group Table" />
          <Tab label="BARCA Actual Table" />
        </Tabs>

        <Box sx={{ flex: 1 }}>
          {tab === 0 && (
            filteredAllocations.length > 0 ? (
              <PerAssetTable rows={filteredAllocations} />
            ) : (
              <EmptyState message="No assets match the selected filters." />
            )
          )}

          {tab === 1 && (
            groupAllocations.length > 0 ? (
              <PerGroupTable rows={groupAllocations} />
            ) : (
              <EmptyState message="No group allocations available." />
            )
          )}

          {tab === 2 && (
            barcaActualAllocations.length > 0 ? (
              <BarcaSection targets={barcaAllocations} actual={barcaActualAllocations} />
            ) : (
              <EmptyState message="No BARCA allocations available." />
            )
          )}
        </Box>
      </Container>
    </ThemeProvider>
  );
}

function EmptyState({ message }) {
  return (
    <Box sx={{ display: "flex", justifyContent: "center", alignItems: "center", minHeight: 200 }}>
      <Typography variant="body1" sx={{ color: "#aaa" }}>
        {message}
      </Typography>
    </Box>
  );
}

export default App;
