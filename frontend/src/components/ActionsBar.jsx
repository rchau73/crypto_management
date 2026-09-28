import { useRef } from "react";
import { Button, CircularProgress, Stack } from "@mui/material";

// "Update Prices" / "Import Wallet CSV" — the two actions that mutate app
// state. Status text (last update, import result) lives in StatusLine, not
// here, so this stays a plain toolbar-friendly button row. Importing a CSV
// is a Manager+ action on the backend, so it's hidden (not just disabled)
// for a plain User — hiding it makes the permission boundary visible instead
// of showing an action that would just 403.
//
// The visible button just opens a native file picker (a hidden <input
// type="file">); `onImport` is called with the chosen File once the user
// picks one, and the file's actual bytes are uploaded — no assumption that
// the backend can read a path from its own filesystem, which doesn't hold
// once the backend is a container/serverless deploy you can't shell into.
export function ActionsBar({ loading, onRefresh, importing, onImport, canImport }) {
  const fileInputRef = useRef(null);

  const handleFileChosen = (e) => {
    const file = e.target.files?.[0];
    e.target.value = ""; // allow re-selecting the same filename next time
    if (file) onImport(file);
  };

  return (
    <Stack direction="row" spacing={1.5}>
      <Button variant="contained" onClick={onRefresh} disabled={loading}>
        {loading ? <CircularProgress size={20} color="inherit" /> : "Update Prices"}
      </Button>
      {canImport && (
        <>
          <input
            ref={fileInputRef}
            type="file"
            accept=".csv,text/csv"
            hidden
            onChange={handleFileChosen}
          />
          <Button
            variant="outlined"
            color="secondary"
            onClick={() => fileInputRef.current?.click()}
            disabled={importing}
          >
            {importing ? <CircularProgress size={20} color="inherit" /> : "Import Wallet CSV"}
          </Button>
        </>
      )}
    </Stack>
  );
}
