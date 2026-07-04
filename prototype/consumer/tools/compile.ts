/**
 * Prototype composite compiler (tsx-run). Implements COMPILE-CONTRACT.md
 * rules 1-15 against the REAL project inputs:
 *
 * - class vocabularies are IMPORTED from components/ui/*.classes.ts -- the
 *   actual exported records, never re-derived or guessed;
 * - applied rules map to native HTML5 constraint attributes (rule-attrs
 *   semantics); validity is read from element.validity at runtime;
 * - emitted elements subclass RaftersElement (lib/primitives), sharing one
 *   utility sheet via RaftersElement.setUtilityCSS();
 * - the composite surface is a real Container (fill/padding/as), not an
 *   invented wrapper;
 * - the sheet adaptation (:root -> :root,:host + inheritance guard) happens
 *   here at compile time and is written next to the emitted module;
 * - knob enum options are filtered to variants whose classes the compiled
 *   sheet actually covers (contract rule 10).
 *
 * Output: preview-dist/<id>.entry.ts (esbuild-bundled to <id>.mjs),
 * preview-dist/sheet.css, preview-dist/index.html.
 *
 * The Rust port of this lives in veneer-adapters when the FR waves land;
 * this file is the executable form of the contract, not the product.
 */

import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { build } from 'esbuild';

import {
  inputBaseClasses,
  inputSizeClasses,
  inputVariantClasses,
} from '../components/ui/input.classes.ts';
import {
  buttonVariantClasses,
  buttonSizeClasses,
  buttonBaseClasses,
} from '../components/ui/button.classes.ts';
import {
  spinnerBaseClasses,
  spinnerSizeClasses,
  spinnerVariantClasses,
} from '../components/ui/spinner.classes.ts';
import { kbdBaseClasses } from '../components/ui/kbd.classes.ts';
import { labelBaseClasses, labelVariantClasses } from '../components/ui/label.classes.ts';
import { fieldContainerClasses } from '../components/ui/field.classes.ts';
import {
  inputGroupBaseClasses,
  inputGroupSizeClasses,
} from '../components/ui/input-group.classes.ts';
import {
  containerBackgroundClasses,
  containerPaddingClasses,
  containerGapClasses,
} from '../components/ui/container.classes.ts';

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, '..');
const outDir = join(root, 'preview-dist');
mkdirSync(outDir, { recursive: true });

// ---------------------------------------------------------------------------
// Manifest + boundary (port of deriveCompositeBoundary)
// ---------------------------------------------------------------------------
interface Block {
  id: string;
  type: string;
  input?: string[];
  output?: string[];
  content?: unknown;
  children?: string[];
  parentId?: string;
  meta?: Record<string, unknown>;
  rules?: (string | { name: string; config: Record<string, unknown> })[];
}
interface CompositeFile {
  manifest: {
    id: string;
    name: string;
    cognitiveLoad: number;
    usagePatterns?: { do: string[]; never: string[] };
  };
  input: string[];
  output: string[];
  blocks: Block[];
}

function deriveBoundary(c: CompositeFile): { input: string[]; output: string[] } {
  const produced = new Set<string>();
  const consumed = new Set<string>();
  for (const b of c.blocks) {
    for (const o of b.output ?? []) produced.add(o);
    for (const i of b.input ?? []) consumed.add(i);
  }
  const input = [...consumed].filter((r) => !produced.has(r));
  const output = [...produced].filter((r) => !consumed.has(r));
  return { input, output };
}

// ---------------------------------------------------------------------------
// rule -> native constraint attributes (rule-attrs.ts semantics, ported)
// ---------------------------------------------------------------------------
const STRING_RULE_ATTRS: Record<string, Record<string, unknown>> = {
  required: { required: true },
  email: { type: 'email' },
  url: { type: 'url' },
  password: { type: 'password', minlength: 8 },
};

function ruleAttrs(rules: Block['rules'] = []): Record<string, unknown> {
  const attrs: Record<string, unknown> = {};
  for (const rule of rules) {
    if (typeof rule === 'string') Object.assign(attrs, STRING_RULE_ATTRS[rule] ?? {});
    else if (rule.name === 'pattern' && typeof rule.config.pattern === 'string')
      attrs.pattern = rule.config.pattern;
  }
  return attrs;
}

// ---------------------------------------------------------------------------
// Sheet adaptation (contract rules 3-4)
// ---------------------------------------------------------------------------
const standalone = readFileSync(join(root, '.rafters/output/rafters.standalone.css'), 'utf8');
const GUARD =
  ':host{all:initial;display:block;font-family:var(--font-sans);font-size:16px;line-height:1.5;color:var(--foreground)}';
const adaptedSheet = GUARD + standalone.replace(/:root(?!,:host)/g, ':root,:host');
writeFileSync(join(outDir, 'sheet.css'), adaptedSheet);

/** Contract rule 10: an enum option is offered only if every class it needs
 * is present in the compiled sheet. */
function coveredOptions(record: Record<string, string>): string[] {
  return Object.keys(record).filter((k) =>
    record[k]
      .split(/\s+/)
      .filter((cls) => !cls.includes(':'))
      .every((cls) => adaptedSheet.includes(`.${cls.replace(/([./])/g, '\\$1')}`) || adaptedSheet.includes(`.${cls}`)),
  );
}

// ---------------------------------------------------------------------------
// Per-type element emitters. Class strings come from the imported records;
// each emitter returns the TS source of a RaftersElement subclass.
// ---------------------------------------------------------------------------
const esc = (s: string) => s.replace(/`/g, '\\`');

interface Emitted {
  tag: string;
  source: string;
}

function emitLabel(): Emitted {
  const cls = `${labelBaseClasses} ${labelVariantClasses.default}`;
  return {
    tag: 'veneer-label',
    source: `
export class VeneerLabel extends RaftersElement {
  render() { return this.el('label', ${JSON.stringify('CLS_LABEL')}, this.attr('content')); }
}`.replace('"CLS_LABEL"', JSON.stringify(cls)),
  };
}

function emitKbd(): Emitted {
  return {
    tag: 'veneer-kbd',
    source: `
export class VeneerKbd extends RaftersElement {
  render() { return this.el('kbd', ${JSON.stringify(kbdBaseClasses)}, this.attr('content')); }
}`,
  };
}

function emitInput(block: Block): Emitted {
  const size = (block.meta?.size as string) ?? 'default';
  const cls = `${inputBaseClasses} ${inputVariantClasses.default} ${inputSizeClasses[size as keyof typeof inputSizeClasses]}`;
  const attrs = ruleAttrs(block.rules);
  const inputType = (block.meta?.inputType as string) ?? (attrs.type as string) ?? 'search';
  return {
    tag: 'veneer-input',
    source: `
export class VeneerInput extends RaftersElement {
  outputRules = [];
  _input = null;
  _value = '';
  render() {
    const input = document.createElement('input');
    input.className = ${JSON.stringify(cls)};
    input.type = ${JSON.stringify(inputType)};
    // Applied rules as native constraint attributes (rule-attrs semantics):
    for (const [k, v] of Object.entries(${JSON.stringify(attrs)})) {
      if (k === 'type') continue;
      if (v === true) input.setAttribute(k, ''); else input.setAttribute(k, String(v));
    }
    input.placeholder = this.attr('placeholder');
    input.setAttribute('aria-label', this.attr('placeholder', 'input'));
    input.value = this._value;
    input.addEventListener('input', () => {
      this._value = input.value;
      this.dispatchEvent(new CustomEvent('veneer:prop-change', { bubbles: true, composed: true, detail: { prop: 'value', value: input.value } }));
      this._emit();
    });
    this._input = input;
    return input;
  }
  // Validity comes from the browser's own constraint validation -- the
  // required/type attributes ARE the rules.
  _emit() {
    for (const rule of this.outputRules) {
      this.dispatchEvent(new CustomEvent('rafters:rule', { bubbles: true, composed: false, detail: { rule, value: this._value, valid: this._input.validity.valid, sourceBlock: this.attr('block-id') } }));
    }
  }
  get value() { return this._value; }
  set value(v) { this._value = String(v); if (this._input) { this._input.value = this._value; this._emit(); } }
  get placeholder() { return this.attr('placeholder'); }
  set placeholder(v) { this.setAttribute('placeholder', String(v)); if (this._input) this._input.placeholder = String(v); }
}`,
  };
}

function emitButton(block: Block): Emitted {
  const size = (block.meta?.size as string) ?? 'default';
  const base = `${buttonBaseClasses} ${buttonSizeClasses[size as keyof typeof buttonSizeClasses] ?? ''}`;
  return {
    tag: 'veneer-button',
    source: `
const BUTTON_VARIANTS = ${JSON.stringify(buttonVariantClasses)};
export class VeneerButton extends RaftersElement {
  _latest = {};
  _disabled = ${JSON.stringify((block.input ?? []).length > 0)};
  _variant = ${JSON.stringify((block.meta?.variant as string) ?? 'default')};
  render() {
    const button = this.el('button', ${JSON.stringify(base)} + ' ' + (BUTTON_VARIANTS[this._variant] ?? BUTTON_VARIANTS.default), this.attr('content'));
    button.disabled = this._disabled;
    button.addEventListener('click', () => {
      for (const rule of ${JSON.stringify(block.output ?? [])}) {
        this.dispatchEvent(new CustomEvent('rafters:rule', { bubbles: true, composed: false, detail: { rule, value: this._latest, valid: true, sourceBlock: this.attr('block-id') } }));
      }
    });
    return button;
  }
  ruleInput(rule, value, valid) { this._latest[rule] = value; this.disabled = !valid; }
  get disabled() { return this._disabled; }
  set disabled(v) { this._disabled = Boolean(v); this.update(); }
  get variant() { return this._variant; }
  set variant(v) { this._variant = String(v); this.update(); }
}`,
  };
}

function emitSpinner(block: Block): Emitted {
  const size = (block.meta?.size as string) ?? 'default';
  const variant = (block.meta?.variant as string) ?? 'primary';
  const cls = `${spinnerBaseClasses} ${spinnerVariantClasses[variant as keyof typeof spinnerVariantClasses]} ${spinnerSizeClasses[size as keyof typeof spinnerSizeClasses]}`;
  return {
    tag: 'veneer-spinner',
    source: `
export class VeneerSpinner extends RaftersElement {
  _active = false;
  render() {
    const wrap = this.el('div', ${JSON.stringify(cls)});
    wrap.setAttribute('role', 'status');
    wrap.setAttribute('aria-label', 'busy');
    wrap.hidden = !this._active;
    return wrap;
  }
  ruleInput(rule, value) { this.active = Boolean(value); }
  get active() { return this._active; }
  set active(v) { this._active = Boolean(v); this.update(); }
}`,
  };
}

function emitField(): Emitted {
  return {
    tag: 'veneer-field',
    source: `
export class VeneerField extends RaftersElement {
  render() {
    const div = this.el('div', ${JSON.stringify(fieldContainerClasses)});
    div.append(document.createElement('slot'));
    return div;
  }
}`,
  };
}

function emitInputGroup(): Emitted {
  const cls = `${inputGroupBaseClasses} ${inputGroupSizeClasses.default} gap-2 px-2`;
  return {
    tag: 'veneer-input-group',
    source: `
export class VeneerInputGroup extends RaftersElement {
  render() {
    const div = this.el('div', ${JSON.stringify(cls)});
    div.append(document.createElement('slot'));
    return div;
  }
}`,
  };
}

type EmitterFn = (block: Block) => Emitted;
const EMITTERS: Record<string, EmitterFn> = {
  label: emitLabel,
  kbd: emitKbd,
  input: emitInput,
  button: emitButton,
  spinner: emitSpinner,
  field: emitField,
  'input-group': emitInputGroup,
};

// ---------------------------------------------------------------------------
// Composite emitter
// ---------------------------------------------------------------------------
function compile(compositePath: string): string {
  const composite: CompositeFile = JSON.parse(readFileSync(compositePath, 'utf8'));
  const { manifest, blocks } = composite;
  const boundary = deriveBoundary(composite);

  const emitted = new Map<string, Emitted>();
  for (const block of blocks) {
    const emitter = EMITTERS[block.type];
    if (!emitter) throw new Error(`no emitter for block type '${block.type}' (block ${block.id})`);
    if (!emitted.has(block.type)) emitted.set(block.type, emitter(block));
  }

  const routes: Record<string, string[]> = {};
  for (const block of blocks) {
    for (const rule of block.input ?? []) (routes[rule] ??= []).push(block.id);
  }

  const surfaceClasses = `${containerBackgroundClasses.background ?? 'bg-background'} ${containerPaddingClasses['4']} rounded-md`;

  // Block tree -> creation code, parent-linked from the manifest structure.
  const roots = blocks.filter((b) => !b.parentId);
  const lines: string[] = [];
  for (const block of blocks) {
    const tag = emitted.get(block.type)!.tag;
    lines.push(`const ${varName(block.id)} = document.createElement('${tag}');`);
    lines.push(`${varName(block.id)}.setAttribute('block-id', '${block.id}');`);
    if (typeof block.content === 'string')
      lines.push(`${varName(block.id)}.setAttribute('content', ${JSON.stringify(block.content)});`);
    for (const [k, v] of Object.entries(block.meta ?? {}))
      lines.push(`${varName(block.id)}.setAttribute('${k.toLowerCase()}', ${JSON.stringify(String(v))});`);
    if (block.output?.length && block.type === 'input')
      lines.push(`${varName(block.id)}.outputRules = ${JSON.stringify(block.output)};`);
  }
  for (const block of blocks)
    for (const child of block.children ?? [])
      lines.push(`${varName(block.id)}.append(${varName(child)});`);

  // Knobs (contract rule 10): boundary inputs + real, sheet-covered options.
  const knobs: unknown[] = boundary.input.map((rule) => ({
    name: camel(rule),
    kind: 'boundary-input',
    type: 'boolean',
    default: false,
  }));
  for (const block of blocks) {
    if (block.type === 'button')
      knobs.push({
        name: `${block.id}.variant`,
        kind: 'block-prop',
        type: 'enum',
        options: coveredOptions(buttonVariantClasses),
        default: (block.meta?.variant as string) ?? 'default',
        target: { block: block.id, prop: 'variant' },
      });
    if (block.type === 'input' && block.meta?.placeholder)
      knobs.push({
        name: `${block.id}.placeholder`,
        kind: 'block-prop',
        type: 'string',
        default: block.meta.placeholder,
        target: { block: block.id, prop: 'placeholder' },
      });
  }

  const compositeClass = `
export class ${pascal(manifest.id)} extends RaftersElement {
  #blocks = new Map();
  constructor() {
    super();
    const ROUTES = ${JSON.stringify(routes)};
    const BOUNDARY_OUT = ${JSON.stringify(boundary.output)};
    this.shadowRoot.addEventListener('rafters:rule', (event) => {
      const { rule, value, valid } = event.detail;
      for (const id of ROUTES[rule] ?? []) this.#blocks.get(id)?.ruleInput?.(rule, value, valid);
      if (BOUNDARY_OUT.includes(rule)) this.dispatchEvent(new CustomEvent(rule, { bubbles: true, composed: true, detail: { value, valid } }));
    });
  }
  render() {
    const surface = this.el('section', ${JSON.stringify(surfaceClasses)});
    surface.setAttribute('data-veneer-surface', '${manifest.id}');
    ${lines.join('\n    ')}
    ${blocks.map((b) => `this.#blocks.set('${b.id}', ${varName(b.id)});`).join('\n    ')}
    ${roots.map((r) => `surface.append(${varName(r.id)});`).join('\n    ')}
    return surface;
  }
  ${boundary.input
    .map(
      (rule) => `
  get ${camel(rule)}() { return this.__${camel(rule)} ?? false; }
  set ${camel(rule)}(v) {
    this.__${camel(rule)} = v;
    for (const id of ${JSON.stringify(routes[rule] ?? [])}) this.#blocks.get(id)?.ruleInput?.('${rule}', v, true);
  }`,
    )
    .join('\n')}
}`;

  return `// GENERATED by tools/compile.ts -- DO NOT EDIT (contract: COMPILE-CONTRACT.md)
import { RaftersElement } from '../lib/primitives/rafters-element.ts';

${[...emitted.values()].map((e) => e.source).join('\n')}
${compositeClass}

export const KNOBS = ${JSON.stringify(knobs, null, 2)};
export const MANIFEST_META = ${JSON.stringify(
    {
      id: manifest.id,
      cognitiveLoad: manifest.cognitiveLoad,
      input: boundary.input,
      output: boundary.output,
      doNever: manifest.usagePatterns ?? { do: [], never: [] },
    },
    null,
    2,
  )};

export async function define(utilityCss) {
  RaftersElement.setUtilityCSS(utilityCss);
${[...emitted.values()].map((e) => `  customElements.define('${e.tag}', ${e.source.match(/class (\w+)/)![1]});`).join('\n')}
  customElements.define('veneer-${manifest.id}', ${pascal(manifest.id)});
}
`;
}

function varName(id: string): string {
  return 'b_' + id.replace(/[^a-zA-Z0-9]/g, '_');
}
function camel(s: string): string {
  return s.replace(/-([a-z])/g, (_, c) => c.toUpperCase());
}
function pascal(s: string): string {
  const c = camel(s);
  return c[0].toUpperCase() + c.slice(1);
}

// ---------------------------------------------------------------------------
// Run: compile every composite in composites/, bundle each entry.
// ---------------------------------------------------------------------------
import { readdirSync } from 'node:fs';
const compositesDir = join(root, 'composites');
for (const file of readdirSync(compositesDir)) {
  if (!file.endsWith('.composite.json')) continue;
  const id = file.replace('.composite.json', '');
  const entry = join(outDir, `${id}.entry.ts`);
  writeFileSync(entry, compile(join(compositesDir, file)));
  await build({
    entryPoints: [entry],
    bundle: true,
    format: 'esm',
    outfile: join(outDir, `${id}.mjs`),
    logLevel: 'silent',
  });
  console.log(`compiled ${id} -> preview-dist/${id}.mjs`);
}
