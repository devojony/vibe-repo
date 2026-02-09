# Upgrade Guide: v0.3.0 → v0.4.0-mvp

**Version:** 0.4.0-mvp  
**Release Date:** 2026-02-07  
**Breaking Changes:** Yes

This guide explains how to upgrade from v0.3.0 to v0.4.0-mvp, which introduces a major architectural change from Provider-Centric to Repository-Centric model.

## ⚠️ Breaking Changes Summary

### Architecture Change

**Before (v0.3.0):** Provider-Centric
- Configure a Git provider (token, base URL)
- System auto-syncs ALL repositories from provider
- Archive unwanted repositories
- 4 database tables: `repo_providers`, `repositories`, `webhook_configs`, `workspaces`

**After (v0.4.0-mvp):** Repository-Centric
- Manually add only desired repositories
- Each repository contains its own provider configuration
- No auto-sync, no archiving needed
- 2 database tables: `repositories`, `workspaces`

### Database Schema Changes

**Removed Tables:**
- `repo_providers` - Provider configuration now stored per-repository
- `webhook_configs` - Webhook secrets now stored per-repository

**Modified Tables:**
- `repositories` - Added fields: `provider_type`, `provider_base_url`, `access_token`, `webhook_secret`, `webhook_status`
- `repositories` - Removed fields: `provider_id`, `polling_enabled`, `polling_interval_seconds`, `last_issue_poll_at`

### API Changes

**New Endpoints:**
- `POST /api/repositories` - Manually add a repository with provider configuration

**Removed Endpoints:**
- `POST /api/settings/providers` - No separate provider management
- `GET /api/settings/providers` - No separate provider management
- `PUT /api/settings/providers/:id` - No separate provider management
- `DELETE /api/settings/providers/:id` - No separate provider management
- `POST /api/settings/providers/:id/sync` - No auto-discovery
- `POST /api/settings/providers/:id/validate` - Validation happens during repository creation

**Modified Endpoints:**
- `GET /api/repositories` - Removed `provider_id` query parameter
- `GET /api/repositories/:id` - Response now includes `provider_type` and `provider_base_url` instead of `provider_id`

### Configuration Changes

**Before (v0.3.0):**
```bash
# .env file
GITHUB_TOKEN=ghp_xxxxxxxxxxxx
GITHUB_BASE_URL=https://api.github.com
WEBHOOK_SECRET=shared_secret_for_all_repos
```

**After (v0.4.0-mvp):**
```bash
# .env file
# No Git provider configuration needed
# Each repository has its own token and webhook secret
```

## 📋 Upgrade Steps

### Step 1: Backup Your Data

Before upgrading, backup your database:

```bash
# SQLite
cp ./data/vibe-repo/db/vibe-repo.db ./data/vibe-repo/db/vibe-repo.db.backup

# PostgreSQL
pg_dump vibe_repo > vibe_repo_backup.sql
```

### Step 2: Export Repository List

Export your current repository list for reference:

```bash
curl http://localhost:3000/api/repositories > repositories_backup.json
```

### Step 3: Stop the Application

```bash
# If running as systemd service
sudo systemctl stop vibe-repo

# If running manually
# Press Ctrl+C to stop
```

### Step 4: Update Code

```bash
cd /path/to/vibe-repo
git fetch origin
git checkout v0.4.0-mvp
```

### Step 5: Update Configuration

Remove Git provider configuration from `.env` file:

```bash
# Remove these lines from .env:
# GITHUB_TOKEN=...
# GITHUB_BASE_URL=...
# WEBHOOK_SECRET=...
```

### Step 6: Run Database Migration

The migration will automatically drop old tables and create new schema:

```bash
cd backend
cargo run
```

**What the migration does:**
1. Drops `webhook_configs` table
2. Drops `repo_providers` table
3. Drops old `repositories` table
4. Creates new `repositories` table with provider configuration fields
5. Preserves `workspaces`, `agents`, and `tasks` tables

### Step 7: Re-add Repositories

You'll need to manually re-add each repository with its provider configuration:

```bash
# For each repository you want to use:
curl -X POST http://localhost:3000/api/repositories \
  -H "Content-Type: application/json" \
  -d '{
    "provider_type": "github",
    "provider_base_url": "https://api.github.com",
    "access_token": "ghp_xxxxxxxxxxxx",
    "full_name": "owner/repo-name",
    "branch_name": "vibe-dev"
  }'
```

**Tip:** Create a script to batch-add multiple repositories:

```bash
#!/bin/bash
# add_repositories.sh

REPOS=(
  "owner/repo1"
  "owner/repo2"
  "owner/repo3"
)

for repo in "${REPOS[@]}"; do
  curl -X POST http://localhost:3000/api/repositories \
    -H "Content-Type: application/json" \
    -d "{
      \"provider_type\": \"github\",
      \"provider_base_url\": \"https://api.github.com\",
      \"access_token\": \"$GITHUB_TOKEN\",
      \"full_name\": \"$repo\",
      \"branch_name\": \"vibe-dev\"
    }"
  echo ""
done
```

### Step 8: Verify Installation

```bash
# Check health
curl http://localhost:3000/health

# List repositories
curl http://localhost:3000/api/repositories

# Check Swagger UI
open http://localhost:3000/swagger-ui
```

### Step 9: Update Webhooks (If Needed)

Webhooks are automatically created during repository addition. If you need to update them manually:

1. Go to each repository's webhook settings
2. Update the webhook secret (each repository now has a unique secret)
3. Verify webhook is receiving events

## 🔄 Rollback Procedure

If you encounter critical issues and need to rollback:

### Step 1: Stop the Application

```bash
sudo systemctl stop vibe-repo
```

### Step 2: Restore Database Backup

```bash
# SQLite
cp ./data/vibe-repo/db/vibe-repo.db.backup ./data/vibe-repo/db/vibe-repo.db

# PostgreSQL
psql vibe_repo < vibe_repo_backup.sql
```

### Step 3: Checkout Previous Version

```bash
git checkout v0.3.0
```

### Step 4: Restore Configuration

Restore your `.env` file with Git provider configuration:

```bash
# Add back to .env:
GITHUB_TOKEN=ghp_xxxxxxxxxxxx
GITHUB_BASE_URL=https://api.github.com
WEBHOOK_SECRET=shared_secret
```

### Step 5: Restart Application

```bash
cd backend
cargo run
```

## 📊 Migration Comparison

### Effort Comparison

| Task | v0.3.0 (100 repos, want 3) | v0.4.0-mvp (want 3) |
|------|---------------------------|---------------------|
| Configure provider | 1 API call | 0 API calls |
| Sync repositories | 1 API call (auto) | 0 API calls |
| Archive unwanted | 97 API calls | 0 API calls |
| Initialize desired | 3 API calls | 0 API calls (auto) |
| Add repositories | 0 API calls | 3 API calls |
| **Total** | **102 operations** | **3 operations** |

### Benefits

**Performance:**
- Database queries per operation: 2 → 1 (50% reduction)
- Response time improvement: ~5-10ms per operation

**User Experience:**
- 97% fewer operations for selective usage (3 vs 102)
- Explicit intent - only add what you want
- No need to archive unwanted repositories

**Security:**
- Per-repository token management (principle of least privilege)
- Unique webhook secret per repository
- Token rotation doesn't affect other repositories

**Flexibility:**
- Support for mixed providers (GitHub + Gitea + GitLab)
- Different tokens for different repositories
- Independent repository lifecycle

### Trade-offs

**Token Management:**
- Before: 1 token to manage
- After: N tokens to manage (one per repository)
- Mitigation: Use same token for multiple repos if desired

**No Auto-Discovery:**
- Before: Browse all available repositories
- After: Must know exact repository name
- Mitigation: Use provider's web UI to find repository names

## 🆘 Troubleshooting

### Issue: Migration fails with "table already exists"

**Solution:** Drop all tables manually and re-run migration:

```bash
# SQLite
sqlite3 ./data/vibe-repo/db/vibe-repo.db
> DROP TABLE IF EXISTS repositories;
> DROP TABLE IF EXISTS repo_providers;
> DROP TABLE IF EXISTS webhook_configs;
> .quit

# Then re-run: cargo run
```

### Issue: Cannot add repository - "Invalid access token"

**Solution:** Verify your token has all required permissions:

**GitHub:**
- `repo` - Full repository access
- `admin:repo_hook` - Webhook management

**Gitea:**
- `read:repository`, `write:repository`, `write:issue`, `write:pull_request`, `write:webhook`

**GitLab:**
- `api` - Full API access

### Issue: Webhook not receiving events

**Solution:**
1. Check webhook was created: `curl http://localhost:3000/api/repositories/:id`
2. Verify webhook status is "active"
3. Check provider's webhook settings page
4. Test webhook delivery manually
5. Check application logs for errors

### Issue: Lost repository data after upgrade

**Solution:** Restore from backup and follow upgrade steps carefully. The migration intentionally drops old tables because:
- No data migration needed (pre-1.0)
- Clean break simplifies implementation
- Users must explicitly re-add repositories

## 📚 Additional Resources

- **[MIGRATION.md](./MIGRATION.md)** - General migration guide
- **[docs/api/user-guide.md](./docs/api/user-guide.md)** - Updated user guide
- **[docs/api/api-reference.md](./docs/api/api-reference.md)** - Updated API reference
- **[docs/database/schema.md](./docs/database/schema.md)** - Updated database schema
- **[CHANGELOG.md](./CHANGELOG.md)** - Detailed changelog

## 💬 Getting Help

If you encounter issues during upgrade:

1. Check this guide and troubleshooting section
2. Review the [MIGRATION.md](./MIGRATION.md) for detailed changes
3. Check application logs for error messages
4. Open an issue on GitHub with the `upgrade` label
5. Include: version numbers, error messages, relevant logs

---

**Last Updated:** 2026-02-07  
**Version:** 0.4.0-mvp
