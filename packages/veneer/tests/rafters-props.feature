# Written by hand from issue #122: FR-RAF-FILL-003 has no verification.scenarios to generate from.
# Tags past @criterion-1 number the issue's other acceptance criteria.

Feature: Checks on rafters component props

  @criterion-1
  Scenario Outline: a retired v2 fill value is flagged with its replacement
    Given a project whose source has fill="<value>" on a rafters component, a <form>
    When veneer's check runs
    Then it flags fill="<value>" with <replacement>
    And the source file is unchanged

    Examples:
      | value             | form                     | replacement                                        |
      | background        | bare word, a surface     | surface="background"                               |
      | notasurface       | bare word, not a surface | a fill the designer authors, fill="bg-fill-<name>" |
      | muted/50          | word/alpha               | a fill the designer authors, fill="bg-fill-<name>" |
      | primary-to-accent | word-to-word             | a fill the designer authors, fill="bg-fill-<name>" |
      | bg-red-500        | bare word, not a surface | a fill the designer authors, fill="bg-fill-<name>" |

  @criterion-1
  Scenario Outline: a value that is not a retired v2 form is not flagged
    Given a project whose source has fill="<value>" on a rafters component
    When veneer's check runs
    Then it reports nothing for that value

    Examples:
      | value               |
      | bg-fill-brand       |
      |                     |

  @criterion-1
  Scenario: a fill on a component that is not a rafters component is not flagged
    Given a project whose source has fill="muted" on its own component
    When veneer's check runs
    Then it reports nothing for that value

  @criterion-2
  Scenario: the language server's worker gives check's answer
    Given a project whose source has fill="background" on a rafters component
    When veneer's check runs
    And the language server's worker reads the same page
    Then the worker's diagnostics equal check's

  @criterion-3
  Scenario: a surface-* token outside the fill namespace does not make a surface
    Given a project whose source has fill="foreground" on a rafters component
    And the project's tokens hold surface-foreground in the namespace semantic
    When veneer's check runs
    Then it flags fill="foreground" with a fill the designer authors, fill="bg-fill-<name>"

  @criterion-4 @criterion-8
  Scenario: the build carries the registry and hands it to nobody
    Then veneer's dist imports no rafters source and no token schema package
    And veneer's published types do not name TokenRegistry

  @criterion-7
  Scenario Outline: the build fails when dist holds what it must not
    Given a dist folder holding the text <name>
    When the build's dist assertion runs on it
    Then it fails naming <name>

    Examples:
      | name                |
      | css-tree            |
      | saveRegistryToDir   |
      | writeFileAtomicSync |

  @criterion-9
  Scenario: a file with no bare-word fill does not load the tokens
    Given a project whose source has fill="muted/50" on a rafters component
    And the project's tokens cannot be loaded
    When veneer's check runs
    Then it flags fill="muted/50" with a fill the designer authors, fill="bg-fill-<name>"
    And the tokens code was not loaded

  @criterion-9
  Scenario: a file with a bare-word fill loads the tokens
    Given a project whose source has fill="background" on a rafters component
    When veneer's check runs
    Then the tokens code was loaded

  @criterion-10
  Scenario: tokens that cannot be loaded name no replacement and stop nothing else
    Given a project whose source has fill="background" and fill="muted/50" on rafters components
    And the project's tokens cannot be loaded
    When veneer's check runs
    Then fill="background" says the tokens could not be loaded and names no replacement
    And it flags fill="muted/50" with a fill the designer authors, fill="bg-fill-<name>"
