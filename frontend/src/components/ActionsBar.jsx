import { Button, CircularProgress, Stack } from "@mui/material";

// "Update Prices" / "Import Wallet CSV" — the two actions that mutate app
// state. Status text (last update, import result) lives in StatusLine, not
// here, so this stays a plain toolbar-friendly button row.
export function ActionsBar({ loading, onRefresh, importing, onImport }) {
  return (
    <Stack direction="row" spacing={1.5}>
      <Button variant="contained" onClick={onRefresh} disabled={loading}>
        {loading ? <CircularProgress size={20} color="inherit" /> : "Update Prices"}
      </Button>
      <Button variant="outlined" color="secondary" onClick={onImport} disabled={importing}>
        {importing ? <CircularProgress size={20} color="inherit" /> : "Import Wallet CSV"}
      </Button>
    </Stack>
  );
}
