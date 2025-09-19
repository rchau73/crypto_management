import { Alert, Box, Button, CircularProgress, Typography } from "@mui/material";
import dayjs from "dayjs";

import { formatNumber } from "../utils/format";

export function SummaryHeader({
  totalWalletValue,
  totalWalletValueBRL,
  usdtBrlRate,
  loading,
  lastUpdate,
  onRefresh,
  error,
  onDismissError,
}) {
  return (
    <Box sx={{ mb: 3 }}>
      <Typography variant="h4" gutterBottom>
        Wallet Allocations (Live)
      </Typography>

      <Box sx={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: 3, mb: 2 }}>
        <Typography variant="h6">
          Total Wallet Value: <span style={{ color: "#00C49F" }}>${formatNumber(totalWalletValue)}</span>
        </Typography>
        <Typography variant="h6">
          {usdtBrlRate
            ? (
              <>
                Total in R$: <span style={{ color: "#FFBB28" }}>R$ {formatNumber(totalWalletValueBRL)}</span>
              </>
            )
            : (
              <span style={{ color: "#FFBB28" }}>R$ --</span>
            )}
        </Typography>
      </Box>

      <Box sx={{ display: "flex", alignItems: "center", gap: 2 }}>
        <Button
          variant="contained"
          color="primary"
          onClick={onRefresh}
          disabled={loading}
          sx={{ mb: 1 }}
        >
          {loading ? <CircularProgress size={24} /> : "Update Prices & Show Distribution"}
        </Button>
        {lastUpdate && (
          <Typography variant="body2" sx={{ color: "#aaa" }}>
            Last update: {dayjs(lastUpdate).format("YYYY-MM-DD HH:mm:ss")}
          </Typography>
        )}
      </Box>

      {error && (
        <Alert severity="error" onClose={onDismissError} sx={{ mt: 2 }}>
          {error}
        </Alert>
      )}
    </Box>
  );
}
