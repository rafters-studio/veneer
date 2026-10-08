# Generated from the legion requirement. Do not hand-edit.
Feature: Mode detection (FR-VEN-033)

  @smoke
  Scenario: A project root without rafters rich source runs in sidecar mode
    Given a project root with no component matrix and no behavior files
    When veneer detects the mode
    Then the mode is Sidecar
