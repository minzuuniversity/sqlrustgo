---
name: gitea-pr-creation
description: Create PRs on self-hosted Gitea servers via REST API when gh CLI (GitHub-only) is unavailable
source: auto-skill
extracted_at: '2026-06-20T00:00:00.000Z'
---

## Create PR on Self-Hosted Gitea via REST API

When working with a self-hosted Gitea instance, the `gh` CLI targets GitHub by default and cannot create PRs. Use curl with the Gitea REST API directly.

### Prerequisites

- Branch is already pushed to origin (the remote pointing to the Gitea server)
- Branch name is known (e.g., `feature/my-fix`)
- Target base branch is known (e.g., `develop/v3.9.0`)
- Gitea server URL, credentials, and repo path are known

### Step 1: Verify Gitea API is reachable

```bash
curl -s --connect-timeout 5 "http://<user>:<pass>@<gitea-host>:3000/api/v1/repos/<owner>/<repo>" | head -5
```

Expected: JSON response with repo metadata. If this fails, the server is unreachable.

### Step 2: Create the PR

```bash
curl -s --connect-timeout 5 -X POST \
  "http://<user>:<pass>@<gitea-host>:3000/api/v1/repos/<owner>/<repo>/pulls" \
  -H "Content-Type: application/json" \
  -d '{
    "title": "<PR title>",
    "head": "<feature-branch>",
    "base": "<target-branch>",
    "body": "<markdown body>"
  }'
```

Key fields:
- `title`: PR title
- `head`: source branch (must be pushed to origin)
- `base`: target branch (e.g., `develop/v3.9.0`)
- `body`: Markdown PR description

Response includes: `number` (PR number), `html_url`, `mergeable`, `state` (open/closed).

### Step 3: Update PR body (if needed after new commits)

After making new commits, force-push the branch and the PR body updates automatically:

```bash
git push origin <feature-branch> --force-with-lease
```

Gitea's remote hook will output: `Visit the existing pull request: <url>`

### Step 4: Verify PR creation

```bash
curl -s --connect-timeout 5 \
  "http://<user>:<pass>@<gitea-host>:3000/api/v1/repos/<owner>/<repo>/pulls/<number>"
```

### Notes

- Authentication: Basic auth embedded in URL works (e.g., `http://user:pass@host:3000`). Alternatively, set up a Gitea personal access token and use `-H "Authorization: token <token>"`.
- The PR must be created from a branch that is already pushed to the origin remote.
- `--force-with-lease` is preferred over `--force` for safety — it only overwrites if the remote ref hasn't advanced beyond your expectation.
- Gitea API docs: http://<gitea-host>:3000/api/v1
