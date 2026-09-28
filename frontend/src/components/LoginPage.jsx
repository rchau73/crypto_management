import { useState } from "react";
import { Alert, Box, Button, CircularProgress, Paper, TextField, Typography } from "@mui/material";
import { PasswordField } from "./PasswordField";

// Full-page swap shown whenever there's no active session — this app has no
// router, so "logged out" is just a different thing App.jsx renders.
export function LoginPage({ onLogin, error, loggingIn }) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");

  const handleSubmit = (e) => {
    e.preventDefault();
    if (!username || !password) return;
    onLogin(username, password);
  };

  return (
    <Box sx={{ minHeight: "100vh", display: "flex", alignItems: "center", justifyContent: "center", p: 2 }}>
      <Paper variant="outlined" sx={{ p: 4, width: "100%", maxWidth: 360 }}>
        <Typography variant="h5" sx={{ fontWeight: 700, mb: 3 }}>
          Wallet Allocations
        </Typography>
        <Box component="form" onSubmit={handleSubmit} sx={{ display: "flex", flexDirection: "column", gap: 2 }}>
          <TextField
            label="Username"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            autoFocus
            autoComplete="username"
            fullWidth
          />
          <PasswordField
            label="Password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            autoComplete="current-password"
            fullWidth
          />
          {error && <Alert severity="error">{error}</Alert>}
          <Button type="submit" variant="contained" disabled={loggingIn || !username || !password}>
            {loggingIn ? <CircularProgress size={20} color="inherit" /> : "Log In"}
          </Button>
        </Box>
      </Paper>
    </Box>
  );
}
