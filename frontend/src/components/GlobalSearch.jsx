import { useEffect, useMemo, useRef, useState } from "react";
import { Autocomplete, Box, TextField, Typography } from "@mui/material";
import { buildSearchIndex, searchIndex } from "../utils/search";

const isMac = typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform || "");
const SHORTCUT_LABEL = isMac ? "⌘K" : "Ctrl+K";

// The header's "find anything" box: type part of an asset, BARCA, group or
// page name and pick a result to go there. Ctrl/⌘+K focuses it from
// anywhere, the way most modern apps do.
export function GlobalSearch({ pages, allocations, onGo }) {
  const [input, setInput] = useState("");
  const inputRef = useRef(null);
  const index = useMemo(() => buildSearchIndex(pages, allocations), [pages, allocations]);
  const results = useMemo(() => searchIndex(index, input), [index, input]);

  useEffect(() => {
    const onKey = (e) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        inputRef.current?.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  return (
    <Autocomplete
      sx={{ flex: "1 1 240px", maxWidth: 420, minWidth: 200 }}
      size="small"
      options={results}
      filterOptions={(x) => x}
      getOptionLabel={(o) => (typeof o === "string" ? o : o.label)}
      isOptionEqualToValue={(a, b) => a.label === b.label && a.kind === b.kind}
      inputValue={input}
      onInputChange={(_, value, reason) => {
        if (reason !== "reset") setInput(value);
      }}
      value={null}
      onChange={(_, option) => {
        if (option && typeof option !== "string") {
          onGo(option.page, option.params);
          setInput("");
          inputRef.current?.blur();
        }
      }}
      noOptionsText={input.trim() ? `Nothing matches “${input.trim()}”` : "Type an asset, BARCA, group or page"}
      autoHighlight
      clearOnBlur={false}
      renderOption={(props, option) => {
        const { key, ...rest } = props;
        return (
          <Box component="li" key={key} {...rest} sx={{ display: "flex", justifyContent: "space-between", gap: 2 }}>
            <Typography component="span" sx={{ fontWeight: 600 }}>
              {option.label}
            </Typography>
            <Typography component="span" variant="body2" sx={{ color: "text.secondary" }}>
              {option.kind}
            </Typography>
          </Box>
        );
      }}
      renderInput={(params) => (
        <TextField
          {...params}
          inputRef={inputRef}
          placeholder={`Find an asset, BARCA or page (${SHORTCUT_LABEL})`}
          slotProps={{ htmlInput: { ...params.inputProps, "aria-label": "Find an asset, BARCA, group or page" } }}
        />
      )}
    />
  );
}
