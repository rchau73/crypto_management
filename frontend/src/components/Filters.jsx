import { Box, TextField, MenuItem } from "@mui/material";

export function Filters({
  assetOptions,
  groupOptions,
  barcaOptions,
  assetFilter,
  groupFilter,
  barcaFilter,
  onAssetFilterChange,
  onGroupFilterChange,
  onBarcaFilterChange,
}) {
  return (
    <Box sx={{ display: "flex", flexWrap: "wrap", gap: 2, mb: 2 }}>
      <TextField
        select
        size="small"
        label="Asset"
        value={assetFilter}
        onChange={(event) => onAssetFilterChange(event.target.value)}
        sx={{ minWidth: 160 }}
      >
        <MenuItem value="">All Assets</MenuItem>
        {assetOptions.map((option) => (
          <MenuItem key={option} value={option}>
            {option}
          </MenuItem>
        ))}
      </TextField>

      <TextField
        select
        size="small"
        label="Group"
        value={groupFilter}
        onChange={(event) => onGroupFilterChange(event.target.value)}
        sx={{ minWidth: 160 }}
      >
        <MenuItem value="">All Groups</MenuItem>
        {groupOptions.map((option) => (
          <MenuItem key={option} value={option}>
            {option}
          </MenuItem>
        ))}
      </TextField>

      <TextField
        select
        size="small"
        label="BARCA"
        value={barcaFilter}
        onChange={(event) => onBarcaFilterChange(event.target.value)}
        sx={{ minWidth: 160 }}
      >
        <MenuItem value="">All BARCA</MenuItem>
        {barcaOptions.map((option) => (
          <MenuItem key={option} value={option}>
            {option}
          </MenuItem>
        ))}
      </TextField>
    </Box>
  );
}
