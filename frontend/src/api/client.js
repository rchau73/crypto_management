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

// --- Portfolios (separate wallets) ---------------------------------------
//
// Every wallet/target/history call below takes the portfolio it belongs to
// as its first argument. There is deliberately no "current portfolio" kept
// in this module: a call that forgot it would quietly use the wrong wallet.

function portfolioPath(portfolioId, rest) {
  return `/api/portfolios/${encodeURIComponent(portfolioId)}${rest}`;
}

// The portfolios the logged-in user can open: [{ id, name, access }].
export async function fetchPortfolios() {
  const res = await apiFetch("/api/portfolios");
  return parseJsonOrThrow(res, "Failed to fetch portfolios");
}

// Manager+ only. `copyBarcaFrom` starts the new portfolio with a copy of
// that portfolio's BARCA targets.
export async function createPortfolio(name, copyBarcaFrom = null) {
  const res = await postJson("/api/portfolios", { name, copy_barca_from: copyBarcaFrom });
  return parseJsonOrThrow(res, "Failed to create portfolio");
}

export async function renamePortfolio(portfolioId, name) {
  const res = await apiFetch(portfolioPath(portfolioId, ""), {
    method: "PATCH",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ name }),
  });
  return parseJsonOrThrow(res, "Failed to rename portfolio");
}

export async function fetchAllocations(portfolioId) {
  const res = await apiFetch(portfolioPath(portfolioId, "/allocations"));
  return parseJsonOrThrow(res, "Network response was not ok");
}

export async function fetchHistory(portfolioId, level) {
  const res = await apiFetch(portfolioPath(portfolioId, `/history?level=${encodeURIComponent(level)}`));
  return parseJsonOrThrow(res, "Failed to fetch history");
}

// Uploads a wallet CSV. Only new positions are added; existing ones are
// never overwritten. Manager+ only, enforced server-side.
export async function uploadWalletCsv(portfolioId, file) {
  const formData = new FormData();
  formData.append("file", file);
  const res = await apiFetch(portfolioPath(portfolioId, "/import_wallets/upload"), {
    method: "POST",
    body: formData,
  });
  return parseJsonOrThrow(res, "Import failed");
}

// Downloads the wallet in the import format (one line per source, plus
// price_usd/value_usd). Manager+ only, enforced server-side. Returns the
// file as a Blob with the server-suggested filename.
export async function downloadWalletCsv(portfolioId) {
  const res = await apiFetch(portfolioPath(portfolioId, "/export_wallets"));
  if (!res.ok) {
    const body = await res.json().catch(() => null);
    throw new Error(body?.error || "Export failed");
  }
  const disposition = res.headers.get("Content-Disposition") || "";
  const filename = disposition.match(/filename="([^"]+)"/)?.[1] || "wallet_allocations.csv";
  return { blob: await res.blob(), filename };
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

export async function fetchPortfolioTargets(portfolioId) {
  const res = await apiFetch(portfolioPath(portfolioId, "/targets"));
  return parseJsonOrThrow(res, "Failed to fetch portfolio targets");
}

// Replaces the whole portfolio-targets table in one atomic save — `rows`
// must be the complete desired state (every row, not a diff), and every
// row's target_percent across the whole set must sum to 100%.
export async function savePortfolioTargets(portfolioId, rows) {
  const res = await putJson(portfolioPath(portfolioId, "/targets"), { rows });
  return parseJsonOrThrow(res, "Failed to save portfolio targets");
}

// Fixes the quantity of a position with exactly one source, immediately
// (not part of "Save All"). The server refuses multi-source positions.
export async function correctWalletQuantity(portfolioId, { symbol, group_name, barca, asset_class, current_quantity }) {
  const res = await putJson(portfolioPath(portfolioId, "/targets/quantity"), {
    symbol,
    group_name,
    barca,
    asset_class,
    current_quantity,
  });
  return parseJsonOrThrow(res, "Failed to correct quantity");
}

// Every source (exchange/wallet) of every position, with its quantity:
// [{ symbol, group_name, barca, asset_class, current_quantity, notes }].
export async function fetchPositionSources(portfolioId) {
  const res = await apiFetch(portfolioPath(portfolioId, "/sources"));
  return parseJsonOrThrow(res, "Failed to fetch the exchanges");
}

// Records a buy or sell made on an exchange (and the cash it moved, if
// `trade.cash` is set) straight into the wallet. Returns the new quantity
// of each source changed: { asset_quantity, cash_quantity }.
export async function recordTrade(portfolioId, trade) {
  const res = await postJson(portfolioPath(portfolioId, "/trades"), trade);
  return parseJsonOrThrow(res, "Failed to record the trade");
}

// Moves a quantity to another BARCA/group and/or exchange, with no trade.
// Returns { from_quantity, to_quantity, new_position }.
export async function recordTransfer(portfolioId, transfer) {
  const res = await postJson(portfolioPath(portfolioId, "/transfers"), transfer);
  return parseJsonOrThrow(res, "Failed to move the balance");
}

export async function fetchBarcaTargets(portfolioId, market) {
  const res = await apiFetch(portfolioPath(portfolioId, `/barca/targets?market=${encodeURIComponent(market)}`));
  return parseJsonOrThrow(res, "Failed to fetch BARCA targets");
}

// Replaces every BARCA target for one market profile in one atomic save —
// `targets` must be the complete desired state for that market, and must
// sum to 100%. A barca left out of `targets` is deleted from that market.
export async function saveBarcaTargets(portfolioId, market, targets) {
  const res = await putJson(portfolioPath(portfolioId, "/barca/targets"), { market, targets });
  return parseJsonOrThrow(res, "Failed to save BARCA targets");
}
