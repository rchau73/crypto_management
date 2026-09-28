import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { LoginPage } from "./LoginPage";

describe("LoginPage", () => {
  it("disables the submit button until both fields are filled", async () => {
    const user = userEvent.setup();
    render(<LoginPage onLogin={vi.fn()} error="" loggingIn={false} />);

    expect(screen.getByRole("button", { name: "Log In" })).toBeDisabled();

    await user.type(screen.getByLabelText("Username"), "alice");
    expect(screen.getByRole("button", { name: "Log In" })).toBeDisabled();

    await user.type(screen.getByLabelText("Password"), "secret");
    expect(screen.getByRole("button", { name: "Log In" })).toBeEnabled();
  });

  it("calls onLogin with the entered credentials on submit", async () => {
    const user = userEvent.setup();
    const onLogin = vi.fn();
    render(<LoginPage onLogin={onLogin} error="" loggingIn={false} />);

    await user.type(screen.getByLabelText("Username"), "alice");
    await user.type(screen.getByLabelText("Password"), "secret");
    await user.click(screen.getByRole("button", { name: "Log In" }));

    expect(onLogin).toHaveBeenCalledWith("alice", "secret");
  });

  it("shows the error message when one is provided", () => {
    render(<LoginPage onLogin={vi.fn()} error="Invalid username or password" loggingIn={false} />);
    expect(screen.getByText("Invalid username or password")).toBeInTheDocument();
  });

  it("shows a spinner and disables submit while logging in", () => {
    const { container } = render(<LoginPage onLogin={vi.fn()} error="" loggingIn={true} />);
    expect(container.querySelector('button[type="submit"]')).toBeDisabled();
  });

  it("can reveal the password being typed", async () => {
    const user = userEvent.setup();
    render(<LoginPage onLogin={vi.fn()} error="" loggingIn={false} />);

    expect(screen.getByLabelText("Password")).toHaveAttribute("type", "password");
    await user.click(screen.getByRole("button", { name: "Show password" }));
    expect(screen.getByLabelText("Password")).toHaveAttribute("type", "text");
  });
});
