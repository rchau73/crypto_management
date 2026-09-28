// Central place for talking to the backend. Nothing else in the app should
// call `fetch()` directly — that keeps the base URL, credentials, and
// session-refresh handling in exactly one spot.

export const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || "http://localhost:3001";

async function parseJsonOrThrow(res, fallbackMessage) {
  if (!res.ok) {
    const body = await res.json().catch(() => null);
    throw new Error(body?.error || fallbackMessage);
  }
  return res.json();
}

let refreshInFlight = null;

function refreshSession() {
  if (!refreshInFlight) {
    refreshInFlight = fetch(`${API_BASE_URL}/api/auth/refresh`, { method: "POST", credentials: "include" }).finally(
      () => {
        refreshInFlight = null;
      }
    );
  }
  return refreshInFlight;
}

const AUTH_ENDPOINTS_EXEMPT_FROM_RETRY = new Set(["/api/auth/login", "/api/auth/refresh"]);

// Every request goes through here so the session cookie is always sent, and
// a single expired access token triggers one silent refresh + retry instead
// of forcing the user to log in again mid-session.
async function apiFetch(path, options = {}) {
  const doFetch = () => fetch(`${API_BASE_URL}${path}`, { ...options, credentials: "include" });
  let res = await doFetch();
  if (res.status === 401 && !AUTH_ENDPOINTS_EXEMPT_FROM_RETRY.has(path)) {
    const refreshed = await refreshSession();
    if (refreshed.ok) {
      res = await doFetch();
    }
  }
  return res;
}

function postJson(path, body) {
  return apiFetch(path, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
}

function putJson(path, body) {
  return apiFetch(path, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
}

export async function fetchAllocations() {
  const res = await apiFetch("/api/allocations");
  return parseJsonOrThrow(res, "Network response was not ok");
}

export async function fetchHistory(level) {
  const res = await apiFetch(`/api/history?level=${level}`);
  if (!res.ok) {
    const text = await res.text().catch(() => "");
    throw new Error(`Failed to fetch history: ${res.status} ${res.statusText} ${text}`);
  }
  return res.json();
}

export async function importWallets(path = "wallet_allocations.csv") {
  const res = await postJson("/api/import_wallets", { path });
  return parseJsonOrThrow(res, "Import failed");
}

// Uploads a CSV file's actual content — unlike importWallets (which asks
// the server to read a path on its own filesystem), this works regardless
// of where the backend is deployed, since the browser sends the file
// itself. Manager+ only, enforced server-side.
export async function uploadWalletCsv(file) {
  const formData = new FormData();
  formData.append("file", file);
  const res = await apiFetch("/api/import_wallets/upload", {
    method: "POST",
    body: formData,
  });
  return parseJsonOrThrow(res, "Import failed");
}

export async function login(username, password) {
  const res = await postJson("/api/auth/login", { username, password });
  return parseJsonOrThrow(res, "Invalid username or password");
}

export async function logout() {
  await apiFetch("/api/auth/logout", { method: "POST" });
}

// Returns null when there's no active session, rather than throwing — this
// is the "am I logged in?" check the app runs on load.
export async function fetchCurrentUser() {
  const res = await apiFetch("/api/auth/me");
  if (res.status === 401) return null;
  return parseJsonOrThrow(res, "Failed to fetch current user");
}

export async function fetchUsers() {
  const res = await apiFetch("/api/admin/users");
  return parseJsonOrThrow(res, "Failed to fetch users");
}

export async function createUser(username, password, role, email, phone) {
  const res = await postJson("/api/admin/users", { username, password, role, email, phone });
  return parseJsonOrThrow(res, "Failed to create user");
}

export async function updateUser(id, { role, password, email, phone } = {}) {
  const res = await apiFetch(`/api/admin/users/${id}`, {
    method: "PATCH",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ role, password, email, phone }),
  });
  return parseJsonOrThrow(res, "Failed to update user");
}

export async function deleteUser(id) {
  const res = await apiFetch(`/api/admin/users/${id}`, { method: "DELETE" });
  return parseJsonOrThrow(res, "Failed to delete user");
}

export async function fetchPortfolioTargets() {
  const res = await apiFetch("/api/portfolio/targets");
  return parseJsonOrThrow(res, "Failed to fetch portfolio targets");
}

// Replaces the whole portfolio-targets table in one atomic save — `rows`
// must be the complete desired state (every row, not a diff), and every
// row's target_percent across the whole set must sum to 100%.
export async function savePortfolioTargets(rows) {
  const res = await putJson("/api/portfolio/targets", { rows });
  return parseJsonOrThrow(res, "Failed to save portfolio targets");
}

export async function fetchBarcaTargets(market) {
  const res = await apiFetch(`/api/barca/targets?market=${encodeURIComponent(market)}`);
  return parseJsonOrThrow(res, "Failed to fetch BARCA targets");
}

// Replaces every BARCA target for one market profile in one atomic save —
// `targets` must be the complete desired state for that market, and must
// sum to 100%. A barca left out of `targets` is deleted from that market.
export async function saveBarcaTargets(market, targets) {
  const res = await putJson("/api/barca/targets", { market, targets });
  return parseJsonOrThrow(res, "Failed to save BARCA targets");
}
