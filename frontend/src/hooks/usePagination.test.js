import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { usePagination } from "./usePagination";

const rows = Array.from({ length: 25 }, (_, i) => i);

describe("usePagination", () => {
  it("slices the first page by default", () => {
    const { result } = renderHook(() => usePagination(rows, { pageSize: 10 }));
    expect(result.current.page).toBe(1);
    expect(result.current.totalPages).toBe(3);
    expect(result.current.paginatedRows).toEqual(rows.slice(0, 10));
  });

  it("advances to the requested page", () => {
    const { result } = renderHook(() => usePagination(rows, { pageSize: 10 }));
    act(() => result.current.setPage(2));
    expect(result.current.paginatedRows).toEqual(rows.slice(10, 20));
  });

  it("clamps back to the last page if the row count shrinks below the current page", () => {
    const { result, rerender } = renderHook(({ data }) => usePagination(data, { pageSize: 10 }), {
      initialProps: { data: rows },
    });
    act(() => result.current.setPage(3));
    expect(result.current.page).toBe(3);

    rerender({ data: rows.slice(0, 5) }); // now only 1 page
    expect(result.current.page).toBe(1);
  });

  it("resets to page 1 whenever resetKey changes", () => {
    const { result, rerender } = renderHook(({ resetKey }) => usePagination(rows, { pageSize: 10, resetKey }), {
      initialProps: { resetKey: "a" },
    });
    act(() => result.current.setPage(2));
    expect(result.current.page).toBe(2);

    rerender({ resetKey: "b" });
    expect(result.current.page).toBe(1);
  });

  it("never reports fewer than 1 total page, even for an empty list", () => {
    const { result } = renderHook(() => usePagination([], { pageSize: 10 }));
    expect(result.current.totalPages).toBe(1);
    expect(result.current.paginatedRows).toEqual([]);
  });
});
