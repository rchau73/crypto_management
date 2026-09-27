import { Box, Typography } from "@mui/material";

export function EmptyState({ title, description }) {
  return (
    <Box sx={{ textAlign: "center", py: 10, color: "text.secondary" }}>
      <Typography variant="h6" sx={{ mb: 1 }}>
        {title}
      </Typography>
      <Typography variant="body2">{description}</Typography>
    </Box>
  );
}
