import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { PasswordField } from "./PasswordField";

describe("PasswordField", () => {
  it("masks the value by default", () => {
    render(<PasswordField label="Password" value="secret" onChange={vi.fn()} />);
    expect(screen.getByLabelText("Password")).toHaveAttribute("type", "password");
  });

  it("reveals the value as plain text when Show is clicked, and re-masks on Hide", async () => {
    const user = userEvent.setup();
    render(<PasswordField label="Password" value="secret" onChange={vi.fn()} />);

    await user.click(screen.getByRole("button", { name: "Show password" }));
    expect(screen.getByLabelText("Password")).toHaveAttribute("type", "text");

    await user.click(screen.getByRole("button", { name: "Hide password" }));
    expect(screen.getByLabelText("Password")).toHaveAttribute("type", "password");
  });

  it("still calls onChange like a normal text field", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<PasswordField label="Password" value="" onChange={onChange} />);
    await user.type(screen.getByLabelText("Password"), "a");
    expect(onChange).toHaveBeenCalled();
  });
});
