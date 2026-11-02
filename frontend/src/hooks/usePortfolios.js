import { useEffect, useState } from "react";
import { fetchPortfolios } from "../api/client";

// The portfolios the logged-in user can open, fetched once on mount.
export function usePortfolios() {
  const [portfolios, setPortfolios] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  useEffect(() => {
    fetchPortfolios()
      .then(setPortfolios)
      .catch((err) => setError(err.message))
      .finally(() => setLoading(false));
  }, []);

  return { portfolios, loading, error };
}
