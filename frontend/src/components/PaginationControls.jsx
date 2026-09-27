import { Box, Button, Typography } from "@mui/material";

// Prev/Page X of Y/Next controls, shared by every paginated table.
export function PaginationControls({ page, totalPages, onPageChange }) {
  return (
    <Box sx={{ display: "flex", justifyContent: "center", alignItems: "center", mt: 2 }}>
      <Button variant="outlined" size="small" onClick={() => onPageChange(page - 1)} disabled={page === 1} sx={{ mr: 1 }}>
        Prev
      </Button>
      <Typography sx={{ fontSize: 12 }}>
        Page {page} of {totalPages}
      </Typography>
      <Button variant="outlined" size="small" onClick={() => onPageChange(page + 1)} disabled={page === totalPages} sx={{ ml: 1 }}>
        Next
      </Button>
    </Box>
  );
}
