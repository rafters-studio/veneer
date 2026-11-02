# Generated from legion requirement FR-VEN-072, revision 2026-11-02T09:39:35.899071+00:00.
# Do not edit: regenerate with scripts/features.mjs.

Feature: The language server

  @criterion-FR-VEN-072-c1 @module:crates/veneer-lsp
  Scenario: A misuse in an open buffer is reported as typed
    Given the lint fixture and veneer-lsp started on it with --worker naming packages/veneer/dist/worker.mjs
    When the client opens src/pages/brackets.astro with the text "<div class=\"p-[1px]\">a</div>"
    Then the server publishes one diagnostic for it, with source "veneer", code "rafters(arbitrary-value)", severity 1, range 0:12 to 0:19
    And when the client changes the text to "<div class=\"p-4\">a</div>" the server publishes an empty list for it

  @criterion-FR-VEN-072-c2 @module:crates/veneer-lsp
  Scenario: A save runs the entry once and publishes its result
    Given the lint fixture and veneer-lsp started on it
    When the client opens src/pages/brackets.astro with the text on disk and sends didSave
    Then the server publishes for src/pages/astro-cases.astro and src/pages/brackets.astro as many diagnostics in total as lint.expected.jsonl has lines for them
    And .rafters/veneer/docs.jsonl exists in the copy

  @criterion-FR-VEN-072-c3 @module:crates/veneer-lsp
  Scenario: A diagnostic help is in its message
    Given the lint fixture and veneer-lsp started on it
    When the client opens src/pages/new.astro with the text "---\nimport Button from '@/components/ui/button.astro';\n---\n<Button tone=\"x\">a</Button>\n"
    Then the server publishes one diagnostic for it at 3:8 whose message ends "\nhelp: variant, quiet"
    And src/pages/new.astro does not exist on disk

  @criterion-FR-VEN-072-c4 @module:crates/veneer-lsp
  Scenario: A burst publishes the newest text
    Given the lint fixture and veneer-lsp started on it, with src/pages/brackets.astro open and its first publish received
    When the client sends 10 changes in a row, the last with the text "<div class=\"p-[9px]\">a</div>"
    Then the last publish for that file holds one diagnostic ending at character 19
    And no publish arrives after it within 1 second

  @criterion-FR-VEN-072-c5 @module:crates/veneer-lsp
  Scenario: Killing the server ends the worker
    Given the lint fixture and veneer-lsp started on it, with a first publish received
    When the server is killed with SIGKILL
    Then within 2 seconds no process with the worker path in its command line remains for this copy

  @criterion-FR-VEN-072-c6 @module:crates/veneer-lsp
  Scenario: A worker that cannot start warns and fails nothing
    Given the lint fixture and veneer-lsp started on it with --worker naming a path that does not exist
    When the client opens src/pages/brackets.astro
    Then the server sends one window/showMessage of type 2
    And it answers shutdown

  @criterion-FR-VEN-072-c9 @module:crates/veneer-lsp
  Scenario: Opening a folder never runs a worker from that folder
    Given the lint fixture, with node_modules/@rafters/veneer/dist/worker.mjs in the copy writing a marker file when run
    And veneer-lsp started on it with no --worker
    When the client opens src/pages/brackets.astro
    Then the server sends one window/showMessage of type 2
    And the marker file does not exist

  @criterion-FR-VEN-072-c10 @module:crates/veneer-lsp
  Scenario: A failed save warns and reporting goes on
    Given the lint fixture and veneer-lsp started on it, with a first publish received for src/pages/brackets.astro
    And .rafters/config.rafters.json in the copy now holds the text "{"
    When the client sends didSave
    Then the server sends one window/showMessage of type 2
    And when the client changes src/pages/brackets.astro to the text "<div class=\"p-[1px]\">a</div>" the server publishes one diagnostic for it
