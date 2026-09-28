import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fetchAllocations, fetchCurrentUser, login, uploadWalletCsv } from "./client";

function jsonResponse(status, body) {
  return {
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
    text: async () => JSON.stringify(body),
  };
}

describe("apiFetch (via fetchAllocations)", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", vi.fn());
  });
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("sends credentials so the session cookie is included", async () => {
    fetch.mockResolvedValueOnce(jsonResponse(200, { per_asset: [] }));
    await fetchAllocations();
    expect(fetch).toHaveBeenCalledWith(expect.stringContaining("/api/allocations"), expect.objectContaining({ credentials: "include" }));
  });

  it("returns data directly when the first request succeeds", async () => {
    fetch.mockResolvedValueOnce(jsonResponse(200, { per_asset: ["ok"] }));
    const result = await fetchAllocations();
    expect(result).toEqual({ per_asset: ["ok"] });
    expect(fetch).toHaveBeenCalledTimes(1);
  });

  it("on a 401, silently refreshes the session and retries once", async () => {
    fetch
      .mockResolvedValueOnce(jsonResponse(401, { error: "expired" })) // original request
      .mockResolvedValueOnce(jsonResponse(200, {})) // /api/auth/refresh
      .mockResolvedValueOnce(jsonResponse(200, { per_asset: ["retried"] })); // retried request

    const result = await fetchAllocations();

    expect(result).toEqual({ per_asset: ["retried"] });
    expect(fetch).toHaveBeenCalledTimes(3);
    expect(fetch.mock.calls[1][0]).toContain("/api/auth/refresh");
  });

  it("propagates the original request's failure when the refresh itself fails", async () => {
    fetch
      .mockResolvedValueOnce(jsonResponse(401, { error: "expired" }))
      .mockResolvedValueOnce(jsonResponse(401, { error: "refresh also expired" }));

    // Surfaces the original endpoint's error, not the refresh endpoint's —
    // that's the more useful message for whoever's showing/logging it.
    await expect(fetchAllocations()).rejects.toThrow("expired");
    expect(fetch).toHaveBeenCalledTimes(2); // no third (retried) call
  });

  it("does not attempt a refresh loop when login itself returns 401", async () => {
    fetch.mockResolvedValueOnce(jsonResponse(401, { error: "Invalid username or password" }));
    await expect(login("alice", "wrong-password")).rejects.toThrow("Invalid username or password");
    expect(fetch).toHaveBeenCalledTimes(1); // never called /api/auth/refresh
  });

  it("fetchCurrentUser returns null instead of throwing when there is no session at all", async () => {
    fetch
      .mockResolvedValueOnce(jsonResponse(401, { error: "Not logged in" })) // /api/auth/me
      .mockResolvedValueOnce(jsonResponse(401, { error: "No refresh token" })); // /api/auth/refresh also fails
    const user = await fetchCurrentUser();
    expect(user).toBeNull();
  });

  it("fetchCurrentUser transparently recovers via refresh when only the access token expired", async () => {
    fetch
      .mockResolvedValueOnce(jsonResponse(401, { error: "expired" })) // /api/auth/me
      .mockResolvedValueOnce(jsonResponse(200, {})) // /api/auth/refresh succeeds
      .mockResolvedValueOnce(jsonResponse(200, { username: "alice", role: "user" })); // retried /me
    const user = await fetchCurrentUser();
    expect(user).toEqual({ username: "alice", role: "user" });
  });

  it("uploadWalletCsv sends the file as multipart form data, not JSON", async () => {
    fetch.mockResolvedValueOnce(jsonResponse(200, { imported: 3 }));
    const file = new File(["symbol,group\nBTC,Core\n"], "wallet.csv", { type: "text/csv" });

    const result = await uploadWalletCsv(file);

    expect(result).toEqual({ imported: 3 });
    expect(fetch).toHaveBeenCalledTimes(1);
    const [url, options] = fetch.mock.calls[0];
    expect(url).toContain("/api/import_wallets/upload");
    expect(options.method).toBe("POST");
    expect(options.body).toBeInstanceOf(FormData);
    expect(options.body.get("file")).toBe(file);
    // No explicit Content-Type — the browser must set multipart's boundary itself.
    expect(options.headers).toBeUndefined();
  });
});
