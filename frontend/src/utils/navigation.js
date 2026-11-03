// The side menu's pages and who sees each. Pure, so the role rules are
// tested once here instead of in every component that needs them.

export const PAGES = [
  { key: "holdings", label: "Holdings", hint: "every position vs target" },
  { key: "history", label: "History", hint: "value over time" },
  { key: "targets", label: "Targets", hint: "asset and BARCA targets", needs: "edit" },
  { key: "settings", label: "Settings", hint: "accounts and users", needs: "manager" },
];
export const DEFAULT_PAGE = "holdings";

export function isManager(user) {
  return user?.role === "manager" || user?.role === "admin";
}

// Editing needs both a manager role and `manage` access to this account
// (the server enforces the same rule).
export function canEdit(user, portfolio) {
  return isManager(user) && portfolio?.access === "manage";
}

export function visiblePages(user, portfolio) {
  return PAGES.filter((p) => {
    if (p.needs === "edit") return canEdit(user, portfolio);
    if (p.needs === "manager") return isManager(user);
    return true;
  });
}

// The page to show for a URL: an unknown or hidden page falls back to the
// default instead of a blank screen.
export function resolvePage(requested, user, portfolio) {
  const pages = visiblePages(user, portfolio);
  return pages.some((p) => p.key === requested) ? requested : DEFAULT_PAGE;
}

// The sub-tab to show for a URL param: an unknown one falls back to the first.
export function selectedTab(tabs, current) {
  return tabs.some((t) => t.key === current) ? current : tabs[0].key;
}
