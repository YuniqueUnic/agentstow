Feature: Workspace And Git Status

  Scenario: workspace init with git creates a repo skeleton
    Given an empty temp directory as the working root
    When I run workspace init with git into "ws"
    Then the command succeeds
    And stdout json boolean field "git_inited" is true
    And the path "ws/.git" exists
    And the path "ws/agentstow.toml" exists
    And the path "ws/artifacts/hello.txt.tera" exists

  Scenario: workspace status reports a clean git repository
    Given a clean git repository workspace
    When I run workspace status as json
    Then the command succeeds
    And stdout json field "branch" equals "main"
    And stdout json boolean field "dirty" is false

  Scenario: workspace status reports a dirty git repository
    Given a dirty git repository workspace
    When I run workspace status as json
    Then the command succeeds
    And stdout json field "branch" equals "main"
    And stdout json boolean field "dirty" is true
