import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AdminTab } from "./AdminTab";
import { useUsers } from "../../hooks/useUsers";

vi.mock("../../hooks/useUsers");

function setupUseUsers(overrides = {}) {
  const hookValue = {
    users: [
      { id: 1, username: "admin", role: "admin", email: "admin@example.com", phone: null, created_at: "2026-01-01" },
      { id: 2, username: "bob", role: "user", email: "bob@example.com", phone: "+1-555-0100", created_at: "2026-01-02" },
    ],
    loading: false,
    error: "",
    create: vi.fn().mockResolvedValue(undefined),
    update: vi.fn().mockResolvedValue(undefined),
    resetPassword: vi.fn().mockResolvedValue(undefined),
    remove: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  };
  useUsers.mockReturnValue(hookValue);
  return hookValue;
}

describe("AdminTab", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("lists existing users with their email, phone, role, and created date", () => {
    setupUseUsers();
    render(<AdminTab currentUsername="admin" />);
    expect(screen.getAllByText("admin").length).toBeGreaterThan(0);
    expect(screen.getByText("bob@example.com")).toBeInTheDocument();
    expect(screen.getByText("+1-555-0100")).toBeInTheDocument();
  });

  it("shows a dash when phone is absent", () => {
    setupUseUsers();
    render(<AdminTab currentUsername="admin" />);
    const adminRow = screen.getByText("admin@example.com").closest("tr");
    expect(within(adminRow).getByText("-")).toBeInTheDocument();
  });

  it("disables deleting your own account", () => {
    setupUseUsers();
    render(<AdminTab currentUsername="admin" />);
    const adminRow = screen.getAllByText("admin")[0].closest("tr");
    expect(within(adminRow).getByRole("button", { name: "Delete" })).toBeDisabled();

    const bobRow = screen.getByText("bob").closest("tr");
    expect(within(bobRow).getByRole("button", { name: "Delete" })).toBeEnabled();
  });

  it("calls remove after the user confirms deletion", async () => {
    const user = userEvent.setup();
    const hook = setupUseUsers();
    vi.spyOn(window, "confirm").mockReturnValue(true);
    render(<AdminTab currentUsername="admin" />);

    const bobRow = screen.getByText("bob").closest("tr");
    await user.click(within(bobRow).getByRole("button", { name: "Delete" }));

    expect(hook.remove).toHaveBeenCalledWith(2);
  });

  it("does not call remove when the user cancels the confirmation", async () => {
    const user = userEvent.setup();
    const hook = setupUseUsers();
    vi.spyOn(window, "confirm").mockReturnValue(false);
    render(<AdminTab currentUsername="admin" />);

    const bobRow = screen.getByText("bob").closest("tr");
    await user.click(within(bobRow).getByRole("button", { name: "Delete" }));

    expect(hook.remove).not.toHaveBeenCalled();
  });

  it("shows an empty-state row when there are no users", () => {
    setupUseUsers({ users: [] });
    render(<AdminTab currentUsername="admin" />);
    expect(screen.getByText("No users yet.")).toBeInTheDocument();
  });

  it("surfaces a fetch error from the hook", () => {
    setupUseUsers({ error: "Failed to fetch users" });
    render(<AdminTab currentUsername="admin" />);
    expect(screen.getByText("Failed to fetch users")).toBeInTheDocument();
  });

  describe("creating a user", () => {
    async function openForm(user) {
      await user.click(screen.getByRole("button", { name: "+ Add User" }));
    }

    it("keeps Create disabled until username, email, and matching passwords are all present", async () => {
      const user = userEvent.setup();
      setupUseUsers();
      render(<AdminTab currentUsername="admin" />);
      await openForm(user);

      const createButton = screen.getByRole("button", { name: "Create" });
      expect(createButton).toBeDisabled();

      await user.type(screen.getByLabelText("Username"), "carol");
      await user.type(screen.getByLabelText("Email"), "carol@example.com");
      expect(createButton).toBeDisabled(); // still no password

      await user.type(screen.getByLabelText("Password"), "password123");
      await user.type(screen.getByLabelText("Confirm Password"), "not-the-same");
      expect(createButton).toBeDisabled(); // mismatched

      await user.clear(screen.getByLabelText("Confirm Password"));
      await user.type(screen.getByLabelText("Confirm Password"), "password123");
      expect(createButton).toBeEnabled();
    });

    it("submits username, matched password, role, email, and phone", async () => {
      const user = userEvent.setup();
      const hook = setupUseUsers();
      render(<AdminTab currentUsername="admin" />);
      await openForm(user);

      await user.type(screen.getByLabelText("Username"), "carol");
      await user.type(screen.getByLabelText("Email"), "carol@example.com");
      await user.type(screen.getByLabelText("Phone (optional)"), "+1-555-0199");
      await user.type(screen.getByLabelText("Password"), "password123");
      await user.type(screen.getByLabelText("Confirm Password"), "password123");
      await user.click(screen.getByRole("button", { name: "Create" }));

      expect(hook.create).toHaveBeenCalledWith("carol", "password123", "user", "carol@example.com", "+1-555-0199");
    });

    it("submits undefined phone when left blank", async () => {
      const user = userEvent.setup();
      const hook = setupUseUsers();
      render(<AdminTab currentUsername="admin" />);
      await openForm(user);

      await user.type(screen.getByLabelText("Username"), "carol");
      await user.type(screen.getByLabelText("Email"), "carol@example.com");
      await user.type(screen.getByLabelText("Password"), "password123");
      await user.type(screen.getByLabelText("Confirm Password"), "password123");
      await user.click(screen.getByRole("button", { name: "Create" }));

      expect(hook.create).toHaveBeenCalledWith("carol", "password123", "user", "carol@example.com", undefined);
    });
  });

  describe("resetting a password", () => {
    it("reveals matching password fields and calls resetPassword with the new password", async () => {
      const user = userEvent.setup();
      const hook = setupUseUsers();
      render(<AdminTab currentUsername="admin" />);

      const bobRow = screen.getByText("bob").closest("tr");
      await user.click(within(bobRow).getByRole("button", { name: "Reset Password" }));

      const passwordField = within(bobRow).getByLabelText("Password");
      const confirmField = within(bobRow).getByLabelText("Confirm Password");
      const saveButton = within(bobRow).getByRole("button", { name: "Save" });
      expect(saveButton).toBeDisabled();

      await user.type(passwordField, "new-password123");
      await user.type(confirmField, "new-password123");
      expect(saveButton).toBeEnabled();

      await user.click(saveButton);
      expect(hook.resetPassword).toHaveBeenCalledWith(2, "new-password123");
    });

    it("keeps Save disabled while the two password fields disagree", async () => {
      const user = userEvent.setup();
      setupUseUsers();
      render(<AdminTab currentUsername="admin" />);

      const bobRow = screen.getByText("bob").closest("tr");
      await user.click(within(bobRow).getByRole("button", { name: "Reset Password" }));
      await user.type(within(bobRow).getByLabelText("Password"), "new-password123");
      await user.type(within(bobRow).getByLabelText("Confirm Password"), "different");

      expect(within(bobRow).getByRole("button", { name: "Save" })).toBeDisabled();
      expect(screen.getByText("Passwords don't match")).toBeInTheDocument();
    });

    it("cancel hides the fields again without calling resetPassword", async () => {
      const user = userEvent.setup();
      const hook = setupUseUsers();
      render(<AdminTab currentUsername="admin" />);

      const bobRow = screen.getByText("bob").closest("tr");
      await user.click(within(bobRow).getByRole("button", { name: "Reset Password" }));
      await user.click(within(bobRow).getByRole("button", { name: "Cancel" }));

      expect(within(bobRow).getByRole("button", { name: "Reset Password" })).toBeInTheDocument();
      expect(hook.resetPassword).not.toHaveBeenCalled();
    });
  });
});
