# Generated from legion requirement FR-VEN-071, revision 2026-11-02T08:57:10.326815+00:00.
# Do not edit: regenerate with scripts/features.mjs.

Feature: One entry: what a save does

  @criterion-FR-VEN-071-c1 @module:packages/veneer
  Scenario: A save lints everything and writes the docs
    Given the lint fixture
    When check runs with save true
    Then its diagnostics have as many entries as lint.expected.jsonl has lines
    And docsPath is .rafters/veneer/docs.jsonl under the copy, and that file exists

  @criterion-FR-VEN-071-c3 @module:packages/veneer
  Scenario: A call with unsaved text writes nothing
    Given the lint fixture
    When check runs with save true and unsaved holding src/pages/brackets.astro with the text "<div class=\"p-[1px]\">a</div>"
    Then its report names p-[1px] at src/pages/brackets.astro:1:13
    And .rafters/veneer does not exist in the copy
    And src/pages/brackets.astro on disk is unchanged

  @criterion-FR-VEN-071-c4 @module:packages/veneer
  Scenario: A call that is not a save is limited to its paths
    Given the lint fixture
    When check runs with paths ["src/pages/brackets-tsx.tsx"]
    Then every diagnostic's file is src/pages/brackets-tsx.tsx
    And .rafters/veneer does not exist in the copy

  @criterion-FR-VEN-071-c6 @module:packages/veneer
  Scenario: A process that only calls check exits on its own
    Given the lint fixture
    And a child script, written by the step to a temporary .mjs file, that imports check from the absolute path of packages/veneer/dist/index.mjs, calls it with save true and writes the line resolved to stdout, and never calls process.exit
    When node runs the child script
    Then its stdout is the line resolved
    And it exits with code 0

  @criterion-FR-VEN-071-c7 @module:packages/veneer
  Scenario: A new page is checked before it is written
    Given the lint fixture
    When check runs with unsaved holding src/pages/new.astro with the text "---\nimport Button from '@/components/ui/button.astro';\n---\n<Button tone=\"x\">a</Button>\n"
    Then its one diagnostic is at src/pages/new.astro 4:9 with code "ts(2322)" and help "variant, quiet"
    And src/pages/new.astro does not exist on disk

  @criterion-FR-VEN-071-c8 @module:packages/veneer
  Scenario: A new page the tsconfig does not take gives a notice
    Given the lint fixture
    When check runs with unsaved holding elsewhere/new.astro with the text "<div />"
    Then it returns ok true with no diagnostics
    And its notices hold one of code "outside-program" naming elsewhere/new.astro
