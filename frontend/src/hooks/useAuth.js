import { useCallback, useEffect, useState } from "react";
import { fetchCurrentUser, login as apiLogin, logout as apiLogout } from "../api/client";

// Owns "who is logged in" for the whole app. On mount it checks for an
// existing session — a still-valid refresh token silently re-authenticates
// the user without ever showing the login screen (see api/client's
// refresh-on-401 handling).
export function useAuth() {
  const [user, setUser] = useState(null);
  const [checkingSession, setCheckingSession] = useState(true);
  const [loginError, setLoginError] = useState("");
  const [loggingIn, setLoggingIn] = useState(false);

  useEffect(() => {
    let cancelled = false;
    fetchCurrentUser()
      .then((u) => {
        if (!cancelled) setUser(u);
      })
      .catch(() => {
        if (!cancelled) setUser(null);
      })
      .finally(() => {
        if (!cancelled) setCheckingSession(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const login = useCallback(async (username, password) => {
    setLoggingIn(true);
    setLoginError("");
    try {
      const loggedInUser = await apiLogin(username, password);
      setUser(loggedInUser);
      return true;
    } catch (err) {
      setLoginError(err.message);
      return false;
    } finally {
      setLoggingIn(false);
    }
  }, []);

  const logout = useCallback(async () => {
    await apiLogout();
    setUser(null);
  }, []);

  return { user, checkingSession, login, logout, loginError, loggingIn };
}
