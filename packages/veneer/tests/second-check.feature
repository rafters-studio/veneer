# Written by hand from issue #164, which traces to no requirement to generate one from.
# Tags number the issue's acceptance criteria. Every call in a scenario runs in one child process.

Feature: A later check in one process answers as a fresh process does

  @criterion-1
  Scenario Outline: a later check in one process answers as a fresh one does
    Given a project with no findings
    When one process runs check, then <change>, then check again
    Then the second report equals the report of a fresh process

    Examples:
      | change                                      |
      | a bracket value added through unsaved text  |
      | a bracket value in a different unsaved file |
      | a bracket value written to the page on disk |
      | an imported const edited on disk            |
      | a component's props edited on disk          |

  @criterion-2
  Scenario: text that is gone is not reported
    Given a project with no findings
    When one process runs check with p-[3px] on a page, then with clean text of the same shape
    Then the second report has no finding

  @criterion-3
  Scenario: a save after a disk edit
    Given a project with no findings
    When one process runs check with save, edits a component's props on disk, and runs check with save again
    Then docs.jsonl equals what a fresh process writes

  @criterion-4
  Scenario Outline: the worker after a disk change it was not told about
    Given a worker open on a project
    When the worker is driven through <sequence>
    Then the worker's answer for the last file equals a fresh worker's

    Examples:
      | sequence                                                             |
      | a new file typed, closed, and typed again with a bracket             |
      | an imported const changed on disk and a new file opened              |
      | a tsx const typed, closed, and a reopen before its importer is typed |
