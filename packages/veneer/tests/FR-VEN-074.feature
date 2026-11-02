# Generated from legion requirement FR-VEN-074, revision 2026-11-02T08:05:49.414837+00:00.
# Do not edit: regenerate with scripts/features.mjs.

Feature: docs.jsonl: the line shape, the file and the call that writes it

  @criterion-FR-VEN-074-c1 @module:packages/veneer
  Scenario: The first line is the project line
    Given the docs fixture, whose config's intent is "efficient"
    When writeDocs runs on it
    Then line 1 of .rafters/veneer/docs.jsonl is {"kind":"project","intent":"efficient"}

  @criterion-FR-VEN-074-c2 @module:packages/veneer
  Scenario: Each component has one line, ordered by file then name
    Given the docs fixture
    When writeDocs runs on it
    Then docs.jsonl has 15 component lines
    And the lines for src/components/wrapped.tsx come last, Memo before Ref

  @criterion-FR-VEN-074-c3 @module:packages/veneer
  Scenario: docs.jsonl is written to docs.jsonl.tmp and renamed over
    Given the docs fixture with an earlier .rafters/veneer/docs.jsonl holding the single line {"kind":"project","intent":"old"}
    And the test has wrapped rename on node:fs/promises to record its calls and called module.syncBuiltinESMExports()
    When writeDocs runs on it
    Then the recorded calls hold one rename from <the copy's path as passed to writeDocs>/.rafters/veneer/docs.jsonl.tmp to <the copy's path as passed to writeDocs>/.rafters/veneer/docs.jsonl
    And docs.jsonl equals packages/veneer/test/fixtures/docs.expected.jsonl byte for byte
    And no docs.jsonl.tmp is left in .rafters/veneer

  @criterion-FR-VEN-074-c4 @module:packages/veneer
  Scenario: writeDocs returns ok with docs.jsonl's path and no notices
    Given the docs fixture
    When writeDocs runs on it, called with the copy's absolute path
    Then it returns {"ok": true, "docsPath": path.resolve(<the copy's absolute path>, ".rafters/veneer/docs.jsonl"), "notices": []}

  @criterion-FR-VEN-074-c5 @module:packages/veneer
  Scenario: A config error returns its code and leaves docs.jsonl as it was
    Given the docs fixture with an earlier docs.jsonl, and its config's veneer block set to {"mdxPath": 42}
    When writeDocs runs on it
    Then it returns ok false with the error code "config-field", field "veneer.mdxPath" and value 42
    And docs.jsonl has the same bytes as before

  @criterion-FR-VEN-074-c6 @module:packages/veneer
  Scenario: The output does not depend on the order files are read in
    Given the docs fixture
    And a second copy of it whose tsconfig.json has "include": [] and a files list naming every .ts, .tsx and .astro file under src, each written as src/components/<name>, in reverse code point order
    When writeDocs runs on each copy, each in its own fresh child process
    Then both docs.jsonl files equal packages/veneer/test/fixtures/docs.expected.jsonl byte for byte
    And each child process writes resolved to stdout, then exits on its own with code 0 within 30 seconds of writing it

  @criterion-FR-VEN-074-c7 @module:packages/veneer
  Scenario: A component file outside the program gets a notice, not a line
    Given the docs fixture with its tsconfig.json include changed to ["src/**/*.ts", "src/**/*.tsx"]
    When writeDocs runs on it
    Then docs.jsonl has no line whose file ends in .astro
    And notices is exactly [{"code":"outside-program","message":"src/components/astro-default.astro is under componentsPath but not in the tsconfig's files"},{"code":"outside-program","message":"src/components/bare.astro is under componentsPath but not in the tsconfig's files"},{"code":"outside-program","message":"src/components/broken.astro is under componentsPath but not in the tsconfig's files"}]

  @criterion-FR-VEN-074-c8 @module:packages/veneer
  Scenario: A project with no tsconfig.json returns tsconfig-missing
    Given the docs fixture with its tsconfig.json deleted
    When writeDocs runs on it
    Then it returns ok false with errors [{"code":"tsconfig-missing","message":"tsconfig.json not found at <the copy's absolute path>"}]
    And nothing exists at .rafters/veneer

  @criterion-FR-VEN-074-c9 @module:packages/veneer
  Scenario: A symbolic link cannot send the write outside the project
    Given the docs fixture, with .rafters/veneer replaced by a symbolic link to a folder outside the copy
    When writeDocs runs on it
    Then it returns ok false with one error of code "output-symlink"
    And that outside folder is empty

  @criterion-FR-VEN-074-c10 @module:packages/veneer
  Scenario: A symbolic link at the temporary path is not followed
    Given the docs fixture, with .rafters/veneer/docs.jsonl.tmp a symbolic link to a file outside the copy holding the text "untouched"
    When writeDocs runs on it
    Then it returns ok true
    And that outside file holds the text "untouched"
