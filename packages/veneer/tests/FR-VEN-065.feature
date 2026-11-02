# Generated from legion requirement FR-VEN-065, revision 2026-11-01T20:06:35.241711+00:00.
# Do not edit: regenerate with scripts/features.mjs.

Feature: Resolve: every component's props and intelligence

  @criterion-FR-VEN-065-c1 @module:packages/veneer
  Scenario: A literal union records its values in source order
    Given the docs fixture, where src/components/literal.tsx declares tone?: 'quiet' | 'loud' | 'calm'
    When veneer writes the docs
    Then Literal's tone has the type ["'quiet' | 'loud' | 'calm'"] and the values ["quiet","loud","calm"]

  @criterion-FR-VEN-065-c2 @module:packages/veneer
  Scenario: A destructured default is recorded as written
    Given the docs fixture, where src/components/param-default.tsx destructures size = 'md' in its parameter
    And src/components/astro-default.astro destructures title = 'Hello' from Astro.props
    When veneer writes the docs
    Then ParamDefault's size has the default "'md'"
    And AstroDefault's title has the default "'Hello'"

  @criterion-FR-VEN-065-c3 @module:packages/veneer
  Scenario: Inherited props are counted by package and type
    Given the docs fixture, where src/components/inherits.tsx takes ButtonHTMLAttributes<HTMLButtonElement> and destructures type = 'button'
    When veneer writes the docs
    Then Inherits has no own props
    And its inherited list holds {"package":"@types/react","type":"ButtonHTMLAttributes","count":10}
    And its inheritedDefaults is {"type":"'button'"}

  @criterion-FR-VEN-065-c4 @module:packages/veneer
  Scenario: An unfamiliar intelligence tag is kept as written
    Given the docs fixture, where src/components/moody.tsx begins with a JSDoc block holding @cognitive-load 2/10 and @mood calm
    When veneer writes the docs
    Then Moody's tags hold {"name":"cognitive-load","text":"2/10"} and then {"name":"mood","text":"calm"}

  @criterion-FR-VEN-065-c6 @module:packages/veneer
  Scenario: The fixture's lines equal the golden lines byte for byte
    Given the docs fixture, packages/veneer/test/fixtures/docs, copied into a fresh temporary folder and installed with pnpm install --frozen-lockfile
    When veneer writes the docs
    Then its .rafters/veneer/docs.jsonl equals packages/veneer/test/fixtures/docs.expected.jsonl byte for byte
