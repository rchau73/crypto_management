import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useAuth } from "./useAuth";
import * as api from "../api/client";

vi.mock("../api/client", () => ({
  fetchCurrentUser: vi.fn(),
  login: vi.fn(),
  logout: vi.fn(),
}));

describe("useAuth", () => {
  it("starts in a checking state and resolves to the session's user", async () => {
    api.fetchCurrentUser.mockResolvedValue({ username: "alice", role: "admin" });

    const { result } = renderHook(() => useAuth());
    expect(result.current.checkingSession).toBe(true);

    await waitFor(() => expect(result.current.checkingSession).toBe(false));
    expect(result.current.user).toEqual({ username: "alice", role: "admin" });
  });

  it("resolves to a logged-out state when there is no session", async () => {
    api.fetchCurrentUser.mockResolvedValue(null);

    const { result } = renderHook(() => useAuth());
    await waitFor(() => expect(result.current.checkingSession).toBe(false));
    expect(result.current.user).toBeNull();
  });

  it("resolves to logged-out (not stuck loading) if the session check itself throws", async () => {
    api.fetchCurrentUser.mockRejectedValue(new Error("network down"));

    const { result } = renderHook(() => useAuth());
    await waitFor(() => expect(result.current.checkingSession).toBe(false));
    expect(result.current.user).toBeNull();
  });

  it("login sets the user on success and clears any previous error", async () => {
    api.fetchCurrentUser.mockResolvedValue(null);
    api.login.mockResolvedValue({ username: "bob", role: "manager" });

    const { result } = renderHook(() => useAuth());
    await waitFor(() => expect(result.current.checkingSession).toBe(false));

    let success;
    await act(async () => {
      success = await result.current.login("bob", "correct-password");
    });

    expect(success).toBe(true);
    expect(result.current.user).toEqual({ username: "bob", role: "manager" });
    expect(result.current.loginError).toBe("");
  });

  it("login surfaces the error message and leaves the user logged out", async () => {
    api.fetchCurrentUser.mockResolvedValue(null);
    api.login.mockRejectedValue(new Error("Invalid username or password"));

    const { result } = renderHook(() => useAuth());
    await waitFor(() => expect(result.current.checkingSession).toBe(false));

    let success;
    await act(async () => {
      success = await result.current.login("bob", "wrong-password");
    });

    expect(success).toBe(false);
    expect(result.current.user).toBeNull();
    expect(result.current.loginError).toBe("Invalid username or password");
  });

  it("logout clears the user", async () => {
    api.fetchCurrentUser.mockResolvedValue({ username: "alice", role: "admin" });
    api.logout.mockResolvedValue(undefined);

    const { result } = renderHook(() => useAuth());
    await waitFor(() => expect(result.current.user).not.toBeNull());

    await act(async () => {
      await result.current.logout();
    });

    expect(result.current.user).toBeNull();
  });
});
