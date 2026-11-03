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
    And the exit code is 2

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
    Then stderr holds "usage: rafters-check [root] [--stdin <path>]"
    And stdout is empty
    And the exit code is 2

    Examples:
      | arguments |
      | --watch   |
      | one two   |
      | --stdin   |

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

  # The scenarios below carry the criterion numbers of issue #163.

  @criterion-1
  Scenario Outline: findings and could-not-run exit differently
    Given <project>
    When rafters-check runs with <arguments>
    Then the exit code is <code>

    Examples:
      | project                    | arguments | code |
      | a project with no findings | none      | 0    |
      | the lint fixture           | none      | 1    |
      | a folder with no rafters config | none | 2    |
      | a project with no findings | --nope    | 2    |

  @criterion-2
  Scenario: a proposed file is checked by path and content
    Given a project with no findings
    When rafters-check runs with --stdin src/pages/new.astro and a bracket value on stdin
    Then the report names src/pages/new.astro
    And the exit code is 1
    And src/pages/new.astro does not exist
    And the project outside node_modules is unchanged

  @criterion-2
  Scenario: proposed content replaces what is on disk
    Given the lint fixture
    When rafters-check runs with --stdin on a file with a finding, and clean content on stdin
    Then the exit code is 0
    And the project outside node_modules is unchanged

  @criterion-2
  Scenario: only the proposed file is checked
    Given the lint fixture
    When rafters-check runs with --stdin src/pages/new.astro and a bracket value on stdin
    Then the report names src/pages/new.astro
    And the report names no other file

  @criterion-3
  Scenario: a path outside the root could not run
    Given a project with no findings
    When rafters-check runs with --stdin ../elsewhere.astro
    Then the exit code is 2
    And stderr holds "../elsewhere.astro"
    And stdout is empty

  @criterion-3
  Scenario: an absolute path is taken
    Given a project with no findings
    When rafters-check runs elsewhere with its root and --stdin on the absolute path of src/pages/new.astro
    Then the report names src/pages/new.astro
    And the exit code is 1

  @criterion-3
  Scenario: a relative path is relative to the current directory
    Given a project with no findings
    When rafters-check runs in src/pages with its root and --stdin new.astro
    Then the report names src/pages/new.astro
    And the exit code is 1

  @criterion-4
  Scenario: a file the tsconfig does not take is a notice
    Given a project with no findings
    When rafters-check runs with --stdin elsewhere/new.astro and a bracket value on stdin
    Then stderr holds "elsewhere/new.astro is not in the tsconfig's files"
    And the report names no file
    And the exit code is 0

  @criterion-6
  Scenario: a .ts path is not a failure to run
    Given a project with no findings
    When rafters-check runs with --stdin on a .ts path the tsconfig takes
    Then the exit code is 0

  @criterion-6
  Scenario: an exempt path under componentsPath is not a failure to run
    Given a project with no findings
    When rafters-check runs with --stdin src/components/new.astro and a bracket value on stdin
    Then the report names no file
    And the exit code is 0

  @criterion-7
  Scenario: a finding has the contracted line
    Given a project with no findings
    When rafters-check runs with --stdin src/pages/new.astro and a bracket value on stdin
    Then a line of stdout matches file:line:column severity code message for src/pages/new.astro
