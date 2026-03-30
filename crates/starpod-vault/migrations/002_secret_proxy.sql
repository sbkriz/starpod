ALTER TABLE vault_entries ADD COLUMN is_secret INTEGER NOT NULL DEFAULT 1;
ALTER TABLE vault_entries ADD COLUMN allowed_hosts TEXT;
