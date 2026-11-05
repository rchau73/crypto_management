import { useEffect, useState } from "react";
import { fetchPositionSources } from "../api/client";
import { useCurrentPortfolio } from "./useCurrentPortfolio";

// The exchanges of every position, loaded each time a dialog opens (mounts)
// so the quantities shown are the latest ones. `sources` is null while
// loading.
export function usePositionSources() {
  const { id: portfolioId } = useCurrentPortfolio();
  const [state, setState] = useState({ sources: null, error: "" });
  useEffect(() => {
    let live = true;
    fetchPositionSources(portfolioId)
      .then((sources) => live && setState({ sources, error: "" }))
      .catch((err) => live && setState({ sources: [], error: err.message }));
    return () => {
      live = false;
    };
  }, [portfolioId]);
  return state;
}
