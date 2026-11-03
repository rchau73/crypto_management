import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { SettingsTab } from "./SettingsTab";

vi.mock("./AccountsTab", () => ({ AccountsTab: () => <div>accounts screen</div> }));
vi.mock("./AdminTab", () => ({ AdminTab: () => <div>users screen</div> }));

describe("SettingsTab", () => {
  it("shows a manager only Accounts, even if the URL asks for Users", () => {
    render(<SettingsTab user={{ role: "manager", username: "m" }} params={{ tab: "users" }} onParamsChange={vi.fn()} />);
    expect(screen.queryByRole("tab", { name: "Users" })).not.toBeInTheDocument();
    expect(screen.getByText("accounts screen")).toBeInTheDocument();
  });

  it("lets an admin switch to Users through the URL params", async () => {
    const user = userEvent.setup();
    const onParamsChange = vi.fn();
    const { rerender } = render(<SettingsTab user={{ role: "admin", username: "a" }} params={{}} onParamsChange={onParamsChange} />);

    await user.click(screen.getByRole("tab", { name: "Users" }));
    expect(onParamsChange).toHaveBeenCalledWith({ tab: "users" });

    rerender(<SettingsTab user={{ role: "admin", username: "a" }} params={{ tab: "users" }} onParamsChange={onParamsChange} />);
    expect(screen.getByText("users screen")).toBeInTheDocument();
  });
});
