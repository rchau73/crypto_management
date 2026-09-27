// Central place for talking to the backend. Nothing else in the app should
// call `fetch()` directly — that keeps the base URL and error handling in
// exactly one spot.

export const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || "http://localhost:3001";

async function parseJsonOrThrow(res, fallbackMessage) {
  if (!res.ok) {
    const body = await res.json().catch(() => null);
    throw new Error(body?.error || fallbackMessage);
  }
  return res.json();
}

export async function fetchAllocations() {
  const res = await fetch(`${API_BASE_URL}/api/allocations`);
  return parseJsonOrThrow(res, "Network response was not ok");
}

export async function fetchHistory(level) {
  const res = await fetch(`${API_BASE_URL}/api/history?level=${level}`);
  if (!res.ok) {
    const text = await res.text().catch(() => "");
    throw new Error(`Failed to fetch history: ${res.status} ${res.statusText} ${text}`);
  }
  return res.json();
}

export async function importWallets(path = "wallet_allocations.csv") {
  const res = await fetch(`${API_BASE_URL}/api/import_wallets`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ path }),
  });
  return parseJsonOrThrow(res, "Import failed");
}
