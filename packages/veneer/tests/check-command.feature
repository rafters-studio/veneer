Feature: The rafters-check bin

  @criterion-1
  Scenario: a project with a finding fails
    Given the lint fixture
    When rafters-check runs in its root
    Then the report is on stdout
    And the exit code is 1

  @criterion-2
  Scenario: a root argument checks that root
    Given the lint fixture
    When rafters-check runs elsewhere with its root as the argument
    Then the report is on stdout
    And the exit code is 1

  @criterion-3
  Scenario: a clean project passes
    Given a project with no findings
    When rafters-check runs in its root
    Then the exit code is 0

  @criterion-4
  Scenario: a check that returns ok false fails on stderr
    Given a folder with no rafters config
    When rafters-check runs in its root
    Then stderr holds "not found"
    And stdout is empty
    And the exit code is 1

  @criterion-5
  Scenario: a notice goes to stderr and leaves the exit code
    Given a project with no findings
    And its componentsPath names a folder that does not exist
    When rafters-check runs in its root
    Then stderr holds "does not exist"
    And the exit code is 0

  @criterion-6
  Scenario Outline: other arguments print the usage line
    When rafters-check runs with the arguments <arguments>
    Then stderr holds "usage: rafters-check [root]"
    And stdout is empty
    And the exit code is 1

    Examples:
      | arguments |
      | --watch   |
      | one two   |

  @criterion-7
  Scenario: it writes nothing to the project
    Given the lint fixture
    When rafters-check runs in its root
    Then the project outside node_modules is unchanged

  @criterion-8
  Scenario: vp runs it from a project that has the package installed
    Given the lint fixture with the packed package installed
    When vp exec rafters-check runs in its root
    Then the report is on stdout
    And the exit code is 1
