import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useUsers } from "./useUsers";
import * as api from "../api/client";

vi.mock("../api/client", () => ({
  fetchUsers: vi.fn(),
  createUser: vi.fn(),
  updateUser: vi.fn(),
  deleteUser: vi.fn(),
}));

describe("useUsers", () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  it("does not fetch while inactive", () => {
    renderHook(() => useUsers(false));
    expect(api.fetchUsers).not.toHaveBeenCalled();
  });

  it("fetches the user list once active", async () => {
    api.fetchUsers.mockResolvedValue([{ id: 1, username: "admin", role: "admin" }]);
    const { result } = renderHook(() => useUsers(true));
    await waitFor(() => expect(result.current.users).toHaveLength(1));
    expect(result.current.loading).toBe(false);
  });

  it("surfaces a fetch failure as an error message, not a thrown exception", async () => {
    api.fetchUsers.mockRejectedValue(new Error("network down"));
    const { result } = renderHook(() => useUsers(true));
    await waitFor(() => expect(result.current.error).toBe("network down"));
    expect(result.current.users).toEqual([]);
  });

  it("create re-fetches the list afterwards", async () => {
    api.fetchUsers.mockResolvedValueOnce([]).mockResolvedValueOnce([{ id: 2, username: "bob", role: "user" }]);
    api.createUser.mockResolvedValue({ id: 2, username: "bob", role: "user" });

    const { result } = renderHook(() => useUsers(true));
    await waitFor(() => expect(api.fetchUsers).toHaveBeenCalledTimes(1));

    await act(async () => {
      await result.current.create("bob", "password123", "user", "bob@example.com", "+1-555-0100");
    });

    expect(api.createUser).toHaveBeenCalledWith("bob", "password123", "user", "bob@example.com", "+1-555-0100");
    expect(api.fetchUsers).toHaveBeenCalledTimes(2);
    expect(result.current.users).toEqual([{ id: 2, username: "bob", role: "user" }]);
  });

  it("resetPassword calls updateUser with only the password field and re-fetches", async () => {
    api.fetchUsers.mockResolvedValue([]);
    api.updateUser.mockResolvedValue({ ok: true });

    const { result } = renderHook(() => useUsers(true));
    await waitFor(() => expect(api.fetchUsers).toHaveBeenCalledTimes(1));

    await act(async () => {
      await result.current.resetPassword(2, "new-password123");
    });

    expect(api.updateUser).toHaveBeenCalledWith(2, { password: "new-password123" });
    expect(api.fetchUsers).toHaveBeenCalledTimes(2);
  });

  it("remove re-fetches the list afterwards", async () => {
    api.fetchUsers.mockResolvedValue([]);
    api.deleteUser.mockResolvedValue(undefined);

    const { result } = renderHook(() => useUsers(true));
    await waitFor(() => expect(api.fetchUsers).toHaveBeenCalledTimes(1));

    await act(async () => {
      await result.current.remove(2);
    });

    expect(api.deleteUser).toHaveBeenCalledWith(2);
    expect(api.fetchUsers).toHaveBeenCalledTimes(2);
  });
});
