import { Box, Chip, FormControl, InputLabel, MenuItem, Select, Stack } from "@mui/material";

function FilterSelect({ label, value, onChange, options }) {
  return (
    <FormControl size="small" sx={{ minWidth: 160 }}>
      <InputLabel>{label}</InputLabel>
      <Select label={label} value={value} onChange={(e) => onChange(e.target.value)}>
        <MenuItem value="">All {label}</MenuItem>
        {options.map((opt) => (
          <MenuItem key={opt} value={opt}>
            {opt}
          </MenuItem>
        ))}
      </Select>
    </FormControl>
  );
}

// Asset/Group/BARCA filters shared by the per-asset tables. Active filters
// show as removable chips (rather than a hint sentence) so it's obvious at a
// glance what's narrowing the view, and easy to undo one at a time.
export function FiltersBar({
  assetFilter,
  onAssetFilterChange,
  assetOptions,
  groupFilter,
  onGroupFilterChange,
  groupOptions,
  barcaFilter,
  onBarcaFilterChange,
  barcaOptions,
}) {
  const activeFilters = [
    assetFilter && { key: "asset", label: `Asset: ${assetFilter}`, clear: () => onAssetFilterChange("") },
    groupFilter && { key: "group", label: `Group: ${groupFilter}`, clear: () => onGroupFilterChange("") },
    barcaFilter && { key: "barca", label: `BARCA: ${barcaFilter}`, clear: () => onBarcaFilterChange("") },
  ].filter(Boolean);

  const clearAll = () => {
    onAssetFilterChange("");
    onGroupFilterChange("");
    onBarcaFilterChange("");
  };

  return (
    <Box sx={{ display: "flex", flexDirection: "column", gap: 1.5, mb: 3 }}>
      <Stack direction="row" spacing={2} useFlexGap flexWrap="wrap">
        <FilterSelect label="Assets" value={assetFilter} onChange={onAssetFilterChange} options={assetOptions} />
        <FilterSelect label="Groups" value={groupFilter} onChange={onGroupFilterChange} options={groupOptions} />
        <FilterSelect label="BARCA" value={barcaFilter} onChange={onBarcaFilterChange} options={barcaOptions} />
      </Stack>
      {activeFilters.length > 0 && (
        <Stack direction="row" spacing={1} useFlexGap flexWrap="wrap" alignItems="center">
          {activeFilters.map((f) => (
            <Chip key={f.key} label={f.label} onDelete={f.clear} size="small" color="primary" variant="outlined" />
          ))}
          <Chip label="Clear all" onClick={clearAll} size="small" variant="filled" />
        </Stack>
      )}
    </Box>
  );
}
