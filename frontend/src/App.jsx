import { useMemo, useState } from "react";
import { Box, Container, CssBaseline, LinearProgress, Tab, Tabs } from "@mui/material";
import { ThemeProvider } from "@mui/material/styles";
import { darkTheme } from "./theme";
import { useAllocations } from "./hooks/useAllocations";
import { aggregateBySymbol } from "./utils/allocationMath";
import { AppHeader } from "./components/AppHeader";
import { StatTiles } from "./components/StatTiles";
import { StatusLine } from "./components/StatusLine";
import { FiltersBar } from "./components/FiltersBar";
import { EmptyState } from "./components/EmptyState";
import { PerAssetTab } from "./components/tabs/PerAssetTab";
import { PerAssetTotalTab } from "./components/tabs/PerAssetTotalTab";
import { PerGroupTab } from "./components/tabs/PerGroupTab";
import { BarcaActualTab } from "./components/tabs/BarcaActualTab";
import { DashboardTab } from "./components/tabs/DashboardTab";

const TABS = ["Per-Asset Table", "Per-Asset Total Table", "Per-Group Table", "BARCA Actual Table", "Dashboard"];

function uniqueSorted(values) {
  return Array.from(new Set(values)).sort();
}

function App() {
  const {
    allocations,
    groupAllocations,
    barcaAllocations,
    barcaActualAllocations,
    loading,
    importing,
    importStatus,
    lastUpdate,
    refresh,
    importFromCsv,
  } = useAllocations();

  const [assetFilter, setAssetFilter] = useState("");
  const [groupFilter, setGroupFilter] = useState("");
  const [barcaFilter, setBarcaFilter] = useState("");
  const [tab, setTab] = useState(0);

  const isFiltered = assetFilter !== "" || groupFilter !== "" || barcaFilter !== "";
  const filterKey = `${assetFilter}|${groupFilter}|${barcaFilter}`;

  const totalWalletValue = allocations.reduce((sum, row) => sum + (typeof row.value === "number" ? row.value : 0), 0);

  const assetOptions = useMemo(() => uniqueSorted(allocations.map((a) => a.symbol)), [allocations]);
  const groupOptions = useMemo(() => uniqueSorted(allocations.map((a) => a.group)), [allocations]);
  const barcaOptions = useMemo(() => uniqueSorted(allocations.map((a) => a.barca)), [allocations]);

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

  const filteredTotalValue = filteredAllocations.reduce((sum, row) => sum + (typeof row.value === "number" ? row.value : 0), 0);
  const relevantTotalValue = isFiltered ? filteredTotalValue : totalWalletValue;

  const symbolAllocations = useMemo(() => aggregateBySymbol(filteredAllocations), [filteredAllocations]);

  const hasData = allocations.length > 0;

  return (
    <ThemeProvider theme={darkTheme}>
      <CssBaseline />
      <AppHeader loading={loading} onRefresh={refresh} importing={importing} onImport={() => importFromCsv()} />
      <Box sx={{ height: 3 }}>{loading && <LinearProgress />}</Box>

      <Container maxWidth="xl" sx={{ py: 3 }}>
        <StatTiles totalWalletValue={totalWalletValue} />
        <StatusLine lastUpdate={lastUpdate} importStatus={importStatus} />

        {hasData && (
          <FiltersBar
            assetFilter={assetFilter}
            onAssetFilterChange={setAssetFilter}
            assetOptions={assetOptions}
            groupFilter={groupFilter}
            onGroupFilterChange={setGroupFilter}
            groupOptions={groupOptions}
            barcaFilter={barcaFilter}
            onBarcaFilterChange={setBarcaFilter}
            barcaOptions={barcaOptions}
          />
        )}

        <Tabs value={tab} onChange={(_, v) => setTab(v)} sx={{ mb: 2 }}>
          {TABS.map((label) => (
            <Tab key={label} label={label} />
          ))}
        </Tabs>

        {/* Dashboard has its own independent data source (history, not
            allocations) so it stays reachable even before the first
            "Update Prices" click; tabs 0-3 need allocations to show anything. */}
        {tab !== 4 && !hasData && (
          <EmptyState
            title="No portfolio data yet"
            description='Click "Update Prices" above to fetch live prices and compute your allocation.'
          />
        )}
        {tab === 0 && hasData && (
          <PerAssetTab rows={filteredAllocations} totalValue={relevantTotalValue} isFiltered={isFiltered} filterKey={filterKey} />
        )}
        {tab === 1 && hasData && (
          <PerAssetTotalTab rows={symbolAllocations} totalValue={relevantTotalValue} isFiltered={isFiltered} filterKey={filterKey} />
        )}
        {tab === 2 && hasData && <PerGroupTab groupAllocations={groupAllocations} />}
        {tab === 3 && hasData && (
          <BarcaActualTab barcaAllocations={barcaAllocations} barcaActualAllocations={barcaActualAllocations} />
        )}
        {tab === 4 && <DashboardTab active={tab === 4} />}
      </Container>
    </ThemeProvider>
  );
}

export default App;
