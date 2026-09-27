import { Typography } from "@mui/material";
import dayjs from "dayjs";

export function StatusLine({ lastUpdate, importStatus }) {
  if (!lastUpdate && !importStatus) return null;

  return (
    <Typography variant="body2" sx={{ color: "text.secondary", mb: 2 }}>
      {lastUpdate && <>Last update: {dayjs(lastUpdate).subtract(3, "hour").format("YYYY-MM-DD HH:mm:ss")}</>}
      {lastUpdate && importStatus && " · "}
      {importStatus}
    </Typography>
  );
}
