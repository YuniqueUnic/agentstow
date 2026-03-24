Feature: Provider Workspace Fixtures

  Scenario: Codex provider renders a native Codex config
    Given the provider workspace "codex"
    When I render artifact "provider_config" to "rendered/.codex/config.toml" with profile "base"
    Then the command succeeds
    And the toml file "rendered/.codex/config.toml" array field "project_doc_fallback_filenames" contains "AGENTS.md"
    And the toml file "rendered/.codex/config.toml" array field "project_doc_fallback_filenames" contains "TEAM_GUIDE.md"
    And the toml file "rendered/.codex/config.toml" has field "shell_environment_policy.set.AGENTSTOW_PROVIDER" equal to "codex"
    And the toml file "rendered/.codex/config.toml" has field "mcp_servers.filesystem.command" equal to "npx"
    And the toml file "rendered/.codex/config.toml" array field "mcp_servers.filesystem.env_vars" contains "GITHUB_TOKEN"
    And the toml file "rendered/.codex/config.toml" has field "mcp_servers.tavily.bearer_token_env_var" equal to "TAVILY_API_KEY"
    And the toml file "rendered/.codex/config.toml" has field "mcp_servers.tavily.env_http_headers.X-Tenant" equal to "AGENTSTOW_TENANT"

  Scenario: Codex provider renders instructions, skills, README, and env outputs
    Given the provider workspace "codex"
    When I render artifact "workspace_bundle" to "rendered/workspace" with profile "base"
    Then the command succeeds
    And the file "rendered/workspace/AGENTS.md" contains "codex-provider-fixture"
    And the file "rendered/workspace/AGENTS.md" contains "README.md` is documentation content, not a default instruction file"
    And the file "rendered/workspace/AGENTS.md" contains "## Reference"
    And the file "rendered/workspace/services/payments/AGENTS.md" contains "payments"
    And the file "rendered/workspace/TEAM_GUIDE.md" contains "TEAM_GUIDE.md"
    And the file "rendered/workspace/README.md" contains "mcpServers:"
    And the file "rendered/workspace/.env.rendered" contains "DUPLICATE_ENV=from-inline-codex"
    And the directory "rendered/workspace/.agents/skills" contains file "repo-guide/SKILL.md"
    And the directory "rendered/workspace/.agents/skills" contains file "repo-guide/references/mcp.md"
    And the directory "rendered/workspace/.agents/skills" contains file "repo-guide/assets/checklist.txt"
    When I run env emit with shell "bash"
    Then the command succeeds
    And stdout contains "export DUPLICATE_ENV='from-inline-codex'"
    And stdout contains "export PROVIDER_NAME='codex'"

  Scenario: Codex provider installs mixed targets and reports healthy status
    Given the provider workspace "codex"
    When I run link apply
    Then the command succeeds
    And the state database exists
    And the path "demo-targets/codex-project/AGENTS.md" is a symlink
    And the file "demo-targets/codex-project/.codex/config.toml" contains "bearer_token_env_var = \"TAVILY_API_KEY\""
    And the path "demo-targets/bundles/codex-workspace/README.md" exists
    And the file "demo-targets/docs/README.md" contains "codex-provider-fixture"
    And link status json marks target path suffix "demo-targets/codex-project/.codex/config.toml" with method "copy" and healthy "true"
    And link status json marks target path suffix "demo-targets/bundles/codex-workspace" with method "copy" and healthy "true"

  Scenario: Codex provider repair heals a drifted copy target
    Given the provider workspace "codex"
    And link has been applied to the workspace
    And I write the exact text "tampered" to "demo-targets/codex-project/.codex/config.toml"
    When I run link status as json
    Then the command succeeds
    And link status json marks target path suffix "demo-targets/codex-project/.codex/config.toml" with method "copy" and healthy "false"
    When I run link repair with force
    Then the command succeeds
    And the file "demo-targets/codex-project/.codex/config.toml" contains "project_doc_fallback_filenames = [\"AGENTS.md\", \"TEAM_GUIDE.md\"]"
    And link status json marks target path suffix "demo-targets/codex-project/.codex/config.toml" with method "copy" and healthy "true"

  Scenario: Claude provider renders native project MCP config
    Given the provider workspace "claude"
    When I render artifact "provider_config" to "rendered/.mcp.json" with profile "base"
    Then the command succeeds
    And the json file "rendered/.mcp.json" has field "mcpServers.filesystem.type" equal to "stdio"
    And the json file "rendered/.mcp.json" has field "mcpServers.release_notes.type" equal to "http"
    And the json file "rendered/.mcp.json" has field "mcpServers.release_notes.url" equal to "https://mcp.example.com/claude"
    And the json file "rendered/.mcp.json" has field "mcpServers.release_notes.oauth.clientId" equal to "claude-client"
    And the json file "rendered/.mcp.json" has field "mcpServers.release_notes.oauth.callbackPort" equal to "4317"
    And the json file "rendered/.mcp.json" has field "mcpServers.release_notes.oauth.authServerMetadataUrl" equal to "https://auth.example.com/.well-known/openid-configuration"
    And the json file "rendered/.mcp.json" has field "mcpServers.release_notes.headers.Authorization" equal to "Bearer ${CLAUDE_API_TOKEN}"

  Scenario: Claude provider renders memory, README, rules, skills, and env outputs
    Given the provider workspace "claude"
    When I render artifact "workspace_bundle" to "rendered/workspace" with profile "base"
    Then the command succeeds
    And the file "rendered/workspace/CLAUDE.md" contains "@README.md"
    And the file "rendered/workspace/CLAUDE.md" contains "@.claude/rules/testing.md"
    And the file "rendered/workspace/CLAUDE.md" contains "claude-provider-fixture"
    And the file "rendered/workspace/README.md" contains "claude-provider-fixture"
    And the file "rendered/workspace/.env.rendered" contains "DUPLICATE_ENV=from-inline-claude"
    And the path "rendered/workspace/.claude/settings.json" exists
    And the path "rendered/workspace/.claude/rules/testing.md" exists
    And the directory "rendered/workspace/.claude/skills" contains file "release-notes/SKILL.md"
    And the directory "rendered/workspace/.claude/skills" contains file "release-notes/template.md"
    And the directory "rendered/workspace/.claude/skills" contains file "release-notes/examples/sample.md"
    And the directory "rendered/workspace/.claude/skills" contains file "release-notes/scripts/validate.sh"
    When I run env emit with shell "bash"
    Then the command succeeds
    And stdout contains "export DUPLICATE_ENV='from-inline-claude'"
    And stdout contains "export PROVIDER_NAME='claude'"

  Scenario: Claude provider installs mixed targets and reports healthy status
    Given the provider workspace "claude"
    When I run link apply
    Then the command succeeds
    And the state database exists
    And the path "demo-targets/claude-linked/CLAUDE.md" is a symlink
    And the file "demo-targets/claude-config/.mcp.json" contains "\"Authorization\": \"Bearer ${CLAUDE_API_TOKEN}\""
    And the path "demo-targets/claude-project/README.md" exists
    And the file "demo-targets/docs/claude/README.md" contains "claude-provider-fixture"
    And link status json marks target path suffix "demo-targets/claude-config/.mcp.json" with method "copy" and healthy "true"
    And link status json marks target path suffix "demo-targets/claude-project" with method "copy" and healthy "true"

  Scenario: Claude provider repair heals a drifted copy target
    Given the provider workspace "claude"
    And link has been applied to the workspace
    And I write the exact text "tampered" to "demo-targets/claude-config/.mcp.json"
    When I run link status as json
    Then the command succeeds
    And link status json marks target path suffix "demo-targets/claude-config/.mcp.json" with method "copy" and healthy "false"
    When I run link repair with force
    Then the command succeeds
    And the file "demo-targets/claude-config/.mcp.json" contains "\"clientId\": \"claude-client\""
    And link status json marks target path suffix "demo-targets/claude-config/.mcp.json" with method "copy" and healthy "true"

  Scenario: Gemini provider renders native Gemini settings
    Given the provider workspace "gemini"
    When I render artifact "provider_config" to "rendered/.gemini/settings.json" with profile "base"
    Then the command succeeds
    And the json file "rendered/.gemini/settings.json" has field "context.loadFromIncludeDirectories" equal to "true"
    And the json file "rendered/.gemini/settings.json" has field "mcpServers.filesystem.command" equal to "npx"
    And the json file "rendered/.gemini/settings.json" has field "mcpServers.cloud_tools.httpUrl" equal to "https://mcp.example.com/gemini"
    And the json file "rendered/.gemini/settings.json" has field "mcpServers.cloud_tools.trust" equal to "false"
    And the json file "rendered/.gemini/settings.json" array field "mcpServers.cloud_tools.includeTools" contains "search"
    And the json file "rendered/.gemini/settings.json" array field "mcpServers.cloud_tools.excludeTools" contains "delete"
    And the json file "rendered/.gemini/settings.json" has field "mcpServers.cloud_tools.authProviderType" equal to "google_credentials"
    And the json file "rendered/.gemini/settings.json" has field "mcpServers.cloud_tools.targetAudience" equal to "https://mcp.example.com/gemini"
    And the json file "rendered/.gemini/settings.json" has field "mcpServers.cloud_tools.targetServiceAccount" equal to "svc@example.iam.gserviceaccount.com"
    And the json file "rendered/.gemini/settings.json" has field "mcpServers.cloud_tools.oauth.clientId" equal to "gemini-client"

  Scenario: Gemini provider renders memory, commands, extensions, README, and env outputs
    Given the provider workspace "gemini"
    When I render artifact "workspace_bundle" to "rendered/workspace" with profile "base"
    Then the command succeeds
    And the file "rendered/workspace/GEMINI.md" contains "gemini-provider-fixture"
    And the file "rendered/workspace/README.md" contains "gemini-provider-fixture"
    And the file "rendered/workspace/.gemini/.env" contains "DUPLICATE_ENV=from-inline-gemini"
    And the file "rendered/workspace/.gemini/commands/review-readme.toml" contains "@{README.md}"
    And the path "rendered/workspace/.gemini/commands/git/commit.toml" exists
    And the file "rendered/workspace/.gemini/extensions/cloud-tools/gemini-extension.json" contains "\"contextFileName\""
    And the file "rendered/workspace/.gemini/extensions/cloud-tools/GEMINI.md" contains "cloud-tools"
    And the path "rendered/workspace/.gemini/extensions/cloud-tools/commands/deploy.toml" exists
    When I run env emit with shell "bash"
    Then the command succeeds
    And stdout contains "export DUPLICATE_ENV='from-inline-gemini'"
    And stdout contains "export PROVIDER_NAME='gemini'"

  Scenario: Gemini provider installs mixed targets and reports healthy status
    Given the provider workspace "gemini"
    When I run link apply
    Then the command succeeds
    And the state database exists
    And the path "demo-targets/gemini-linked/GEMINI.md" is a symlink
    And the file "demo-targets/gemini-config/.gemini/settings.json" contains "\"authProviderType\": \"google_credentials\""
    And the path "demo-targets/gemini-project/README.md" exists
    And the file "demo-targets/docs/gemini/README.md" contains "gemini-provider-fixture"
    And link status json marks target path suffix "demo-targets/gemini-config/.gemini/settings.json" with method "copy" and healthy "true"
    And link status json marks target path suffix "demo-targets/gemini-project" with method "copy" and healthy "true"

  Scenario: Gemini provider repair heals a drifted copy target
    Given the provider workspace "gemini"
    And link has been applied to the workspace
    And I write the exact text "tampered" to "demo-targets/gemini-config/.gemini/settings.json"
    When I run link status as json
    Then the command succeeds
    And link status json marks target path suffix "demo-targets/gemini-config/.gemini/settings.json" with method "copy" and healthy "false"
    When I run link repair with force
    Then the command succeeds
    And the file "demo-targets/gemini-config/.gemini/settings.json" contains "\"targetServiceAccount\": \"svc@example.iam.gserviceaccount.com\""
    And link status json marks target path suffix "demo-targets/gemini-config/.gemini/settings.json" with method "copy" and healthy "true"
