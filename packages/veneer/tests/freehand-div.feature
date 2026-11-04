# Written by hand from issue #159, which traces to no requirement to generate one from.
# Tags number the issue's acceptance criteria.

Feature: A freehand div around rafters components is a warning

  @criterion-1
  Scenario: a div around a rafters component warns
    Given a page with a div holding a rafters component
    When veneer's check runs
    Then it reports rafters(freehand-div) at severity warning on the div
    And the help names Container and Grid

  @criterion-5
  Scenario: a file using no component in scan is left alone
    Given a project file with a div and no component in scan
    When veneer's check runs
    Then it reports nothing for that file

  @criterion-3
  Scenario: a warning alone does not fail the check
    Given a project whose only diagnostic is rafters(freehand-div)
    When rafters-check runs
    Then the report names rafters(freehand-div)
    And the exit code is 0

  @criterion-4
  Scenario Outline: a component's own file is left alone
    Given <file> holding a div and using a component in scan
    When veneer's check runs
    Then it reports nothing for that file

    Examples:
      | file                             |
      | a file under componentsPath      |
      | a qualified component's own file |

  @criterion-6
  Scenario: the check and the worker agree
    Given a page with a div holding a rafters component
    When veneer's check runs
    And the language server's worker reads the same page
    Then the worker's diagnostics equal check's
