import { Box, Button, Tooltip, Typography } from "@mui/material";
import { canRemove } from "../utils/portfolioTargets";

// The Actions cell of one Portfolio Targets row. Every change is applied by
// "Save All", so a marked row says what Save will do and can be undone:
// - new row: Remove drops it;
// - saved row holding nothing: Remove marks it "Removed on Save";
// - saved row holding something: Remove is disabled (Corrigir to 0 first);
// - removed row (Show removed): Restore marks it "Restored on Save".
export function TargetRowActions({ row, onDrop, onMark }) {
  if (row.isNew) {
    return (
      <Button size="small" color="error" onClick={onDrop}>
        Remove
      </Button>
    );
  }

  if (row.removed) {
    return row.markedForRestore ? (
      <Pending label="Restored on Save" action="Keep removed" onClick={() => onMark("markedForRestore", false)} />
    ) : (
      <Box sx={{ display: "flex", alignItems: "center", justifyContent: "flex-end", gap: 1 }}>
        <Typography variant="caption" sx={{ color: "text.secondary" }}>
          Removed
        </Typography>
        <Button size="small" onClick={() => onMark("markedForRestore", true)}>
          Restore
        </Button>
      </Box>
    );
  }

  if (row.markedForRemoval) {
    return <Pending label="Removed on Save" action="Undo" onClick={() => onMark("markedForRemoval", false)} />;
  }

  if (!canRemove(row)) {
    return (
      <Tooltip title="It still holds a quantity. Set it to 0 with Corrigir first.">
        {/* A disabled button fires no events: the span carries the tooltip. */}
        <span>
          <Button size="small" color="error" disabled>
            Remove
          </Button>
        </span>
      </Tooltip>
    );
  }

  return (
    <Button size="small" color="error" onClick={() => onMark("markedForRemoval", true)}>
      Remove
    </Button>
  );
}

function Pending({ label, action, onClick }) {
  return (
    <Box sx={{ display: "flex", alignItems: "center", justifyContent: "flex-end", gap: 1 }}>
      <Typography variant="caption" sx={{ fontWeight: 700 }}>
        {label}
      </Typography>
      <Button size="small" onClick={onClick}>
        {action}
      </Button>
    </Box>
  );
}
