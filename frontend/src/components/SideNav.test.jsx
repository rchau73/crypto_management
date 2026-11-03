import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { SideNav } from "./SideNav";
import { PAGES } from "../utils/navigation";

describe("SideNav", () => {
  it("marks the current page and navigates on click", async () => {
    const user = userEvent.setup();
    const onNavigate = vi.fn();
    render(<SideNav pages={PAGES} current="history" onNavigate={onNavigate} />);

    expect(screen.getByRole("button", { name: /History/ })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("button", { name: /Holdings/ })).not.toHaveAttribute("aria-current");

    await user.click(screen.getByRole("button", { name: /Targets/ }));
    expect(onNavigate).toHaveBeenCalledWith("targets");
  });
});
