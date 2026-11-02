# Generated from legion requirement FR-VEN-061, revision 2026-11-01T20:06:35.042611+00:00.
# Do not edit: regenerate with scripts/features.mjs.

Feature: Veneer's settings: the rafters config and its veneer block

  @criterion-FR-VEN-061-c1 @module:packages/veneer
  Scenario: The intent comes from the config
    Given the docs fixture, whose .rafters/config.rafters.json is {"componentsPath":"src/components","intent":"efficient"}
    When writeDocs runs on it
    Then line 1 of .rafters/veneer/docs.jsonl is {"kind":"project","intent":"efficient"}

  @criterion-FR-VEN-061-c2 @module:packages/veneer
  Scenario Outline: The components folders come from the config
    Given the docs fixture with .rafters/config.rafters.json set to {"componentsPath":<componentsPath>,"intent":"efficient"}
    And the test has created the empty folder src/other in the copy
    When writeDocs runs on it
    Then it returns ok true with docs.jsonl holding <count> component lines
    And its notices are <notices>
  
    Examples:
      | componentsPath                                          | count | notices |
      | "src/components"                                        | 15    | [] |
      | ["src/other", {"path": "src/components", "root": true}]   | 15    | [] |
      | ["src/components", "src/components"]                     | 15    | [] |
      | "src/other"                                             | 0     | [] |
      | "src/missing"                                           | 0     | [{"code":"components-folder-missing","message":"src/missing named by componentsPath does not exist"}] |

  @criterion-FR-VEN-061-c6 @module:packages/veneer
  Scenario: A block field of the wrong type is reported and no docs are written
    Given the docs fixture with .rafters/config.rafters.json set to {"componentsPath":"src/components","veneer":{"mdxPath":42}}
    When writeDocs runs on it
    Then it returns ok false with errors [{"code":"config-field","field":"veneer.mdxPath","value":42}] and each error has a message
    And .rafters/veneer/docs.jsonl does not exist

  @criterion-FR-VEN-061-c7 @module:packages/veneer
  Scenario: An absent intent is recorded as null
    Given the docs fixture with .rafters/config.rafters.json set to {"componentsPath":"src/components"}
    When writeDocs runs on it
    Then line 1 of docs.jsonl is {"kind":"project","intent":null}

  @criterion-FR-VEN-061-c8 @module:packages/veneer
  Scenario Outline: A missing or unreadable config stops veneer, named
    Given the docs fixture with .rafters/config.rafters.json <state>
    When writeDocs runs on it
    Then it returns ok false with one error whose code is <code>, whose field is <field>, and whose message contains <contains>
    And .rafters/veneer/docs.jsonl does not exist
  
    Examples:
      | state                                      | code                | field            | contains                       |
      | deleted                                    | "config-missing"    | absent           | ".rafters/config.rafters.json" |
      | holding the text {"componentsPath":        | "config-unreadable" | absent           | ".rafters/config.rafters.json" |
      | set to {"intent":"efficient"}              | "config-field"      | "componentsPath" | "componentsPath"               |
      | set to {"componentsPath":7}                | "config-field"      | "componentsPath" | "componentsPath"               |

  @criterion-FR-VEN-061-c9 @module:packages/veneer
  Scenario: Every field error is reported, ordered by field
    Given the docs fixture with .rafters/config.rafters.json set to {"intent":7,"veneer":{"mdxPath":42}}
    When writeDocs runs on it
    Then it returns ok false with errors, in this order, of code "config-field" and field "componentsPath" with no value, field "intent" with value 7, and field "veneer.mdxPath" with value 42
    And nothing exists at .rafters/veneer

  @criterion-FR-VEN-061-c10 @module:packages/veneer
  Scenario: Each bad componentsPath entry is its own error, in index order
    Given the docs fixture with .rafters/config.rafters.json set to {"componentsPath":["src/components","/abs",{"path":""},"../out",{"path":"src/components","root":false},"src/components/toggle.tsx"]}
    When writeDocs runs on it
    Then it returns ok false with five errors of code "config-field", in this order: field "componentsPath[1]" value "/abs", field "componentsPath[2]" value {"path":""}, field "componentsPath[3]" value "../out", field "componentsPath[4]" value {"path":"src/components","root":false}, field "componentsPath[5]" value "src/components/toggle.tsx"
    And each error's message starts with its field and ": "
