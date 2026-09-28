import { useMemo, useState } from "react";
import { Box, CircularProgress, Container, CssBaseline, LinearProgress, Tab, Tabs } from "@mui/material";
import { ThemeProvider } from "@mui/material/styles";
import { darkTheme } from "./theme";
import { useAuth } from "./hooks/useAuth";
import { useAllocations } from "./hooks/useAllocations";
import { aggregateBySymbol } from "./utils/allocationMath";
import { AppHeader } from "./components/AppHeader";
import { StatTiles } from "./components/StatTiles";
import { StatusLine } from "./components/StatusLine";
import { FiltersBar } from "./components/FiltersBar";
import { EmptyState } from "./components/EmptyState";
import { LoginPage } from "./components/LoginPage";
import { PerAssetTab } from "./components/tabs/PerAssetTab";
import { PerAssetTotalTab } from "./components/tabs/PerAssetTotalTab";
import { PerGroupTab } from "./components/tabs/PerGroupTab";
import { BarcaActualTab } from "./components/tabs/BarcaActualTab";
import { DashboardTab } from "./components/tabs/DashboardTab";
import { PortfolioTargetsTab } from "./components/tabs/PortfolioTargetsTab";
import { BarcaTargetsTab } from "./components/tabs/BarcaTargetsTab";
import { AdminTab } from "./components/tabs/AdminTab";

// Named keys instead of raw tab indices — inserting/reordering a tab (e.g.
// the Manager-only ones below) can't silently shift which panel a stale
// numeric literal points at.
const BASE_TABS = [
  { key: "per-asset", label: "Per-Asset Table" },
  { key: "per-asset-total", label: "Per-Asset Total Table" },
  { key: "per-group", label: "Per-Group Table" },
  { key: "barca-actual", label: "BARCA Actual Table" },
  { key: "dashboard", label: "Dashboard" },
];
// Tabs that need live allocation data to show anything.
const DATA_TAB_KEYS = new Set(["per-asset", "per-asset-total", "per-group", "barca-actual"]);
const MANAGER_TABS = [
  { key: "portfolio-targets", label: "Portfolio Targets" },
  { key: "barca-targets", label: "BARCA Targets" },
];
const ADMIN_TABS = [{ key: "admin", label: "Admin" }];

function uniqueSorted(values) {
  return Array.from(new Set(values)).sort();
}

function FullPageSpinner() {
  return (
    <Box sx={{ minHeight: "100vh", display: "flex", alignItems: "center", justifyContent: "center" }}>
      <CircularProgress />
    </Box>
  );
}

function Dashboard({ user, onLogout }) {
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
  const [tabIndex, setTabIndex] = useState(0);

  const canImport = user.role === "manager" || user.role === "admin";
  const isAdmin = user.role === "admin";
  const tabs = [...BASE_TABS, ...(canImport ? MANAGER_TABS : []), ...(isAdmin ? ADMIN_TABS : [])];
  const activeKey = tabs[tabIndex]?.key ?? tabs[0].key;

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
    <>
      <AppHeader
        loading={loading}
        onRefresh={refresh}
        importing={importing}
        onImport={() => importFromCsv()}
        canImport={canImport}
        username={user.username}
        role={user.role}
        onLogout={onLogout}
      />
      <Box sx={{ height: 3 }}>{loading && <LinearProgress />}</Box>

      <Container maxWidth="xl" sx={{ py: 3 }}>
        <StatTiles totalWalletValue={totalWalletValue} />
        <StatusLine lastUpdate={lastUpdate} importStatus={importStatus} />

        {hasData && DATA_TAB_KEYS.has(activeKey) && (
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

        <Tabs
          value={tabIndex}
          onChange={(_, v) => setTabIndex(v)}
          variant="scrollable"
          scrollButtons="auto"
          allowScrollButtonsMobile
          sx={{ mb: 2 }}
        >
          {tabs.map((t) => (
            <Tab key={t.key} label={t.label} />
          ))}
        </Tabs>

        {/* Dashboard/Admin/Portfolio Targets/BARCA Targets each have their
            own independent data source, so they stay reachable even before
            the first "Update Prices" click; only the four table tabs need
            allocations to show anything. */}
        {DATA_TAB_KEYS.has(activeKey) && !hasData && (
          <EmptyState
            title="No portfolio data yet"
            description='Click "Update Prices" above to fetch live prices and compute your allocation.'
          />
        )}
        {activeKey === "per-asset" && hasData && (
          <PerAssetTab rows={filteredAllocations} totalValue={relevantTotalValue} isFiltered={isFiltered} filterKey={filterKey} />
        )}
        {activeKey === "per-asset-total" && hasData && (
          <PerAssetTotalTab rows={symbolAllocations} totalValue={relevantTotalValue} isFiltered={isFiltered} filterKey={filterKey} />
        )}
        {activeKey === "per-group" && hasData && <PerGroupTab groupAllocations={groupAllocations} />}
        {activeKey === "barca-actual" && hasData && (
          <BarcaActualTab barcaAllocations={barcaAllocations} barcaActualAllocations={barcaActualAllocations} />
        )}
        {activeKey === "dashboard" && <DashboardTab active={activeKey === "dashboard"} />}
        {canImport && activeKey === "portfolio-targets" && (
          <PortfolioTargetsTab active={activeKey === "portfolio-targets"} />
        )}
        {canImport && activeKey === "barca-targets" && <BarcaTargetsTab active={activeKey === "barca-targets"} />}
        {isAdmin && activeKey === "admin" && <AdminTab active={activeKey === "admin"} currentUsername={user.username} />}
      </Container>
    </>
  );
}

function App() {
  const { user, checkingSession, login, logout, loginError, loggingIn } = useAuth();

  return (
    <ThemeProvider theme={darkTheme}>
      <CssBaseline />
      {checkingSession ? (
        <FullPageSpinner />
      ) : !user ? (
        <LoginPage onLogin={login} error={loginError} loggingIn={loggingIn} />
      ) : (
        <Dashboard user={user} onLogout={logout} />
      )}
    </ThemeProvider>
  );
}

export default App;
