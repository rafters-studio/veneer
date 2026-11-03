# Written by hand from issue #161, which traces to no requirement to generate one from.
# Tags past @criterion-3 number the issue's other acceptance criteria; #7 rests on lint.expected.jsonl.

Feature: What classy guarded is an error in the check

  @criterion-1
  Scenario Outline: a bracket value is an error wherever it is written
    Given a page with <written>
    When veneer's check runs
    Then it reports rafters(arbitrary-value) at severity error

    Examples:
      | written                                      |
      | class="p-[3px]"                              |
      | class={classy("p-[3px]", cond && "m-[1px]")} |
      | const box = classy("p-[3px]") outside JSX    |
      | const box = "p-[3px]" used in class={box}    |

  @criterion-1
  Scenario: a const imported from the project's own source is reported where it is used
    Given a page with const box imported from the own source, used in class={box}
    When veneer's check runs
    Then it reports rafters(arbitrary-value) at severity error
    And each finding sits on box in class={box}

  @criterion-1
  Scenario: a const imported from a dependency is not the project's own source
    Given a page with const box imported from a dependency, used in class={box}
    When veneer's check runs
    Then it reports nothing

  @criterion-1
  Scenario: a bracket in a modifier is not a bracket value
    Given a page with class="data-[state=open]:p-2"
    When veneer's check runs
    Then it reports nothing

  @criterion-2
  Scenario Outline: fill takes exactly one bg-fill token
    Given a project whose source has fill="<value>" on a rafters component
    When veneer's check runs
    Then it reports <code>

    Examples:
      | value               | code                  |
      | bg-fill-glass extra | rafters(invalid-fill) |
      | BG-FILL-X           | rafters(invalid-fill) |
      | background          | rafters(retired-fill) |
      | bg-fill-brand       | nothing               |
      |                     | nothing               |

  @criterion-3
  Scenario Outline: surface takes a surface in the registry
    Given a project whose tokens hold surface-card in the namespace fill
    And its source has surface="<value>" on a rafters component
    When veneer's check runs
    Then it reports <code>

    Examples:
      | value  | code                     |
      | card   | nothing                  |
      | nosuch | rafters(unknown-surface) |
      |        | nothing                  |

  @criterion-3
  Scenario: tokens that cannot be loaded are said, and no surface is named
    Given a project whose source has surface="card" on a rafters component
    And the project's tokens cannot be loaded
    When veneer's check runs
    Then it reports rafters(unknown-surface)
    And each finding says the tokens could not be loaded and names no surface

  @criterion-4
  Scenario: a finding that needs no tokens does not load them
    Given a project whose source has fill="bg-fill-glass extra" on a rafters component
    When veneer's check runs
    Then it reports rafters(invalid-fill)
    And the tokens code was not loaded

  @criterion-4
  Scenario: an empty surface does not load the tokens
    Given a project whose source has surface="" on a rafters component
    When veneer's check runs
    Then it reports nothing
    And the tokens code was not loaded

  @criterion-4
  Scenario: a surface loads the tokens
    Given a project whose source has surface="nosuch" on a rafters component
    When veneer's check runs
    Then the tokens code was loaded

  @criterion-5
  Scenario: the check changes nothing on disk
    Given a project whose source has surface="nosuch" on a rafters component
    When veneer's check runs
    Then it reports rafters(unknown-surface)
    And the source file is unchanged

  @criterion-6
  Scenario Outline: the language server's worker gives check's answer
    Given a project whose source has <use> on a rafters component
    When veneer's check runs
    And the language server's worker reads the same page
    Then the worker's diagnostics equal check's

    Examples:
      | use                        |
      | surface="nosuch"           |
      | fill="bg-fill-glass extra" |
