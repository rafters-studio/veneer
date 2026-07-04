/**
 * HAND-WRITTEN CODEGEN TARGET (a spec artifact, not output) for composites/search-box.composite.json.
 *
 * This file is what veneer's generator must emit for a composite: it is the
 * compile contract, written by hand first so the shape is proven in a real
 * browser before codegen exists. Once the generator emits real modules,
 * files like this are READ-ONLY references: fixes amend COMPILE-CONTRACT.md
 * and the generator, never the emitted output. Every class string is read verbatim from the
 * installed *.classes.ts vocabulary; every rule edge comes from the manifest.
 *
 * Contract with consumers (components.jsonl):
 * - The module exports define(sheet): consumers provide ONE shared
 *   CSSStyleSheet (built from .rafters/output/rafters.standalone.css).
 *   Modules never fetch and never embed the sheet - shared asset, stored once.
 * - Child components expose typed camelCase property setters that re-render
 *   on set, and dispatch a `veneer:prop-change` CustomEvent (bubbles,
 *   composed, detail {prop, value}) on USER-driven changes only - suppressed
 *   on programmatic set. (Standing gitpress/kelex contract.)
 * - Rule dataflow: inside a composite, a producer block dispatches
 *   `rafters:rule` (bubbles within the composite shadow root, NOT composed)
 *   with detail {rule, value, valid, sourceBlock}. The composite root owns a
 *   single routing table compiled from the manifest edges. Boundary outputs
 *   re-dispatch from the composite host as a composed CustomEvent named by
 *   the rule; boundary inputs are typed camelCase properties on the host.
 * - Applied rules (manifest block.rules, e.g. "required") gate the edge:
 *   the producer evaluates them and sets detail.valid; consumers decide
 *   behavior on validity (the button disables on invalid query).
 */

// ---------------------------------------------------------------------------
// Class vocabulary - verbatim from components/ui/*.classes.ts (codegen reads
// these via the existing extract_classes pipeline; never re-derived).
// ---------------------------------------------------------------------------
const CLASSES = {
  fieldContainer: 'flex flex-col gap-2',
  label: 'text-label-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70 text-foreground',
  inputGroup:
    'flex items-center w-full rounded-md border border-input bg-background ring-offset-background focus-within:outline-none focus-within:ring-2 focus-within:ring-ring focus-within:ring-offset-2 h-10',
  inputBase:
    'flex w-full rounded-md border bg-background py-2 ring-offset-background placeholder:text-muted-foreground hover:border-input-hover focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 transition-shadow duration-100 motion-reduce:transition-none disabled:cursor-not-allowed disabled:opacity-50',
  inputSize: { sm: 'h-8 px-2 text-label-small', default: 'h-10 px-3 text-body-small', lg: 'h-12 px-4 text-body-medium' },
  inputVariant: { default: 'border-primary focus-visible:ring-2 focus-visible:ring-primary-ring' },
  kbd: 'inline-flex items-center justify-center rounded border border-border bg-muted px-1.5 py-0.5 text-code-small text-muted-foreground shadow-sm',
  buttonBase:
    'inline-flex items-center justify-center rounded-md font-medium ring-offset-background transition-colors focus-visible:outline-none focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 h-10 px-4 py-2',
  buttonVariant: {
    default:
      'bg-primary text-primary-foreground hover:bg-primary-hover active:bg-primary-active focus-visible:ring-2 focus-visible:ring-primary-ring',
  },
  spinnerBase: 'inline-block rounded-full animate-spin motion-reduce:animate-none',
  spinnerVariant: { primary: 'border-primary border-r-transparent' },
  spinnerSize: { sm: 'h-4 w-4 border-2', default: 'h-6 w-6 border-2' },
};

// ---------------------------------------------------------------------------
// Applied rules - compiled from the manifest's rules[] references to the
// registry built-in rule set. Codegen emits only the rules the manifest uses.
// ---------------------------------------------------------------------------
const APPLIED_RULES = {
  required: (value) => typeof value === 'string' && value.trim().length > 0,
};

function evaluateRules(ruleNames, value) {
  return ruleNames.every((name) => (APPLIED_RULES[name] ?? (() => true))(value));
}

// ---------------------------------------------------------------------------
// Shared-sheet plumbing. define(sheet) is the single entry point.
// ---------------------------------------------------------------------------
let SHEET = null;

function attachRoot(el) {
  const root = el.attachShadow({ mode: 'open' });
  root.adoptedStyleSheets = [SHEET];
  return root;
}

/** Dispatch the kelex/gitpress prop-change contract event (user-driven only). */
function propChange(el, prop, value) {
  el.dispatchEvent(
    new CustomEvent('veneer:prop-change', { bubbles: true, composed: true, detail: { prop, value } }),
  );
}

/** Dispatch a rule edge inside the composite (bubbles, not composed). */
function emitRule(el, rule, value, valid, sourceBlock) {
  el.dispatchEvent(
    new CustomEvent('rafters:rule', { bubbles: true, composed: false, detail: { rule, value, valid, sourceBlock } }),
  );
}

// ---------------------------------------------------------------------------
// Child components - one class per manifest block type. Codegen emits these
// from the React source structure (ReactAdapter::extract_structure) with the
// block's meta as initial property values.
// ---------------------------------------------------------------------------

class VeneerLabel extends HTMLElement {
  connectedCallback() {
    const root = attachRoot(this);
    const label = document.createElement('label');
    label.className = CLASSES.label;
    label.textContent = this.getAttribute('content') ?? '';
    root.append(label);
  }
}

class VeneerKbd extends HTMLElement {
  connectedCallback() {
    const root = attachRoot(this);
    const kbd = document.createElement('kbd');
    kbd.className = CLASSES.kbd;
    kbd.textContent = this.getAttribute('content') ?? '';
    root.append(kbd);
  }
}

class VeneerInput extends HTMLElement {
  #value = '';
  #size = 'default';
  #placeholder = '';
  #rules = [];
  #blockId = '';
  #input = null;

  connectedCallback() {
    this.#size = this.getAttribute('size') ?? 'default';
    this.#placeholder = this.getAttribute('placeholder') ?? '';
    this.#rules = (this.getAttribute('rules') ?? '').split(/\s+/).filter(Boolean);
    this.#blockId = this.getAttribute('block-id') ?? '';
    const root = attachRoot(this);
    const input = document.createElement('input');
    input.type = 'search';
    input.className = `${CLASSES.inputBase} ${CLASSES.inputVariant.default} ${CLASSES.inputSize[this.#size]} border-0 focus-visible:ring-0`;
    input.placeholder = this.#placeholder;
    input.setAttribute('aria-label', this.getAttribute('aria-label') ?? this.#placeholder);
    input.addEventListener('input', () => {
      this.#value = input.value;
      propChange(this, 'value', this.#value);
      emitRule(this, this.outputRule, this.#value, evaluateRules(this.#rules, this.#value), this.#blockId);
    });
    this.#input = input;
    root.append(input);
  }

  outputRule = 'query';

  get value() {
    return this.#value;
  }
  /** Programmatic set: re-renders, does NOT dispatch veneer:prop-change. */
  set value(v) {
    this.#value = String(v);
    if (this.#input) this.#input.value = this.#value;
    emitRule(this, this.outputRule, this.#value, evaluateRules(this.#rules, this.#value), this.#blockId);
  }
  get placeholder() {
    return this.#placeholder;
  }
  set placeholder(v) {
    this.#placeholder = String(v);
    if (this.#input) this.#input.placeholder = this.#placeholder;
  }
}

class VeneerButton extends HTMLElement {
  #variant = 'default';
  #disabled = true; // consumes a rule-gated input; starts unfed => disabled
  #latest = undefined;
  #button = null;

  connectedCallback() {
    this.#variant = this.getAttribute('variant') ?? 'default';
    const root = attachRoot(this);
    const button = document.createElement('button');
    button.className = `${CLASSES.buttonBase} ${CLASSES.buttonVariant[this.#variant] ?? CLASSES.buttonVariant.default}`;
    button.textContent = this.getAttribute('content') ?? '';
    button.disabled = this.#disabled;
    button.addEventListener('click', () => {
      emitRule(this, 'search-submitted', { query: this.#latest }, true, this.getAttribute('block-id') ?? '');
    });
    this.#button = button;
    root.append(button);
  }

  /** Rule intake - the composite router calls this for each consumed edge. */
  ruleInput(rule, value, valid) {
    if (rule !== 'query') return;
    this.#latest = value;
    this.disabled = !valid;
  }

  get disabled() {
    return this.#disabled;
  }
  set disabled(v) {
    this.#disabled = Boolean(v);
    if (this.#button) this.#button.disabled = this.#disabled;
  }
  get variant() {
    return this.#variant;
  }
  set variant(v) {
    this.#variant = String(v);
    if (this.#button)
      this.#button.className = `${CLASSES.buttonBase} ${CLASSES.buttonVariant[this.#variant] ?? CLASSES.buttonVariant.default}`;
  }
}

class VeneerSpinner extends HTMLElement {
  #active = false; // consumes a boundary input; starts unfed => hidden
  #wrap = null;

  connectedCallback() {
    const size = this.getAttribute('size') ?? 'default';
    const variant = this.getAttribute('variant') ?? 'primary';
    const root = attachRoot(this);
    const wrap = document.createElement('div');
    wrap.className = `${CLASSES.spinnerBase} ${CLASSES.spinnerVariant[variant] ?? CLASSES.spinnerVariant.primary} ${CLASSES.spinnerSize[size] ?? CLASSES.spinnerSize.default}`;
    wrap.setAttribute('role', 'status');
    wrap.setAttribute('aria-label', 'Search in progress');
    wrap.hidden = !this.#active;
    this.#wrap = wrap;
    root.append(wrap);
  }

  ruleInput(rule, value) {
    if (rule !== 'search-pending') return;
    this.active = Boolean(value);
  }

  get active() {
    return this.#active;
  }
  set active(v) {
    this.#active = Boolean(v);
    if (this.#wrap) this.#wrap.hidden = !this.#active;
  }
}

class VeneerField extends HTMLElement {
  connectedCallback() {
    const root = attachRoot(this);
    const div = document.createElement('div');
    div.className = CLASSES.fieldContainer;
    div.append(document.createElement('slot'));
    root.append(div);
  }
}

class VeneerInputGroup extends HTMLElement {
  connectedCallback() {
    const root = attachRoot(this);
    const div = document.createElement('div');
    div.className = `${CLASSES.inputGroup} gap-2 px-2`;
    div.append(document.createElement('slot'));
    root.append(div);
  }
}

// ---------------------------------------------------------------------------
// The composite. Codegen compiles the manifest's block tree into the template
// and its rule edges into ROUTES. Boundary inputs become camelCase host
// properties; boundary outputs re-dispatch as composed CustomEvents.
// ---------------------------------------------------------------------------

/** Routing table compiled from the manifest: rule -> consuming block ids. */
const ROUTES = {
  'query': ['submit-button'],
  'search-pending': ['pending-spinner'],
};

/** Boundary edges compiled from the manifest (matches deriveCompositeBoundary). */
const BOUNDARY = { input: ['search-pending'], output: ['search-submitted'] };

class VeneerSearchBox extends HTMLElement {
  #blocks = new Map();

  connectedCallback() {
    const root = attachRoot(this);

    // Block tree, compiled from manifest children/parentId structure.
    const field = document.createElement('veneer-field');
    const label = document.createElement('veneer-label');
    label.setAttribute('content', 'Search');
    const group = document.createElement('veneer-input-group');
    const input = document.createElement('veneer-input');
    input.setAttribute('block-id', 'query-input');
    input.setAttribute('placeholder', 'Search components, tokens, patterns...');
    input.setAttribute('size', 'default');
    input.setAttribute('rules', 'required');
    const kbd = document.createElement('veneer-kbd');
    kbd.setAttribute('content', 'Ctrl K');
    const button = document.createElement('veneer-button');
    button.setAttribute('block-id', 'submit-button');
    button.setAttribute('content', 'Search');
    button.setAttribute('variant', 'default');
    const spinner = document.createElement('veneer-spinner');
    spinner.setAttribute('block-id', 'pending-spinner');
    spinner.setAttribute('size', 'sm');
    spinner.setAttribute('variant', 'primary');

    group.append(input, kbd, button, spinner);
    field.append(label, group);
    // Compile rule: the composite surface paints its own ground. Document
    // rules that target the host element outrank :host styles by spec, so
    // transparent shadow regions would show hostile host paint through.
    const surface = document.createElement('div');
    surface.className = 'bg-background rounded-md p-4';
    surface.append(field);
    root.append(surface);

    this.#blocks.set('query-input', input);
    this.#blocks.set('submit-button', button);
    this.#blocks.set('pending-spinner', spinner);

    // Single rule router per composite root.
    root.addEventListener('rafters:rule', (event) => {
      const { rule, value, valid } = event.detail;
      for (const blockId of ROUTES[rule] ?? []) {
        this.#blocks.get(blockId)?.ruleInput(rule, value, valid);
      }
      if (BOUNDARY.output.includes(rule)) {
        this.dispatchEvent(
          new CustomEvent(rule, { bubbles: true, composed: true, detail: { value, valid } }),
        );
      }
    });
  }

  /** Boundary input `search-pending` as a typed camelCase host property. */
  get searchPending() {
    return this.#blocks.get('pending-spinner')?.active ?? false;
  }
  set searchPending(v) {
    for (const blockId of ROUTES['search-pending'] ?? []) {
      this.#blocks.get(blockId)?.ruleInput('search-pending', Boolean(v), true);
    }
  }
}

// ---------------------------------------------------------------------------
// Knobs - the editor contract. Codegen derives these from boundary inputs
// (host properties) plus the block props the classes vocabulary declares.
// FieldDescriptor-shaped per the kelex/gitpress contract.
// ---------------------------------------------------------------------------
export const KNOBS = [
  { name: 'searchPending', kind: 'boundary-input', type: 'boolean', default: false, description: 'Fed by the host while a search runs; routes to pending-spinner.' },
  { name: 'query-input.placeholder', kind: 'block-prop', type: 'string', default: 'Search components, tokens, patterns...', target: { block: 'query-input', prop: 'placeholder' } },
  { name: 'submit-button.variant', kind: 'block-prop', type: 'enum', options: ['default'], default: 'default', target: { block: 'submit-button', prop: 'variant' }, description: 'Options limited to variants whose classes the compiled sheet covers.' },
];

/** Composite metadata for the editor - carried through from the manifest. */
export const MANIFEST_META = {
  id: 'search-box',
  cognitiveLoad: 4,
  input: BOUNDARY.input,
  output: BOUNDARY.output,
  doNever: {
    do: [
      'Keep the label visible - placeholder-only search fields fail recall',
      'Emit the query upward; never fetch inside the composite',
    ],
    never: [
      'Multiple search boxes competing for attention on one view',
      'Auto-search on every keystroke without a pending affordance',
    ],
  },
};

export function define(sheet) {
  if (SHEET) return;
  SHEET = sheet;
  customElements.define('veneer-label', VeneerLabel);
  customElements.define('veneer-kbd', VeneerKbd);
  customElements.define('veneer-input', VeneerInput);
  customElements.define('veneer-button', VeneerButton);
  customElements.define('veneer-spinner', VeneerSpinner);
  customElements.define('veneer-field', VeneerField);
  customElements.define('veneer-input-group', VeneerInputGroup);
  customElements.define('veneer-search-box', VeneerSearchBox);
}
