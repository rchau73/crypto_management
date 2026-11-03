import { useState } from "react";
import { downloadWalletCsv, fetchAllocations, uploadWalletCsv } from "../api/client";
import { saveFile } from "../utils/saveFile";
import { describeImportSummary } from "../utils/importSummary";
import { useCurrentPortfolio } from "./useCurrentPortfolio";

// Owns the current portfolio's "live allocations" data: the
// per-asset/per-group/per-BARCA breakdown, plus the CSV import/export
// actions.
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
  const [lastUpdate, setLastUpdate] = useState(null);
  // { brl_per_usd, quoted_at } from the Banco Central, or null.
  const [usdBrl, setUsdBrl] = useState(null);

  const refresh = async () => {
    setLoading(true);
    setLoadError("");
    try {
      const data = await fetchAllocations(portfolioId);
      setAllocations(data.per_asset || []);
      setGroupAllocations(data.per_group || []);
      setBarcaAllocations(data.per_barca || []);
      setBarcaActualAllocations(data.per_barca_actual || []);
      setUsdBrl(data.usd_brl ?? null);
      setLastUpdate(new Date());
    } catch (err) {
      // Shown inline by StatusLine (alert() would block the page).
      setLoadError("Failed to fetch allocations: " + err.message);
    } finally {
      setLoading(false);
    }
  };

  const importFromCsv = async (file) => {
    setImporting(true);
    setImportStatus("");
    setImportError("");
    try {
      const data = await uploadWalletCsv(portfolioId, file);
      setImportStatus(describeImportSummary(data));
      await refresh();
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
    lastUpdate,
    usdBrl,
    refresh,
    importFromCsv,
    exportToCsv,
  };
}
