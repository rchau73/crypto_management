import { Typography } from "@mui/material";
import { formatBrt } from "../utils/time";

// The exact time of the prices (the header shows their age), plus the
// outcome of the last import/export or a failed load.
export function StatusLine({ pricesAsOf, importStatus, importError }) {
  if (!pricesAsOf && !importStatus && !importError) return null;

  return (
    <>
      <Typography variant="body2" sx={{ color: "text.secondary", mb: importError ? 0.5 : 2 }}>
        {pricesAsOf && <>Prices as of {formatBrt(pricesAsOf)} (BRT)</>}
        {pricesAsOf && importStatus && " · "}
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
