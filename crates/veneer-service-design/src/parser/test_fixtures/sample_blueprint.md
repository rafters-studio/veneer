## Service Blueprint

**Primary Persona:** Developer
**Trigger:** Discovers rafters
**Scope:** First run to production
**Channels:** CLI, Web

### Step 1: Discovery

| Layer | Detail |
| --- | --- |
| Evidence | README, blog posts |
| Customer Actions | Searches for design token tools |
| Frontstage | Landing page, docs |
| Backstage | SEO, content pipeline |
| Support Processes | Static site hosting |

**Pain points:**
- Too many competing tools
- Unclear value proposition

**Emotional state:** Curiosity (+1). Interested but cautious.

**Metrics:**
- Time on landing page > 2 min

**Moments of truth:**
- First impression of documentation quality

### Step 2: Installation

| Layer | Detail |
| --- | --- |
| Evidence | Install output, first run log |
| Customer Actions | Runs npx rafters init |
| Frontstage | CLI prompts and output |
| Backstage | Package resolution, config generation |
| Support Processes | npm registry, Node.js |

**Pain points:**
- Dependency conflicts with existing tools

**Emotional state:** Anxious (-1). Will this break my setup?

## Design Decisions

- CLI-first approach for developer familiarity
- Zero-config defaults with escape hatches

## Open Questions

- Should we support Yarn PnP out of the box?
- How to handle monorepo setups?
