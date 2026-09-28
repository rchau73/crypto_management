import { Typography } from "@mui/material";
import dayjs from "dayjs";

export function StatusLine({ lastUpdate, importStatus, importError }) {
  if (!lastUpdate && !importStatus && !importError) return null;

  return (
    <>
      <Typography variant="body2" sx={{ color: "text.secondary", mb: importError ? 0.5 : 2 }}>
        {lastUpdate && <>Last update: {dayjs(lastUpdate).subtract(3, "hour").format("YYYY-MM-DD HH:mm:ss")}</>}
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
