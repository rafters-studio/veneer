# Written by hand from issue #158: it traces to no requirement to generate from.
# Tags number the issue's acceptance criteria.

Feature: A custom component with the rafters shape is checked as a system component

  @criterion-2
  Scenario: a qualified custom component is checked like a system one
    Given a project with a custom component outside componentsPath, with all six tags, a behavior file and a classes file
    And a page passing it a prop its type refuses
    When veneer's check runs
    Then it reports the type error at severity error

  @criterion-4
  Scenario Outline: a component missing part of the shape stays the project's own
    Given a project with a custom component outside componentsPath, missing <part>
    And a page passing it a prop its type refuses
    When veneer's check runs
    Then it reports nothing for that use

    Examples:
      | part                    |
      | the @usage-patterns tag |
      | its behavior file       |
      | its classes file        |

  @criterion-8
  Scenario Outline: scan decides which components are checked
    Given a project with a system component, a qualified custom component and an unqualified custom component
    And a page passing each a prop its type refuses
    And the veneer block sets scan to <scan>
    When veneer's check runs
    Then it reports the type error on <reported>

    Examples:
      | scan   | reported                            |
      | system | the system component only           |
      | format | the system and qualified components |
      | all    | all three                           |

  @criterion-1
  Scenario: the six tags must be in one JSDoc block
    Given a project with a custom component outside componentsPath, missing one block holding the tags
    And a page passing it a prop its type refuses
    When veneer's check runs
    Then it reports nothing for that use

  @criterion-2
  Scenario Outline: a retired fill value follows the same uses
    Given a project with a custom component outside componentsPath, <shape>
    And a page passing it fill="background"
    And the veneer block sets scan to <scan>
    When veneer's check runs
    Then it flags fill="background" with surface="background"

    Examples:
      | shape                                                  | scan   |
      | with all six tags, a behavior file and a classes file  | format |
      | missing its classes file                               | all    |

  @criterion-4
  Scenario: a retired fill value on a component that does not qualify is not reported
    Given a project with a custom component outside componentsPath, missing its classes file
    And a page passing it fill="background"
    When veneer's check runs
    Then it reports nothing for that use

  @criterion-10
  Scenario Outline: scan does not change which files are exempt from the bracket check
    Given a project with a custom component outside componentsPath, <shape>
    And the veneer block sets scan to <scan>
    When veneer's check runs on the component's own files
    Then it reports <count> bracket values

    Examples:
      | shape                                                 | scan   | count |
      | with all six tags, a behavior file and a classes file | system | 0     |
      | with all six tags, a behavior file and a classes file | format | 0     |
      | with all six tags, a behavior file and a classes file | all    | 0     |
      | missing its behavior file                             | system | 2     |
      | missing its behavior file                             | format | 2     |
      | missing its behavior file                             | all    | 2     |

  @criterion-5
  Scenario Outline: the language server's worker gives check's answer
    Given a project with a custom component outside componentsPath, <shape>
    And a page passing it a prop its type refuses
    And the veneer block sets scan to <scan>
    When veneer's check runs
    And the language server's worker reads the same page
    Then the worker's diagnostics equal check's

    Examples:
      | shape                                                 | scan   |
      | with all six tags, a behavior file and a classes file | format |
      | missing its classes file                              | all    |

  @criterion-7
  Scenario: deciding runs none of the project's code
    Given a project with a custom component outside componentsPath, with all six tags, a behavior file and a classes file
    And a page passing it a prop its type refuses
    When veneer's check runs
    Then no file of the component was loaded as a module

  @criterion-9
  Scenario: a component declared under node_modules does not count under all
    Given a project with a custom component outside componentsPath, with all six tags, a behavior file and a classes file
    And a page passing a prop its type refuses to a component from node_modules
    And the veneer block sets scan to all
    When veneer's check runs
    Then it reports nothing for that use

  @criterion-11
  Scenario: any other value for scan is a config error
    Given a project with a custom component outside componentsPath, with all six tags, a behavior file and a classes file
    And the veneer block sets scan to everything
    When veneer's check runs on the component's own files
    Then the check fails with a config error on veneer.scan
