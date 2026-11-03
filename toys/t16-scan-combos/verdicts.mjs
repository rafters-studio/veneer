// Hand verdicts: one reader (the toy's author), reading results/census-*.txt and, where named, the source.
// R: Container or Grid (or, for raw elements, the named rafters equivalent) truly replaces it, with a gain.
// W: a mechanical swap works, but gains little or needs class overrides.
// O: another rafters component is the real answer, so "use Container or Grid" would mislead.
// K: raw is right, or nothing installed replaces it.
const flex = ["K", "flex row: no Flex component, Container stacks and Grid is columns"];
const typo = ["O", "text in a div: Typography"];
const card = ["O", "chrome: Card or a Card part"];
const stack = ["R", "flex-col gap: Container gap"];
const term = ["K", "terminal line inside code panel"];
const scene = ["K", "custom-CSS scene, no layout decision to replace"];
const hook = ["K", "script hook"];
const canvas = ["K", "canvas or SVG host"];
const fullBleed = ["K", "full-bleed paint around a sized Container: merging insets the paint"];
const bareInContainer = ["W", "Container as=section works, gains only a query context"];
const merge = ["R", "bare wrapper of one Container: Container as=section"];
const landmark = ["W", "Container as=main or article works, gains little"];
const oldHeader = ["K", "this copy's Container has no header, nav or footer mode"];
const stub = ["O", "placeholder panel: Card"];

export const DIV = {
  huttspawn: {
    "src/components/Shape.astro": { 27: ["W", "bg and padding, Container padding and fill could carry it"] },
    "src/components/hs-panel.astro": { 20: ["K", "data-part root of a component that nearly qualifies"] },
  },
  "rafters.studio": {
    "src/components/mail/ArchitectureDiagram.astro": { 6: card, 7: flex, 123: flex },
    "src/components/mail/FaqSection.astro": { 13: ["K", "hairline around a list"] },
    "src/components/mail/Hero.astro": { 59: ["W", "label and value stack, nesting a Container here is what its doc warns of"] },
    "src/components/mail/OssCallout.astro": { 19: stack, 32: ["W", "stack with responsive alignment overrides"] },
    "src/components/mail/SiteHeader.astro": { 19: flex },
    "src/components/mail/composites/EyebrowLabel.astro": { 18: ["K", "inline label root"] },
    "src/components/mail/composites/FAQItem.astro": { 26: ["K", "max-w-prose on slotted text, not a page shell"] },
    "src/components/mail/composites/InstallCard.astro": { 27: card, 50: term, 51: term, 57: term, 58: term, 61: term, 73: card },
    "src/components/mail/composites/ModuleCell.astro": { 15: typo, 20: typo, 23: flex },
    "src/components/mail/composites/SectionHead.astro": { 13: ["W", "stack that turns into a row at lg, needs overrides"], 14: stack, 18: ["K", "responsive measure on a paragraph"] },
    "src/components/mail/composites/StatCell.astro": { 16: typo },
    "src/layouts/MailDocs.astro": { 71: ["W", "CSS flex column shell, read in the style block"], 80: ["K", "CSS flex row with a 240px sidebar, no Grid preset"], 84: ["K", "nav group"] },
  },
  "runlegion.dev": {
    "src/components/ArchitectureCanvas.astro": { 6: canvas },
    "src/components/HexCard.astro": { 18: ["K", "SVG-backed custom card"], 33: scene },
    "src/components/LegionHero.astro": { 7: scene, 8: scene, 10: typo, 17: canvas, 21: scene, 23: typo, 24: typo, 25: typo, 30: typo, 33: typo, 34: typo, 35: typo, 40: typo, 43: typo, 44: typo, 45: typo, 50: typo, 54: scene, 55: typo, 56: hook, 59: scene },
  },
  "sean.silvius.me": {
    "src/components/content-section.tsx": { 2: ["K", "margin wrapper for MDX"] },
    "src/components/conversation-view.tsx": { 2: stub },
    "src/components/dashboard-comparison.tsx": { 2: stub },
    "src/components/flowchart-view.tsx": { 2: stub },
    "src/components/intervention-results.tsx": { 2: stub },
    "src/components/onboarding-analytics-dashboard.tsx": { 2: stub },
    "src/components/onboarding-success-metrics.tsx": { 2: stub },
    "src/components/technical-onboarding-flow.tsx": { 2: stub },
    "src/components/user-centered-onboarding-flow.tsx": { 2: stub },
    "src/components/user-centered-toolkit.tsx": { 2: stub },
    "src/components/Topbar.astro": { 21: flex, 43: hook },
    "src/pages/index.astro": { 46: ["O", "image frame: AspectRatio, not installed here"] },
    "src/pages/posts/index.astro": { 49: ["O", "image frame: AspectRatio, not installed here"] },
    "src/pages/stories/[...slug].astro": { 31: flex, 48: ["W", "prose wrapper: Container as=article has typography, but not the same typography"] },
    "src/pages/stories/index.astro": { 20: stack, 24: ["W", "p-6 inside a Card: Container padding, or a Card part"], 25: flex },
  },
  "smugglr.dev": {
    "src/components/HeroInstallCTA.astro": { 17: ["W", "stack, but gap-2.5 is off Container's scale"] },
    "src/components/HeroTopology.astro": { 21: card, 25: flex, 26: flex, 34: canvas, 141: flex },
    "src/components/MrsBlock.astro": { 101: flex, 102: flex, 130: ["K", "accent rule callout"], 139: ["K", "dt and dd group: HTML wants a div in a dl"], 169: card, 170: typo, 171: typo },
    "src/components/NarrativeSections.astro": { 22: fullBleed, 26: typo, 55: typo, 89: typo, 95: typo, 134: typo, 158: typo },
    "src/components/SiteHeader.astro": { 10: flex },
    "src/components/VoiceHero.astro": { 37: ["R", "w-full max-w-xl: Container size=xl"] },
  },
};

// Sectioning elements, keyed file -> line.
export const SECTIONING = {
  "gitpress.app": { "src/pages/index.astro": { 6: ["K", "landmark, no components installed"] } },
  huttspawn: { "src/components/nav/SiteFooter.astro": { 70: ["K", "labelled nav, Container has no nav mode"] } },
  "rafters.studio": {
    "src/components/Hero.astro": { 9: merge },
    "src/components/Section.astro": { 14: fullBleed },
    "src/components/TopBar.astro": { 6: oldHeader },
    "src/components/mail/ArchitectureSection.astro": { 7: fullBleed },
    "src/components/mail/ComparisonSection.astro": { 14: fullBleed },
    "src/components/mail/FaqSection.astro": { 7: fullBleed },
    "src/components/mail/Hero.astro": { 34: fullBleed },
    "src/components/mail/ModulesSection.astro": { 11: fullBleed },
    "src/components/mail/OssCallout.astro": { 14: fullBleed },
    "src/components/mail/SiteHeader.astro": { 17: oldHeader, 22: flex },
    "src/components/mail/WhyEdgeSection.astro": { 9: fullBleed },
    "src/layouts/Base.astro": { 109: landmark },
    "src/layouts/MailDocs.astro": { 72: oldHeader, 74: oldHeader, 81: landmark, 82: oldHeader, 103: landmark, 104: landmark },
    "src/pages/mail/index.astro": { 15: landmark },
  },
  "runlegion.dev": {
    "src/components/Hero.astro": { 9: merge },
    "src/components/LegionHero.astro": { 9: scene },
    "src/layouts/Base.astro": { 126: oldHeader, 182: landmark },
  },
  "sean.silvius.me": {
    "src/components/Topbar.astro": { 16: oldHeader, 17: oldHeader },
    "src/layouts/Base.astro": { 74: landmark, 78: fullBleed },
    "src/pages/index.astro": { 34: fullBleed },
    "src/pages/software/index.astro": { 64: ["K", "groups a Container and a Separator"] },
    "src/pages/stories/[...slug].astro": { 23: oldHeader, 29: oldHeader },
  },
  "smugglr.dev": {
    "src/components/Hero.astro": { 9: merge },
    "src/components/MrsBlock.astro": { 99: fullBleed, 111: bareInContainer, 116: bareInContainer, 121: bareInContainer, 126: bareInContainer, 131: oldHeader, 148: bareInContainer, 154: bareInContainer, 159: bareInContainer, 165: bareInContainer, 177: bareInContainer },
    "src/components/MrsSectionHeader.astro": { 16: oldHeader },
    "src/components/NarrativeSections.astro": { 25: bareInContainer, 54: bareInContainer, 88: bareInContainer, 94: bareInContainer, 133: bareInContainer, 157: bareInContainer },
    "src/components/Section.astro": { 14: fullBleed },
    "src/components/SiteHeader.astro": { 9: oldHeader, 13: oldHeader },
    "src/components/TopBar.astro": { 6: oldHeader },
    "src/components/VoiceHero.astro": { 17: fullBleed },
    "src/layouts/Base.astro": { 158: landmark },
  },
};

// Raw elements: I read all of them, and the verdicts group by what I saw, so they are rules.
export function rawVerdict(row, site) {
  const { tag, file, line } = row;
  if (tag === "a") return ["K", "no site has a Link component"];
  if (tag === "img") return ["K", "no Image installed on the sites holding these"];
  if (tag === "table") return ["R", "smugglr has Table installed"];
  if (tag === "button") {
    if (file.endsWith("LegionHero.astro")) return line >= 60 ? ["R", "plain action: Button"] : ["K", "phase tile with block content and custom CSS"];
    if (file.endsWith("HeroInstallCTA.astro")) return ["W", "copy CTA with its own chrome: Button with overrides"];
    return ["K", "pagination dot"];
  }
  if (site === "huttspawn") return ["K", "bare text inside Container as=article, which sets the typography"];
  if (site === "gitpress.app") return ["K", "no Typography installed"];
  if (file.endsWith("docs/index.astro")) return ["K", "prose page inside a docs layout that styles bare elements"];
  if (file.endsWith("MailDocs.astro")) return ["W", "nav heading with custom CSS: Label or Small"];
  return ["R", "styled text: Typography"];
}
