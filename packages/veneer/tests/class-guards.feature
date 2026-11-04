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
      | const box = { text: "p-[3px]" } used in class={box.text} |
      | const box = { text: "p-[3px]" } as const used in class={box.text} |

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

  # The scenarios below carry the criterion numbers of issue #166.

  @criterion-1
  Scenario: a project's own classes file is checked
    Given a project with src/components/x.classes.ts holding text-[0.625rem] and aria-[current=page]:text-primary
    When rafters-check runs
    Then the report names text-[0.625rem] at its line in src/components/x.classes.ts
    And the report does not name aria-[current=page]:text-primary

  @criterion-1
  Scenario: a classes file in a dependency is not the project's own
    Given a project whose tsconfig takes a .classes.ts file with a bracket value from a dependency
    When rafters-check runs
    Then the report does not name that file

  @criterion-2
  Scenario: an installed classes file is not
    Given a project with a bracket value in a .classes.ts file under componentsPath
    When rafters-check runs
    Then the report does not name that file

  @criterion-3
  Scenario: a const object's property reaches a class
    Given a page with const box = { text: "text-[0.625rem]" } and classy(box.text)
    When rafters-check runs
    Then the report names text-[0.625rem] on the page at the classy call

  @criterion-3
  Scenario: a const object from the own source is reported where it is used
    Given a page with a const object imported from the own source, used in class={box.text}
    When veneer's check runs
    Then it reports rafters(arbitrary-value) at severity error
    And each finding sits on box.text in class={box.text}

  @criterion-3
  Scenario Outline: what is not a const object's bracket value
    Given a page with <written>
    When veneer's check runs
    Then it reports nothing

    Examples:
      | written                                                             |
      | let box = { text: "p-[3px]" } used in class={box.text}              |
      | a function returning { text: "p-[3px]" }, used in class={box.text}  |
      | a const object imported from a dependency, used in class={box.text} |

  @criterion-5
  Scenario: the worker gives check's answer on a const object's property
    Given a page with const box = { text: "p-[3px]" } used in class={box.text}
    When veneer's check runs
    And the language server's worker reads the same page
    Then the worker's diagnostics equal check's

  @criterion-5
  Scenario: the worker gives check's answer on a classes file
    Given a project with src/components/x.classes.ts holding text-[0.625rem] and aria-[current=page]:text-primary
    When rafters-check runs
    And the language server's worker reads src/components/x.classes.ts
    Then the worker's answer on the classes file is the report's

  @criterion-6
  Scenario: a proposed classes file is checked
    Given a project with no findings
    And its componentsPath is src/components/ui
    When rafters-check runs with --stdin src/components/x.classes.ts and a bracket value on stdin
    Then the report names src/components/x.classes.ts
    And the exit code is 1

  @criterion-6
  Scenario: a proposed classes file under componentsPath is not checked
    Given a project with no findings
    When rafters-check runs with --stdin src/components/x.classes.ts and a bracket value on stdin
    Then the report names no file
    And the exit code is 0

  @criterion-6
  Scenario: every other .ts path still passes
    Given a project with no findings
    And its componentsPath is src/components/ui
    When rafters-check runs with --stdin src/components/x.ts and a bracket value on stdin
    Then the report names no file
    And the exit code is 0
