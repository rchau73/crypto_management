-- 0005_add_user_email_phone.sql
-- Add email (mandatory, unique) and phone (optional) to users, ahead of
-- future account-activation / email-verification / password-reset-by-link
-- features. Only the fields are added here — sending emails and verifying
-- tokens is separate, not-yet-built infrastructure.

-- SQLite can't add a UNIQUE/NOT NULL constraint to an existing column, so
-- the column is added with a DEFAULT (satisfying NOT NULL for existing
-- rows), backfilled with a placeholder derived from the already-unique
-- username, and only then does a unique index go on top.
ALTER TABLE users ADD COLUMN email TEXT NOT NULL DEFAULT '';
UPDATE users SET email = username || '@example.invalid' WHERE email = '';
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_email ON users(email);

ALTER TABLE users ADD COLUMN phone TEXT;
