import { useState } from "react";
import {
  Box,
  Button,
  FormControl,
  InputLabel,
  MenuItem,
  Paper,
  Select,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  TextField,
  Typography,
} from "@mui/material";
import { useUsers } from "../../hooks/useUsers";
import { usePasswordConfirmation } from "../../hooks/usePasswordConfirmation";
import { PasswordField } from "../PasswordField";
import { zebraRowSx } from "../../styles/tableStyles";

const ROLES = ["user", "manager", "admin"];

// Two password fields that must agree before submitting is allowed — shared
// shape between "create a user" and "reset a password".
function PasswordConfirmFields({ pw, size = "medium" }) {
  return (
    <>
      <PasswordField size={size} label="Password" value={pw.password} onChange={(e) => pw.setPassword(e.target.value)} />
      <PasswordField
        size={size}
        label="Confirm Password"
        value={pw.confirmPassword}
        onChange={(e) => pw.setConfirmPassword(e.target.value)}
        error={pw.confirmPassword.length > 0 && !pw.matches}
        helperText={pw.confirmPassword.length > 0 && !pw.matches ? "Passwords don't match" : " "}
      />
    </>
  );
}

function NewUserForm({ onCreate, onCancel }) {
  const [username, setUsername] = useState("");
  const [email, setEmail] = useState("");
  const [phone, setPhone] = useState("");
  const [role, setRole] = useState("user");
  const [error, setError] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const pw = usePasswordConfirmation();

  const canSubmit = username && email && pw.isValid && !submitting;

  const handleSubmit = async (e) => {
    e.preventDefault();
    if (!canSubmit) return;
    setSubmitting(true);
    setError("");
    try {
      await onCreate(username, pw.password, role, email, phone || undefined);
      onCancel();
    } catch (err) {
      setError(err.message);
    }
    setSubmitting(false);
  };

  return (
    <Paper
      variant="outlined"
      component="form"
      onSubmit={handleSubmit}
      sx={{ p: 2, mb: 2, display: "flex", gap: 2, alignItems: "flex-start", flexWrap: "wrap" }}
    >
      <TextField size="small" label="Username" value={username} onChange={(e) => setUsername(e.target.value)} />
      <TextField size="small" label="Email" type="email" value={email} onChange={(e) => setEmail(e.target.value)} />
      <TextField
        size="small"
        label="Phone (optional)"
        value={phone}
        onChange={(e) => setPhone(e.target.value)}
      />
      <FormControl size="small" sx={{ minWidth: 140 }}>
        <InputLabel>Role</InputLabel>
        <Select label="Role" value={role} onChange={(e) => setRole(e.target.value)}>
          {ROLES.map((r) => (
            <MenuItem key={r} value={r}>
              {r}
            </MenuItem>
          ))}
        </Select>
      </FormControl>
      <PasswordConfirmFields pw={pw} size="small" />
      <Button type="submit" variant="contained" disabled={!canSubmit}>
        Create
      </Button>
      <Button variant="text" onClick={onCancel} disabled={submitting}>
        Cancel
      </Button>
      {error && (
        <Typography variant="body2" sx={{ color: "error.main", width: "100%" }}>
          {error}
        </Typography>
      )}
    </Paper>
  );
}

function ResetPasswordControl({ userId, onResetPassword }) {
  const [open, setOpen] = useState(false);
  const [error, setError] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const pw = usePasswordConfirmation();

  if (!open) {
    return (
      <Button size="small" onClick={() => setOpen(true)}>
        Reset Password
      </Button>
    );
  }

  const handleSubmit = async () => {
    if (!pw.isValid) return;
    setSubmitting(true);
    setError("");
    try {
      await onResetPassword(userId, pw.password);
      setOpen(false);
      pw.reset();
    } catch (err) {
      setError(err.message);
    }
    setSubmitting(false);
  };

  return (
    <Box sx={{ display: "flex", flexDirection: "column", gap: 1, alignItems: "flex-end" }}>
      <Box sx={{ display: "flex", gap: 1 }}>
        <PasswordConfirmFields pw={pw} size="small" />
      </Box>
      <Box sx={{ display: "flex", gap: 1 }}>
        <Button size="small" variant="contained" disabled={!pw.isValid || submitting} onClick={handleSubmit}>
          Save
        </Button>
        <Button
          size="small"
          onClick={() => {
            setOpen(false);
            pw.reset();
            setError("");
          }}
          disabled={submitting}
        >
          Cancel
        </Button>
      </Box>
      {error && (
        <Typography variant="body2" sx={{ color: "error.main" }}>
          {error}
        </Typography>
      )}
    </Box>
  );
}

// Admin-only tab: list/create/change-role/delete user accounts, and reset a
// user's password. There is no public sign-up anywhere in the app — this is
// the only way an account gets created, beyond the env-var-seeded bootstrap admin.
export function AdminTab({ currentUsername }) {
  const { users, loading, error, create, update, resetPassword, remove } = useUsers();
  const [showNewUserForm, setShowNewUserForm] = useState(false);
  // Shown inline (not with alert(), which blocks the whole page).
  const [actionError, setActionError] = useState("");

  const handleRoleChange = async (id, role) => {
    setActionError("");
    try {
      await update(id, { role });
    } catch (err) {
      setActionError("Failed to update role: " + err.message);
    }
  };

  const handleDelete = async (id, username) => {
    if (!window.confirm(`Delete user "${username}"? This cannot be undone.`)) return;
    setActionError("");
    try {
      await remove(id);
    } catch (err) {
      setActionError("Failed to delete user: " + err.message);
    }
  };

  return (
    <Box>
      <Typography variant="h6" sx={{ mt: 2, mb: 2 }}>
        User Management
      </Typography>

      {showNewUserForm ? (
        <NewUserForm onCreate={create} onCancel={() => setShowNewUserForm(false)} />
      ) : (
        <Button variant="outlined" sx={{ mb: 2 }} onClick={() => setShowNewUserForm(true)}>
          + Add User
        </Button>
      )}

      {[error, actionError].filter(Boolean).map((message) => (
        <Typography key={message} role="alert" variant="body2" sx={{ color: "error.main", mb: 2 }}>
          {message}
        </Typography>
      ))}

      <TableContainer component={Paper} sx={{ maxWidth: "100%", overflowX: "auto" }}>
        <Table sx={{ minWidth: 700 }}>
          <TableHead>
            <TableRow>
              <TableCell>Username</TableCell>
              <TableCell>Email</TableCell>
              <TableCell>Phone</TableCell>
              <TableCell>Role</TableCell>
              <TableCell>Created</TableCell>
              <TableCell align="right">Actions</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {users.map((u) => (
              <TableRow key={u.id} hover sx={zebraRowSx}>
                <TableCell>{u.username}</TableCell>
                <TableCell>{u.email}</TableCell>
                <TableCell>{u.phone || "-"}</TableCell>
                <TableCell>
                  <FormControl size="small" sx={{ minWidth: 120 }}>
                    <Select value={u.role} onChange={(e) => handleRoleChange(u.id, e.target.value)}>
                      {ROLES.map((r) => (
                        <MenuItem key={r} value={r}>
                          {r}
                        </MenuItem>
                      ))}
                    </Select>
                  </FormControl>
                </TableCell>
                <TableCell>{u.created_at || "-"}</TableCell>
                <TableCell align="right">
                  <Box sx={{ display: "flex", gap: 1, justifyContent: "flex-end" }}>
                    <ResetPasswordControl userId={u.id} onResetPassword={resetPassword} />
                    <Button
                      size="small"
                      color="error"
                      disabled={u.username === currentUsername}
                      title={u.username === currentUsername ? "You cannot delete your own account" : undefined}
                      onClick={() => handleDelete(u.id, u.username)}
                    >
                      Delete
                    </Button>
                  </Box>
                </TableCell>
              </TableRow>
            ))}
            {!loading && users.length === 0 && (
              <TableRow>
                <TableCell colSpan={6} align="center" sx={{ color: "text.secondary" }}>
                  No users yet.
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </TableContainer>
    </Box>
  );
}
