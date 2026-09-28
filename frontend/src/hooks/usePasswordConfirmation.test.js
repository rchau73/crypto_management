import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { usePasswordConfirmation } from "./usePasswordConfirmation";

describe("usePasswordConfirmation", () => {
  it("is invalid when both fields are empty", () => {
    const { result } = renderHook(() => usePasswordConfirmation());
    expect(result.current.matches).toBe(true); // "" === "" — matching, but...
    expect(result.current.isValid).toBe(false); // ...empty is never valid
  });

  it("is invalid while the two fields disagree", () => {
    const { result } = renderHook(() => usePasswordConfirmation());
    act(() => result.current.setPassword("hunter2"));
    act(() => result.current.setConfirmPassword("hunter3"));
    expect(result.current.matches).toBe(false);
    expect(result.current.isValid).toBe(false);
  });

  it("is valid once both fields match and are non-empty", () => {
    const { result } = renderHook(() => usePasswordConfirmation());
    act(() => result.current.setPassword("hunter2"));
    act(() => result.current.setConfirmPassword("hunter2"));
    expect(result.current.matches).toBe(true);
    expect(result.current.isValid).toBe(true);
  });

  it("reset clears both fields", () => {
    const { result } = renderHook(() => usePasswordConfirmation());
    act(() => result.current.setPassword("hunter2"));
    act(() => result.current.setConfirmPassword("hunter2"));
    act(() => result.current.reset());
    expect(result.current.password).toBe("");
    expect(result.current.confirmPassword).toBe("");
  });
});
