import { useState } from "react";

// Shared "type it twice" validation for any password-setting form (create
// user, reset password) — catches a typo before it becomes a locked-out
// account, without a password-strength policy nobody asked for.
export function usePasswordConfirmation() {
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");

  const matches = password === confirmPassword;
  const isValid = password.length > 0 && matches;

  const reset = () => {
    setPassword("");
    setConfirmPassword("");
  };

  return { password, setPassword, confirmPassword, setConfirmPassword, matches, isValid, reset };
}
