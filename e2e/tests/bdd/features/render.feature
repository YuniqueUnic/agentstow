Feature: Render CLI

  Scenario: render dry-run prints rendered text
    Given a minimal render workspace
    When I run render dry-run for artifact "hello"
    Then the command succeeds
    And stdout contains "Hello BDD!"

