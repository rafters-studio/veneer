# Generated from legion requirement FR-VEN-066, revision 2026-11-04T04:22:20.064876+00:00.
# Do not edit: regenerate with scripts/features.mjs.

Feature: Lint: report only the rafters components

  @criterion-FR-VEN-066-c1 @module:packages/veneer
  Scenario: The lint fixture gives exactly the expected diagnostics
    Given the lint fixture, opened with openProject
    When lint runs with componentFolders ["src/components/ui"]
    Then its diagnostics, in order, equal the lines of lint.expected.jsonl on the fields those lines hold

  @criterion-FR-VEN-066-c2 @module:packages/veneer
  Scenario: A type error with no rafters component is not reported
    Given the lint fixture, opened with openProject
    And project.diagnostics("src/pages/astro-cases.astro") includes one of code "ts(2322)" at line 15
    When lint runs with componentFolders ["src/components/ui"] and files ["src/pages/astro-cases.astro"]
    Then no reported diagnostic is at line 15

  @criterion-FR-VEN-066-c3 @module:packages/veneer
  Scenario: A bracket value in a component folder is not reported
    Given the lint fixture, opened with openProject
    When lint runs with componentFolders ["src/components/ui"] and files ["src/components/ui/uses-bracket.astro"]
    Then it reports nothing
    And when lint runs with componentFolders ["src/widgets"] and the same files it reports one diagnostic of code "rafters(arbitrary-value)"

  @criterion-FR-VEN-066-c4 @module:packages/veneer
  Scenario: The report has a finding line per diagnostic, the later lines of its message, its help and a count
    Given two diagnostics, the first with help "a, b" and the message "bad\nmore"
    When report runs on them
    Then its lines are "a.astro:1:2 error ts(2322) bad", "  more", "  help: a, b", "b.astro:1:2 error ts(2741) worse" and "2 problems"

  @criterion-FR-VEN-066-c5 @module:packages/veneer
  Scenario: A retired prop names its replacement from its own source
    Given the lint fixture, opened with openProject
    When lint runs with componentFolders ["src/components/ui"] and files ["src/pages/astro-cases.astro"]
    Then the diagnostic of code "ts(6385)" has help "Use variant=\"ghost\"."
