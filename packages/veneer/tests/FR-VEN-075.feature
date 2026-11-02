# Generated from legion requirement FR-VEN-075, revision 2026-11-02T05:40:39.671456+00:00.
# Do not edit: regenerate with scripts/features.mjs.

Feature: The @rafters/veneer package: its build, its scenarios, CI and release

  @criterion-FR-VEN-075-c1 @module:packages/veneer
  Scenario: The package builds to dist/index.mjs
    Given the workspace after pnpm install --frozen-lockfile
    When pnpm -r build runs
    Then it exits with code 0
    And packages/veneer/dist/index.mjs and packages/veneer/dist/index.d.mts exist
    And packages/veneer holds no .node file

  @criterion-FR-VEN-075-c2 @module:packages/veneer
  Scenario: A scenario with no @criterion- tag fails
    Given a temporary folder holding untagged.feature, one scenario named "untagged probe" with no tags and the single step "Given a step that passes", and pass.steps.ts defining Given("a step that passes", () => {})
    When pnpm exec cucumber-js <folder>/untagged.feature --import packages/veneer/tests/support/criterion-tag.steps.ts --import <folder>/pass.steps.ts --strict runs from the repository root
    Then it exits with a non-zero code
    And its stdout names "untagged probe"

  @criterion-FR-VEN-075-c5 @module:packages/veneer
  Scenario: The previous build is gone
    Given the repository at the build's head
    When cargo metadata --no-deps --format-version 1 runs at the root
    Then its workspace members are exactly veneer-check and veneer-lsp
    And crates/veneer, crates/veneer-adapters, crates/veneer-node and install.sh do not exist
    And no file under .github/workflows names install.sh
