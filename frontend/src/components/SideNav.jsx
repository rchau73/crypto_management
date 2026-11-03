import { List, ListItemButton, ListItemText } from "@mui/material";

// The app's main menu: one entry per page the user may open. Rendered as a
// column beside the content on wide screens and inside a drawer on phones
// (App decides which); it only knows which page is current.
export function SideNav({ pages, current, onNavigate }) {
  return (
    <List component="nav" aria-label="Main" sx={{ p: 1 }}>
      {pages.map((p) => (
        <ListItemButton
          key={p.key}
          selected={p.key === current}
          aria-current={p.key === current ? "page" : undefined}
          onClick={() => onNavigate(p.key)}
          sx={{
            borderRadius: 2,
            mb: 0.5,
            minHeight: 48,
            "&.Mui-selected": { boxShadow: (t) => `inset 3px 0 0 ${t.palette.primary.main}` },
          }}
        >
          <ListItemText
            primary={p.label}
            secondary={p.hint}
            slotProps={{ primary: { sx: { fontWeight: 600 } }, secondary: { sx: { fontSize: 12 } } }}
          />
        </ListItemButton>
      ))}
    </List>
  );
}
