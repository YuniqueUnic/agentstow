Feature: Scripts CLI

  Scenario: global timeout overrides script timeout
    Given a workspace with a sleepy script
    When I run the sleepy script with global timeout "50"
    Then the command fails with exit code 7
    And stderr contains "脚本超时（50ms）"
