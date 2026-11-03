import { useCallback, useEffect, useState } from "react";
import { formatHash, parseHash } from "../utils/route";

// The current { page, params } from the URL hash, and `navigate(page,
// params, { replace })` to change it. Each navigation is a browser history
// entry, so Back returns to the previous page or view; `replace` updates
// the URL in place (typing in a filter box shouldn't add an entry per key).
export function useHashRoute() {
  const [route, setRoute] = useState(() => parseHash(window.location.hash));

  useEffect(() => {
    const onChange = () => setRoute(parseHash(window.location.hash));
    window.addEventListener("hashchange", onChange);
    return () => window.removeEventListener("hashchange", onChange);
  }, []);

  const navigate = useCallback((page, params = {}, { replace = false } = {}) => {
    const next = formatHash(page, params);
    if (replace) window.history.replaceState(null, "", next);
    else if (next !== window.location.hash) window.location.hash = next;
    // Set state directly too: tests (and some browsers on an identical
    // hash) don't fire hashchange synchronously.
    setRoute(parseHash(next));
  }, []);

  return { ...route, navigate };
}
