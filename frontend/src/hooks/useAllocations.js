import { useState } from "react";
import { fetchAllocations, importWallets } from "../api/client";

// Owns the "live allocations" data: the per-asset/per-group/per-BARCA
// breakdown from /api/allocations, plus the CSV import action that feeds it.
export function useAllocations() {
  const [allocations, setAllocations] = useState([]);
  const [groupAllocations, setGroupAllocations] = useState([]);
  const [barcaAllocations, setBarcaAllocations] = useState([]);
  const [barcaActualAllocations, setBarcaActualAllocations] = useState([]);
  const [loading, setLoading] = useState(false);
  const [importing, setImporting] = useState(false);
  const [importStatus, setImportStatus] = useState("");
  const [lastUpdate, setLastUpdate] = useState(null);

  const refresh = async () => {
    setLoading(true);
    try {
      const data = await fetchAllocations();
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

  const importFromCsv = async (path = "wallet_allocations.csv") => {
    setImporting(true);
    setImportStatus("");
    try {
      const data = await importWallets(path);
      setImportStatus(`Imported ${data.imported ?? 0} wallet rows from CSV`);
      await refresh();
    } catch (err) {
      alert("Failed to import wallet allocations: " + err.message);
    }
    setImporting(false);
  };

  return {
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
  };
}
