import { useCallback, useEffect, useState } from "react";
import { createUser, deleteUser, fetchUsers, updateUser } from "../api/client";

// Admin-only user management. Fetches when the Admin tab mounts.
export function useUsers() {
  const [users, setUsers] = useState([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const refresh = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      setUsers(await fetchUsers());
    } catch (err) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const create = useCallback(
    async (username, password, role, email, phone) => {
      await createUser(username, password, role, email, phone);
      await refresh();
    },
    [refresh]
  );

  const update = useCallback(
    async (id, changes) => {
      await updateUser(id, changes);
      await refresh();
    },
    [refresh]
  );

  // A named alias for `update(id, { password })` — same PATCH endpoint, but
  // a distinct action in the UI (and in intent) from editing a profile.
  const resetPassword = useCallback(
    async (id, password) => {
      await updateUser(id, { password });
      await refresh();
    },
    [refresh]
  );

  const remove = useCallback(
    async (id) => {
      await deleteUser(id);
      await refresh();
    },
    [refresh]
  );

  return { users, loading, error, refresh, create, update, resetPassword, remove };
}
