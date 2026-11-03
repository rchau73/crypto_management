// The page and its options live in the URL hash ("#/holdings?view=barca"),
// so a reload, a bookmark or the browser's Back button keep your place
// without a router library. Pure functions, used by hooks/useHashRoute.

export function parseHash(hash) {
  const raw = String(hash || "").replace(/^#\/?/, "");
  const [path, query = ""] = raw.split("?");
  const params = {};
  new URLSearchParams(query).forEach((value, key) => {
    params[key] = value;
  });
  return { page: path.split("/")[0] || "", params };
}

// Empty and undefined params are dropped, so defaults don't clutter the URL.
export function formatHash(page, params = {}) {
  const query = new URLSearchParams();
  Object.keys(params)
    .sort()
    .forEach((key) => {
      const value = params[key];
      if (value !== undefined && value !== null && value !== "") query.set(key, String(value));
    });
  const qs = query.toString();
  return `#/${page}${qs ? `?${qs}` : ""}`;
}
