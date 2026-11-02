// Pure helpers for choosing which portfolio the app shows.

// The portfolio to show: the remembered one if the user can still open it,
// otherwise the first. `null` only when the list is empty.
export function pickPortfolio(portfolios, rememberedId) {
  return portfolios.find((p) => p.id === rememberedId) || portfolios[0] || null;
}

const STORAGE_KEY = "cm.selectedPortfolioId";

// The last selected portfolio id, per browser. Only a convenience: storage
// can be blocked (private window), so failures just mean "no memory".
export function loadRememberedPortfolioId() {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    return raw === null ? null : Number(raw);
  } catch {
    return null;
  }
}

export function rememberPortfolioId(id) {
  try {
    window.localStorage.setItem(STORAGE_KEY, String(id));
  } catch {
    // Not remembered next time; nothing else depends on it.
  }
}
