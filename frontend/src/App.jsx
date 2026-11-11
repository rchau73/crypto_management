import { useMemo, useState } from "react";
import { Box, CircularProgress, Container, CssBaseline, Drawer, LinearProgress, useMediaQuery } from "@mui/material";
import { ThemeProvider } from "@mui/material/styles";
import { darkTheme } from "./theme";
import { useAuth } from "./hooks/useAuth";
import { useAllocations } from "./hooks/useAllocations";
import { useHashRoute } from "./hooks/useHashRoute";
import { canEdit, resolvePage, visiblePages } from "./utils/navigation";
import { AppHeader } from "./components/AppHeader";
import { GlobalSearch } from "./components/GlobalSearch";
import { SideNav } from "./components/SideNav";
import { StatTiles } from "./components/StatTiles";
import { StatusLine } from "./components/StatusLine";
import { EmptyState } from "./components/EmptyState";
import { LoginPage } from "./components/LoginPage";
import { PortfolioGate } from "./components/PortfolioGate";
import { RebalanceTab } from "./components/tabs/RebalanceTab";
import { HoldingsTab } from "./components/tabs/HoldingsTab";
import { DashboardTab } from "./components/tabs/DashboardTab";
import { TargetsTab } from "./components/tabs/TargetsTab";
import { SettingsTab } from "./components/tabs/SettingsTab";

const NAV_WIDTH = 240;

function FullPageSpinner() {
  return (
    <Box sx={{ minHeight: "100vh", display: "flex", alignItems: "center", justifyContent: "center" }}>
      <CircularProgress />
    </Box>
  );
}

// Everything inside one portfolio. PortfolioGate remounts this when the
// portfolio changes, so its state always belongs to `portfolio`; the page
// and its options live in the URL, so they survive that remount.
function Dashboard({ user, portfolio, onLogout }) {
  const {
    allocations,
    groupAllocations,
    barcaAllocations,
    barcaActualAllocations,
    unpriced,
    loading,
    loaded,
    loadError,
    importing,
    exporting,
    importStatus,
    importError,
    pricesAsOf,
    refreshIntervalSecs,
    usdBrl,
    importFromCsv,
    exportToCsv,
    reload,
  } = useAllocations();
  const route = useHashRoute();
  const wide = useMediaQuery(darkTheme.breakpoints.up("md"));
  const [menuOpen, setMenuOpen] = useState(false);
  // Today's market sentiment and the steps ticked off, for this visit and
  // account only: PortfolioGate remounts this component on an account
  // switch and a reload starts over, so the sentiment is asked again.
  const [sentiment, setSentiment] = useState(null);
  const [doneSteps, setDoneSteps] = useState(() => new Set());
  const toggleStep = (id) =>
    setDoneSteps((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });

  const pages = useMemo(() => visiblePages(user, portfolio), [user, portfolio]);
  const page = resolvePage(route.page, user, portfolio);
  const canImport = canEdit(user, portfolio);
  const report = useMemo(
    () => ({ allocations, groupAllocations, barcaAllocations, barcaActualAllocations, unpriced }),
    [allocations, groupAllocations, barcaAllocations, barcaActualAllocations, unpriced]
  );
  const totalWalletValue = allocations.reduce((sum, row) => sum + (typeof row.value === "number" ? row.value : 0), 0);
  // An account holding only unpriced assets still gets its pages, where
  // UnpricedAlert says why the numbers are missing.
  const hasData = allocations.length > 0 || unpriced.length > 0;

  const go = (key, params = {}) => {
    route.navigate(key, params);
    setMenuOpen(false);
  };
  const setParams = (params, options) => route.navigate(page, params, options);
  const nav = <SideNav pages={pages} current={page} onNavigate={(key) => go(key)} />;

  return (
    <>
      <AppHeader
        onOpenMenu={wide ? undefined : () => setMenuOpen(true)}
        search={<GlobalSearch pages={pages} allocations={allocations} onGo={go} />}
        pricesAsOf={pricesAsOf}
        refreshIntervalSecs={refreshIntervalSecs}
        accountName={portfolio.name}
        importing={importing}
        onImport={importFromCsv}
        exporting={exporting}
        onExport={exportToCsv}
        canImport={canImport}
        username={user.username}
        role={user.role}
        onLogout={onLogout}
      />
      <Box sx={{ height: 3 }}>{loading && <LinearProgress />}</Box>

      <Box sx={{ display: "flex", alignItems: "flex-start" }}>
        {wide ? (
          <Box
            component="aside"
            sx={{ width: NAV_WIDTH, flexShrink: 0, position: "sticky", top: 80, borderRight: 1, borderColor: "divider", minHeight: "60vh" }}
          >
            {nav}
          </Box>
        ) : (
          <Drawer open={menuOpen} onClose={() => setMenuOpen(false)} slotProps={{ paper: { sx: { width: NAV_WIDTH } } }}>
            {nav}
          </Drawer>
        )}

        <Container component="main" maxWidth="xl" sx={{ py: 3, flex: 1, minWidth: 0 }}>
          <StatTiles totalWalletValue={totalWalletValue} usdBrl={usdBrl} />
          <StatusLine pricesAsOf={pricesAsOf} importStatus={importStatus} importError={importError || loadError} />

          {/* Before the first answer the progress bar shows; after a failed
              load, StatusLine says why. History, Targets and Settings have
              their own data, so they work for an account with no holdings. */}
          {page === "rebalance" && hasData && (
            <RebalanceTab
              report={report}
              sentiment={sentiment}
              onSentimentChange={setSentiment}
              done={doneSteps}
              onToggleDone={toggleStep}
              pricesAsOf={pricesAsOf}
              refreshIntervalSecs={refreshIntervalSecs}
              onOpenHoldings={() => go("holdings", { out: "1" })}
              onOpenUnpriced={() => go("holdings")}
              accountName={portfolio.name}
              canRecord={canImport}
              onTradeRecorded={() => reload({ silent: true })}
            />
          )}
          {page === "holdings" && hasData && <HoldingsTab report={report} params={route.params} onParamsChange={setParams} />}
          {(page === "rebalance" || page === "holdings") && loaded && !hasData && !loadError && (
            <EmptyState
              title={`No holdings in ${portfolio.name} yet`}
              description={
                canImport
                  ? `Import this account's wallet CSV ("Import CSV into ${portfolio.name}" above) to see its allocations.`
                  : "Its allocations appear here once its manager imports the account's wallet."
              }
            />
          )}
          {page === "history" && <DashboardTab />}
          {page === "targets" && <TargetsTab params={route.params} onParamsChange={setParams} />}
          {page === "settings" && <SettingsTab user={user} params={route.params} onParamsChange={setParams} />}
        </Container>
      </Box>
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
        <PortfolioGate onLogout={logout}>
          {(portfolio) => <Dashboard user={user} portfolio={portfolio} onLogout={logout} />}
        </PortfolioGate>
      )}
    </ThemeProvider>
  );
}

export default App;
