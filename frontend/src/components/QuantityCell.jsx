import { useState } from "react";
import { Box, Button, TextField, Typography } from "@mui/material";
import { correctWalletQuantity } from "../api/client";
import { canCorrectQuantity, parseNonNegative } from "../utils/portfolioTargets";

// The Quantity cell of one Portfolio Targets row:
// - new row: an editable starting quantity (saved with "Save All");
// - existing single-source row: read-only, with "Corrigir" to fix it right
//   away through its own endpoint (not part of "Save All");
// - existing multi-source row: read-only, marked "multi-fonte".
export function QuantityCell({ row, onChange, onCorrected }) {
  const [editing, setEditing] = useState(false);
  const [value, setValue] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  if (row.isNew) {
    return (
      <TextField
        size="small"
        type="number"
        value={row.current_quantity}
        onChange={(e) => onChange(e.target.value)}
        error={parseNonNegative(row.current_quantity) === null}
        sx={{ width: 110 }}
        slotProps={{ htmlInput: { step: "any", min: 0 } }}
      />
    );
  }

  const startEditing = () => {
    setValue(String(row.current_quantity ?? 0));
    setError("");
    setEditing(true);
  };

  const newQuantity = parseNonNegative(value);

  const submit = async () => {
    setSaving(true);
    setError("");
    try {
      await correctWalletQuantity({
        symbol: row.symbol,
        group_name: row.group_name || null,
        barca: row.barca || null,
        asset_class: row.asset_class,
        current_quantity: newQuantity,
      });
      setEditing(false);
      onCorrected(newQuantity);
    } catch (err) {
      setError(err.message);
    } finally {
      setSaving(false);
    }
  };

  if (editing) {
    return (
      <Box sx={{ display: "flex", flexDirection: "column", alignItems: "flex-end", gap: 0.5 }}>
        <Box sx={{ display: "flex", gap: 0.5 }}>
          <TextField
            size="small"
            type="number"
            autoFocus
            value={value}
            onChange={(e) => setValue(e.target.value)}
            error={newQuantity === null}
            sx={{ width: 90 }}
            slotProps={{ htmlInput: { step: "any", min: 0 } }}
          />
          <Button size="small" disabled={saving || newQuantity === null} onClick={submit}>
            Salvar
          </Button>
          <Button size="small" disabled={saving} onClick={() => setEditing(false)}>
            Cancelar
          </Button>
        </Box>
        {error && (
          <Typography variant="caption" sx={{ color: "error.main" }}>
            {error}
          </Typography>
        )}
      </Box>
    );
  }

  return (
    <Box sx={{ display: "flex", alignItems: "center", gap: 0.5, justifyContent: "flex-end" }}>
      <TextField
        size="small"
        type="number"
        value={row.current_quantity}
        disabled
        title="Managed by CSV import — use Corrigir to fix a wrong value"
        sx={{ width: 90 }}
      />
      {canCorrectQuantity(row) ? (
        <Button size="small" onClick={startEditing}>
          Corrigir
        </Button>
      ) : (
        <Typography
          variant="caption"
          title={`Soma de ${row.source_count} fontes — corrija cada fonte via importação de CSV`}
          sx={{ color: "text.disabled", px: 0.5 }}
        >
          multi-fonte
        </Typography>
      )}
    </Box>
  );
}
