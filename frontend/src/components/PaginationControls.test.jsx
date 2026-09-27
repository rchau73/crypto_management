import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { PaginationControls } from "./PaginationControls";

describe("PaginationControls", () => {
  it("shows the current page and total", () => {
    render(<PaginationControls page={2} totalPages={5} onPageChange={() => {}} />);
    expect(screen.getByText("Page 2 of 5")).toBeInTheDocument();
  });

  it("disables Prev on the first page and Next on the last page", () => {
    render(<PaginationControls page={1} totalPages={1} onPageChange={() => {}} />);
    expect(screen.getByRole("button", { name: "Prev" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Next" })).toBeDisabled();
  });

  it("calls onPageChange with page - 1 / page + 1", async () => {
    const user = userEvent.setup();
    const onPageChange = vi.fn();
    render(<PaginationControls page={2} totalPages={5} onPageChange={onPageChange} />);

    await user.click(screen.getByRole("button", { name: "Next" }));
    expect(onPageChange).toHaveBeenCalledWith(3);

    await user.click(screen.getByRole("button", { name: "Prev" }));
    expect(onPageChange).toHaveBeenCalledWith(1);
  });
});
