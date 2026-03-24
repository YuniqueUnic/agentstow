Feature: Link CLI And Target Installation

  Scenario: real-example link plan enumerates all targets
    Given the workspace fixture "real-example"
    When I run link plan as json
    Then the command succeeds
    And stdout json has 6 items

  Scenario: real-example link apply installs copy and symlink targets and records state
    Given the workspace fixture "real-example"
    When I run link apply
    Then the command succeeds
    And the state database exists
    And the path "demo-targets/codex-lab/.agents" is a symlink
    And the path "demo-targets/codex-lab/.env" is a symlink
    And the path "demo-targets-2/AGENTS.md" is a symlink
    And the file "demo-targets/codex-lab/AGENTS.md" contains "## Codex MCP Example"

  Scenario: link status reports healthy recorded real-example targets
    Given the workspace fixture "real-example"
    And link has been applied to the workspace
    When I run link status as json
    Then the command succeeds
    And stdout json has 6 items
    And link status json marks target path suffix "demo-targets/codex-lab/AGENTS.md" as healthy
    And link status json marks target path suffix "demo-targets/codex-lab/.agents" as healthy

  Scenario: link repair with force heals an unhealthy recorded target
    Given the workspace fixture "real-example"
    And link has been applied to the workspace
    And I write the exact text "user-owned" to "demo-targets/codex-lab/AGENTS.md"
    When I run link repair with force
    Then the command succeeds
    And the file "demo-targets/codex-lab/AGENTS.md" contains "AgentStow Demo Workspace"
    And link status json marks target path suffix "demo-targets/codex-lab/AGENTS.md" as healthy

  Scenario: link force overwrites a conflicting copy target
    Given the workspace fixture "real-example"
    And I write the exact text "user-owned" to "demo-targets/codex-lab/AGENTS.md"
    When I run link with force for target "workspace_agents_copy"
    Then the command succeeds
    And the file "demo-targets/codex-lab/AGENTS.md" contains "AgentStow Demo Workspace"

  Scenario: directory copy targets are installed and tracked
    Given the workspace fixture "targets/dir-copy"
    When I run link apply
    Then the command succeeds
    And the file "proj/.agents/worker.toml" contains "name = \"dir-copy-demo\""
    And the file "proj/.agents/skills/rule.md" contains "Keep builds reproducible."
    And link status json marks target path suffix "proj/.agents" as healthy
