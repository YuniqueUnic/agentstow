Feature: Provider MCP Template Rendering

  Scenario Outline: provider fixture renders using its native default format
    Given the provider workspace "<provider>"
    When I run render dry-run for artifact "config" with profile "base"
    Then the command succeeds
    And stdout contains "<expected>"
    And stdout contains "<format_hint>"

    Examples:
      | provider | expected                                         | format_hint           |
      | codex    | enabled_tools = ["forecast"]                    | startup_timeout_sec = 20 |
      | claude   | "clientId": "claude-client"                     | "callbackPort": 4317  |
      | gemini   | "authProviderType": "google_credentials"        | "trust": true         |
