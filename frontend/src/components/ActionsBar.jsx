import { useRef, useState } from "react";
import {
  Button,
  CircularProgress,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  Stack,
} from "@mui/material";
import { fetchRecordedActivity } from "../api/client";
import { useCurrentPortfolio } from "../hooks/useCurrentPortfolio";
import { staleImportWarning } from "../utils/importGuard";
import { formatBrt } from "../utils/time";

// "Import CSV into <account>" / "Export Wallet CSV" (prices refresh by
// themselves, so there is no Update Prices button any more). Import names
// the account it writes to, so a relative's file can't land in the wrong
// wallet unnoticed. Export
// downloads the wallet in the same format Import reads, for reports outside
// the app. Status text (last update, import result) lives in StatusLine, not
// here, so this stays a plain toolbar-friendly button row. Importing a CSV
// (and exporting) is a Manager+ action on the backend, so it's hidden (not just disabled)
// for a plain User — hiding it makes the permission boundary visible instead
// of showing an action that would just 403.
//
// The visible button just opens a native file picker (a hidden <input
// type="file">); `onImport` is called with the chosen File once the user
// picks one, and the file's actual bytes are uploaded — no assumption that
// the backend can read a path from its own filesystem, which doesn't hold
// once the backend is a container/serverless deploy you can't shell into.
//
// A file saved before the latest trade or move recorded in the app would put the
// old balances back (see utils/importGuard), so that one asks first.
export function ActionsBar({ accountName, importing, onImport, exporting, onExport, canImport }) {
  const { id: portfolioId } = useCurrentPortfolio();
  const fileInputRef = useRef(null);
  const [checking, setChecking] = useState(false);
  // { file, latestAt } while the "older file" warning is open.
  const [stale, setStale] = useState(null);

  const handleFileChosen = async (e) => {
    const file = e.target.files?.[0];
    e.target.value = ""; // allow re-selecting the same filename next time
    if (!file) return;
    setChecking(true);
    const activity = await fetchRecordedActivity(portfolioId).catch(() => null);
    setChecking(false);
    const warning = staleImportWarning(file, activity);
    if (warning) setStale({ file, ...warning });
    else onImport(file);
  };

  const importAnyway = () => {
    onImport(stale.file);
    setStale(null);
  };
  const exportInstead = () => {
    setStale(null);
    onExport();
  };

  if (!canImport) return null;

  return (
    <Stack direction="row" spacing={1.5}>
      <input ref={fileInputRef} type="file" accept=".csv,text/csv" hidden onChange={handleFileChosen} />
      <Button
        variant="outlined"
        color="secondary"
        onClick={() => fileInputRef.current?.click()}
        disabled={importing || checking}
      >
        {importing || checking ? <CircularProgress size={20} color="inherit" /> : `Import CSV into ${accountName}`}
      </Button>
      <Button variant="outlined" color="secondary" onClick={onExport} disabled={exporting}>
        {exporting ? <CircularProgress size={20} color="inherit" /> : "Export Wallet CSV"}
      </Button>
      {stale && (
        <Dialog open onClose={() => setStale(null)} maxWidth="sm" aria-labelledby="stale-import-title">
          <DialogTitle id="stale-import-title" sx={{ fontWeight: 700 }}>
            This file may undo trades or moves recorded in {accountName}
          </DialogTitle>
          <DialogContent>
            <DialogContentText sx={{ mb: 1.5 }}>
              {stale.latestAt
                ? `The latest trade or move was recorded on ${formatBrt(stale.latestAt)} (BRT), after ${stale.file.name} was saved.`
                : `Couldn't check the trades and moves recorded in ${accountName}. If any came after ${stale.file.name} was saved, read this first.`}
            </DialogContentText>
            <DialogContentText>
              Importing sets every exchange the file lists back to the file's quantities. A balance recorded on an
              exchange or BARCA the file doesn't list is kept, so it can be counted twice. Export the current wallet and edit
              that file instead.
            </DialogContentText>
          </DialogContent>
          <DialogActions sx={{ px: 3, pb: 2 }}>
            <Button onClick={() => setStale(null)}>Cancel</Button>
            <Button onClick={importAnyway}>Import anyway into {accountName}</Button>
            <Button variant="contained" onClick={exportInstead}>
              Export current CSV
            </Button>
          </DialogActions>
        </Dialog>
      )}
    </Stack>
  );
}
