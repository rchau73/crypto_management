import { Box, Tab, Tabs, Typography } from "@mui/material";

// A page title plus tabs between related screens (Targets: asset / BARCA;
// Settings: accounts / users). The selected tab is a URL param, so the
// browser's Back button and a reload keep it.
// `current` must be one of `tabs` (see selectedTab in utils/navigation).
export function SubTabs({ title, tabs, current, onChange }) {
  return (
    <Box sx={{ mb: 2 }}>
      <Typography variant="h5" component="h1" sx={{ fontWeight: 700, mb: 1 }}>
        {title}
      </Typography>
      <Tabs value={current} onChange={(_, key) => onChange(key)} aria-label={title}>
        {tabs.map((t) => (
          <Tab key={t.key} value={t.key} label={t.label} />
        ))}
      </Tabs>
    </Box>
  );
}
