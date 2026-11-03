import { Typography } from "@mui/material";
import { formatBrt } from "../utils/time";

export function StatusLine({ lastUpdate, importStatus, importError }) {
  if (!lastUpdate && !importStatus && !importError) return null;

  return (
    <>
      <Typography variant="body2" sx={{ color: "text.secondary", mb: importError ? 0.5 : 2 }}>
        {lastUpdate && <>Last update: {formatBrt(lastUpdate)}</>}
        {lastUpdate && importStatus && " · "}
        {importStatus}
      </Typography>
      {importError && (
        <Typography variant="body2" sx={{ color: "error.main", mb: 2 }}>
          {importError}
        </Typography>
      )}
    </>
  );
}
