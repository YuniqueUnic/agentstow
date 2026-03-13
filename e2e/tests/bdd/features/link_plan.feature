Feature: Link Plan CLI

  Scenario: link plan json is machine readable
    Given a minimal link workspace
    When I run link plan as json
    Then the command succeeds
    And stdout is a json array with 1 item
