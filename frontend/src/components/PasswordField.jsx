import { useState } from "react";
import { Button, InputAdornment, TextField } from "@mui/material";

// A password TextField with a "Show"/"Hide" toggle. Plain text rather than
// an eye icon, deliberately — this app doesn't otherwise depend on an icon
// library, and a text toggle is just as clear.
export function PasswordField({ label = "Password", value, onChange, autoComplete, ...props }) {
  const [visible, setVisible] = useState(false);

  return (
    <TextField
      label={label}
      type={visible ? "text" : "password"}
      value={value}
      onChange={onChange}
      autoComplete={autoComplete}
      slotProps={{
        input: {
          endAdornment: (
            <InputAdornment position="end">
              <Button
                size="small"
                onClick={() => setVisible((v) => !v)}
                sx={{ minWidth: 0, px: 1 }}
                aria-label={visible ? "Hide password" : "Show password"}
                tabIndex={-1}
              >
                {visible ? "Hide" : "Show"}
              </Button>
            </InputAdornment>
          ),
        },
      }}
      {...props}
    />
  );
}
