import { useState } from "react";
import { fetchAllocations, uploadWalletCsv } from "../api/client";

// Owns the "live allocations" data: the per-asset/per-group/per-BARCA
// breakdown from /api/allocations, plus the CSV import action that feeds it.
export function useAllocations() {
  const [allocations, setAllocations] = useState([]);
  const [groupAllocations, setGroupAllocations] = useState([]);
  const [barcaAllocations, setBarcaAllocations] = useState([]);
  const [barcaActualAllocations, setBarcaActualAllocations] = useState([]);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState("");
  const [importing, setImporting] = useState(false);
  const [importStatus, setImportStatus] = useState("");
  const [importError, setImportError] = useState("");
  const [lastUpdate, setLastUpdate] = useState(null);

  const refresh = async () => {
    setLoading(true);
    setLoadError("");
    try {
      const data = await fetchAllocations();
      setAllocations(data.per_asset || []);
      setGroupAllocations(data.per_group || []);
      setBarcaAllocations(data.per_barca || []);
      setBarcaActualAllocations(data.per_barca_actual || []);
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
      const data = await uploadWalletCsv(file);
      setImportStatus(`Imported ${data.imported ?? 0} wallet rows from CSV`);
      await refresh();
    } catch (err) {
      // A bad file comes back as a 400 whose message is written for a human.
      setImportError(err.message);
    } finally {
      setImporting(false);
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
    importStatus,
    importError,
    lastUpdate,
    refresh,
    importFromCsv,
  };
}
