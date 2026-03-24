Feature: Real Example Workspace

  Scenario: real-example render dry-run includes generic and provider snippets
    Given the workspace fixture "real-example"
    When I run render dry-run for artifact "workspace_agents" with profile "base"
    Then the command succeeds
    And stdout contains "## Codex MCP Example"
    And stdout contains "## Claude MCP Example"
    And stdout contains "## Gemini MCP Example"
    And stdout contains "\"type\": \"stdio\""
    And stdout contains "trust = false"

  Scenario: real-example renders directory artifacts to an output directory
    Given the workspace fixture "real-example"
    When I render artifact "agents_dir" to "demo-render/.agents" with profile "base"
    Then the command succeeds
    And the file "demo-render/.agents/worker.toml" contains "bearer_token_env_var = \"TAVILY_API_KEY\""
    And the file "demo-render/.agents/explorer.toml" contains "command = \"npx\""

  Scenario: mcp render outputs codex-compatible JSON for the whole workspace
    Given the workspace fixture "real-example"
    When I run mcp render to stdout
    Then the command succeeds
    And stdout contains "\"mcpServers\""
    And stdout contains "\"bearer_token_env_var\": \"TAVILY_API_KEY\""
