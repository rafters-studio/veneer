# Generated from legion requirement FR-VEN-064, revision 2026-11-02T08:57:10.233445+00:00.
# Do not edit: regenerate with scripts/features.mjs.

Feature: Read: the project as astro check's one TypeScript program

  @criterion-FR-VEN-064-c1 @module:packages/veneer
  Scenario: A path alias resolves to the component it names
    Given the read fixture
    And the project is opened with openProject
    When setUnsaved("src/pages/index.astro", <the file's text with its fourth line replaced by `<Card title={1} />`>) is called
    Then diagnostics("src/pages/index.astro") is one diagnostic with code "ts(2322)", severity "error", line 4, column 7, endLine 4, endColumn 12

  @criterion-FR-VEN-064-c2 @module:packages/veneer
  Scenario: Unsaved text stands in for its file, and the file on disk is untouched
    Given the read fixture
    And the project is opened with openProject
    And the bytes and modified time of src/pages/index.astro are noted
    When setUnsaved("src/pages/index.astro", <the file's text with its fourth line replaced by `<Card title="Hello" tone="x" />`>) is called
    Then diagnostics("src/pages/index.astro") is one diagnostic with code "ts(2322)", severity "error", line 4, column 21, endLine 4, endColumn 25
    And src/pages/index.astro has the noted bytes and modified time

  @criterion-FR-VEN-064-c5 @module:packages/veneer
  Scenario: Removing unsaved text returns the file on disk
    Given the read fixture
    And the project is opened with openProject
    And setUnsaved("src/pages/index.astro", <the file's text with its fourth line replaced by `<Card title="Hello" tone="x" />`>) has been called
    When clearUnsaved("src/pages/index.astro") is called
    Then diagnostics("src/pages/index.astro") is empty

  @criterion-FR-VEN-064-c3 @module:packages/veneer
  Scenario: A project whose typescript is 7 is read with veneer's TypeScript 6.0.3
    Given the read fixture with "typescript": "7.0.2" added to its devDependencies, installed with pnpm install
    When the project is opened with openProject
    Then project.ts.version is "6.0.3"
    And project.ts is the module require("typescript") gives from veneer's own package
    And diagnostics("src/pages/index.astro") is empty

  @criterion-FR-VEN-064-c6 @module:packages/veneer
  Scenario: A project with no Astro reads its .tsx components with nothing on stderr
    Given the no-Astro fixture
    And a child script, written by the step to a temporary .mjs file, that imports openProject from the absolute path of packages/veneer/dist/index.mjs, opens the copy, writes to stdout one line of JSON {"hasButton": <program().getSourceFile(path.join(project.root, "src/components/button.tsx")) !== undefined>}, and closes the project
    When node runs the child script
    Then its stdout is the line {"hasButton":true}
    And its stderr is empty
    And it exits with code 0

  @criterion-FR-VEN-064-c7 @module:packages/veneer
  Scenario: .astro and .tsx components are read in one program
    Given the read fixture
    When the project is opened with openProject
    Then program().getSourceFile(path.join(project.root, "src/components/ui/card.astro")) and program().getSourceFile(path.join(project.root, "src/components/button.tsx")) are both defined
    And diagnostics("src/components/button.tsx") is two diagnostics of code "ts(7026)" and severity "error", the first at line 1, column 80, endLine 1, endColumn 89, the second at line 1, column 59, endLine 1, endColumn 67

  @criterion-FR-VEN-064-c10 @module:packages/veneer
  Scenario: One project is open per process
    Given the read fixture opened with openProject
    When openProject is called on the no-Astro fixture before the first is closed
    Then it rejects with a ProjectError of code "already-open" and the message "a veneer project is already open in this process"
    And after the first project is closed, openProject on the no-Astro fixture resolves

  @criterion-FR-VEN-064-c11 @module:packages/veneer
  Scenario: close restores ts.sys and ends the project
    Given ts.sys.readFile, ts.sys.getModifiedTime and ts.sys.fileExists of require("typescript") resolved from packages/veneer, noted before openProject
    And the read fixture opened with openProject, with unsaved text set for src/pages/index.astro
    When the project is closed
    Then ts.sys.readFile, getModifiedTime and fileExists are the noted functions
    And diagnostics("src/pages/index.astro") rejects with a ProjectError of code "closed" and the message "the veneer project is closed"

  @criterion-FR-VEN-064-c13 @module:packages/veneer
  Scenario: Opening a project never runs its code
    Given the untrusted fixture
    And VENEER_TEST_MARKER names a path in the fixture copy
    When the project is opened with openProject and diagnostics("src/pages/imports-svelte.astro") is called
    Then the file at that path does not exist

  @criterion-FR-VEN-064-c14 @module:packages/veneer
  Scenario: An import of a svelte file gives one diagnostic and no output
    Given the untrusted fixture
    And a child script, written by the step to a temporary .mjs file, that imports openProject from the absolute path of packages/veneer/dist/index.mjs, opens the copy, writes to stdout one line of JSON of diagnostics("src/pages/imports-svelte.astro") mapped to its code, line and column, and closes the project
    When node runs the child script
    Then its stdout is the line [{"code":"ts(2306)","line":2,"column":17}]
    And its stderr is empty
    And it exits with code 0

  @criterion-FR-VEN-064-c15 @module:packages/veneer
  Scenario: A page that is not on disk yet can be opened with its text
    Given the read fixture
    When the project is opened with openProject and unsaved holding src/pages/new.astro with the text of src/pages/index.astro, its fourth line replaced by `<Card title={1} />`
    Then diagnostics("src/pages/new.astro") is one diagnostic with code "ts(2322)", severity "error", line 4, column 7, endLine 4, endColumn 12
    And src/pages/new.astro does not exist on disk

  @criterion-FR-VEN-064-c16 @module:packages/veneer
  Scenario: A new path the tsconfig does not take is not a root file
    Given the read fixture
    When the project is opened with openProject and unsaved holding elsewhere/new.astro with the text "<div />"
    Then diagnostics("elsewhere/new.astro") rejects with a ProjectError of code "not-root-file"
