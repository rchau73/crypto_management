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
import { createPortfolio, renamePortfolio } from "../../api/client";
import { useCurrentPortfolio } from "../../hooks/useCurrentPortfolio";
import { usePortfolioList } from "../../hooks/usePortfolioList";
import { zebraRowSx } from "../../styles/tableStyles";

// Manager+ tab: your portfolios (separate wallets — yours and the ones you
// manage for others). Create one, or rename one you have `manage` access
// to. Nothing is shared between portfolios; a new one starts empty, with
// an optional copy of another's BARCA targets.
export function AccountsTab() {
  const current = useCurrentPortfolio();
  const { portfolios, reload } = usePortfolioList();

  return (
    <Box>
      <Typography variant="h6" sx={{ mt: 2, mb: 1 }}>
        Accounts
      </Typography>
      <Typography variant="body2" sx={{ color: "text.secondary", mb: 2, maxWidth: 640 }}>
        Each account is a separate wallet with its own positions, targets and history. Switch between them with
        the selector at the top. Import a CSV while an account is selected to fill it.
      </Typography>
      <AccountsTable portfolios={portfolios} currentId={current.id} onRenamed={() => reload()} />
      <NewAccountForm portfolios={portfolios} currentId={current.id} onCreated={(id) => reload(id)} />
    </Box>
  );
}

function AccountsTable({ portfolios, currentId, onRenamed }) {
  return (
    <TableContainer component={Paper} sx={{ maxWidth: 640, mb: 3 }}>
      <Table size="small">
        <TableHead>
          <TableRow>
            <TableCell>Name</TableCell>
            <TableCell>Access</TableCell>
            <TableCell align="right">Actions</TableCell>
          </TableRow>
        </TableHead>
        <TableBody>
          {portfolios.map((p) => (
            <AccountRow key={p.id} portfolio={p} isCurrent={p.id === currentId} onRenamed={onRenamed} />
          ))}
        </TableBody>
      </Table>
    </TableContainer>
  );
}

function AccountRow({ portfolio, isCurrent, onRenamed }) {
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState(portfolio.name);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  const save = async () => {
    setSaving(true);
    setError("");
    try {
      await renamePortfolio(portfolio.id, name.trim());
      setEditing(false);
      onRenamed();
    } catch (err) {
      setError(err.message);
    } finally {
      setSaving(false);
    }
  };

  return (
    <TableRow hover sx={zebraRowSx}>
      <TableCell>
        {editing ? (
          <TextField
            size="small"
            value={name}
            onChange={(e) => setName(e.target.value)}
            error={Boolean(error)}
            helperText={error}
            slotProps={{ htmlInput: { "aria-label": `New name for ${portfolio.name}`, maxLength: 60 } }}
          />
        ) : (
          <>
            {portfolio.name}
            {isCurrent && (
              <Typography component="span" variant="caption" sx={{ color: "text.secondary", ml: 1 }}>
                (showing)
              </Typography>
            )}
          </>
        )}
      </TableCell>
      <TableCell>{portfolio.access === "manage" ? "Manage" : "View only"}</TableCell>
      <TableCell align="right">
        {portfolio.access === "manage" &&
          (editing ? (
            <>
              <Button size="small" onClick={save} disabled={saving || name.trim() === ""}>
                Save
              </Button>
              <Button
                size="small"
                onClick={() => {
                  setEditing(false);
                  setName(portfolio.name);
                  setError("");
                }}
              >
                Cancel
              </Button>
            </>
          ) : (
            <Button size="small" onClick={() => setEditing(true)}>
              Rename
            </Button>
          ))}
      </TableCell>
    </TableRow>
  );
}

const NO_COPY = "";

function NewAccountForm({ portfolios, currentId, onCreated }) {
  const [name, setName] = useState("");
  const [copyFrom, setCopyFrom] = useState(currentId);
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState("");

  const create = async () => {
    setCreating(true);
    setError("");
    try {
      const created = await createPortfolio(name.trim(), copyFrom === NO_COPY ? null : copyFrom);
      setName("");
      onCreated(created.id);
    } catch (err) {
      setError(err.message);
    } finally {
      setCreating(false);
    }
  };

  return (
    <Box sx={{ maxWidth: 640 }}>
      <Typography variant="subtitle1" sx={{ mb: 1, fontWeight: 600 }}>
        New account
      </Typography>
      <Box sx={{ display: "flex", gap: 2, flexWrap: "wrap", alignItems: "flex-start" }}>
        <TextField
          size="small"
          label="Name"
          value={name}
          onChange={(e) => setName(e.target.value)}
          error={Boolean(error)}
          helperText={error}
          slotProps={{ htmlInput: { maxLength: 60 } }}
        />
        <FormControl size="small" sx={{ minWidth: 220 }}>
          <InputLabel id="copy-barca-label">Start BARCA targets from</InputLabel>
          <Select
            labelId="copy-barca-label"
            label="Start BARCA targets from"
            value={copyFrom}
            onChange={(e) => setCopyFrom(e.target.value)}
          >
            {portfolios.map((p) => (
              <MenuItem key={p.id} value={p.id}>
                A copy of {p.name}
              </MenuItem>
            ))}
            <MenuItem value={NO_COPY}>Empty</MenuItem>
          </Select>
        </FormControl>
        <Button variant="contained" onClick={create} disabled={creating || name.trim() === ""}>
          Create
        </Button>
      </Box>
      <Typography variant="caption" sx={{ color: "text.secondary", display: "block", mt: 1 }}>
        The new account opens right away, empty. Its BARCA targets are a copy: editing them never changes the
        original.
      </Typography>
    </Box>
  );
}
