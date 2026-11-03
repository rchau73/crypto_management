import { useCallback, useEffect, useRef, useState } from "react";
import { downloadWalletCsv, fetchAllocations, uploadWalletCsv } from "../api/client";
import { saveFile } from "../utils/saveFile";
import { describeImportSummary } from "../utils/importSummary";
import { useCurrentPortfolio } from "./useCurrentPortfolio";
import { POLL_INTERVAL_MS } from "../utils/pricesAge";

// Owns the current portfolio's "live allocations" data: the
// per-asset/per-group/per-BARCA breakdown, plus the CSV import/export
// actions. It loads on mount and then polls every POLL_INTERVAL_MS while
// the page is visible; the backend decides when to call the price APIs.
// Import and export share one status/error pair, since StatusLine shows
// the outcome of whichever wallet-file action ran last.
export function useAllocations() {
  const { id: portfolioId } = useCurrentPortfolio();
  const [allocations, setAllocations] = useState([]);
  const [groupAllocations, setGroupAllocations] = useState([]);
  const [barcaAllocations, setBarcaAllocations] = useState([]);
  const [barcaActualAllocations, setBarcaActualAllocations] = useState([]);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState("");
  const [importing, setImporting] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [importStatus, setImportStatus] = useState("");
  const [importError, setImportError] = useState("");
  // When the backend fetched the prices (RFC 3339) and how often it does.
  const [pricesAsOf, setPricesAsOf] = useState(null);
  const [refreshIntervalSecs, setRefreshIntervalSecs] = useState(0);
  // { brl_per_usd, quoted_at } from the Banco Central, or null.
  const [usdBrl, setUsdBrl] = useState(null);
  const [loaded, setLoaded] = useState(false);

  const inFlight = useRef(false);
  const lastBody = useRef("");
  const mounted = useRef(true);

  // `silent` (polls): no progress bar, and a failure keeps the numbers on
  // screen, since the age shown next to them tells how old they are.
  const refresh = useCallback(
    async ({ silent = false } = {}) => {
      if (inFlight.current) return;
      inFlight.current = true;
      if (!silent) setLoading(true);
      try {
        const data = await fetchAllocations(portfolioId);
        if (!mounted.current) return;
        setLoadError("");
        // Most polls return exactly the same report: skip the re-render.
        const body = JSON.stringify(data);
        if (body !== lastBody.current) {
          lastBody.current = body;
          setAllocations(data.per_asset || []);
          setGroupAllocations(data.per_group || []);
          setBarcaAllocations(data.per_barca || []);
          setBarcaActualAllocations(data.per_barca_actual || []);
          setUsdBrl(data.usd_brl ?? null);
          setPricesAsOf(data.prices_as_of ?? null);
          setRefreshIntervalSecs(data.refresh_interval_secs ?? 0);
        }
      } catch (err) {
        // Shown inline by StatusLine (alert() would block the page).
        if (mounted.current) setLoadError("Failed to fetch allocations: " + err.message);
      } finally {
        inFlight.current = false;
        if (mounted.current) {
          setLoading(false);
          setLoaded(true);
        }
      }
    },
    [portfolioId]
  );

  useEffect(() => {
    mounted.current = true;
    refresh();
    const poll = () => {
      if (!document.hidden) refresh({ silent: true });
    };
    const id = setInterval(poll, POLL_INTERVAL_MS);
    // Coming back to the tab: catch up at once instead of waiting a tick.
    document.addEventListener("visibilitychange", poll);
    return () => {
      mounted.current = false;
      clearInterval(id);
      document.removeEventListener("visibilitychange", poll);
    };
  }, [refresh]);

  const importFromCsv = async (file) => {
    setImporting(true);
    setImportStatus("");
    setImportError("");
    try {
      const data = await uploadWalletCsv(portfolioId, file);
      setImportStatus(describeImportSummary(data));
      await refresh({ silent: true });
    } catch (err) {
      // A bad file comes back as a 400 whose message is written for a human.
      setImportError(err.message);
    } finally {
      setImporting(false);
    }
  };

  const exportToCsv = async () => {
    setExporting(true);
    setImportStatus("");
    setImportError("");
    try {
      const { blob, filename } = await downloadWalletCsv(portfolioId);
      saveFile(blob, filename);
      setImportStatus(`Exported ${filename}`);
    } catch (err) {
      setImportError(err.message);
    } finally {
      setExporting(false);
    }
  };

  return {
    allocations,
    groupAllocations,
    barcaAllocations,
    barcaActualAllocations,
    loading,
    loadError,
    importing,
    exporting,
    importStatus,
    importError,
    loaded,
    pricesAsOf,
    refreshIntervalSecs,
    usdBrl,
    importFromCsv,
    exportToCsv,
  };
}
