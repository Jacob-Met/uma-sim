// Controlled JSX/hook/native-dialog protocol. This is not React or a browser.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import vm from 'node:vm';
const require = createRequire(import.meta.url);
const ts = require(process.env.UMA_TYPESCRIPT_MODULE || 'typescript');

export function controlledComponent(path, initialOpen = false) {
  let cursor = 0;
  let mountedNode = null;
  const slots = [];
  const effects = [];
  const calls = [];
  const dialog = {
    open: initialOpen,
    showModal() { assert.equal(this.open, false); this.open = true; calls.push('showModal'); },
    close() { assert.equal(this.open, true); this.open = false; calls.push('close'); },
  };
  const hooks = {
    useRef(value) {
      const i = cursor++;
      if (!slots[i]) slots[i] = { kind: 'ref', value: { current: value } };
      assert.equal(slots[i].kind, 'ref');
      return slots[i].value;
    },
    useId() {
      const i = cursor++;
      if (!slots[i]) slots[i] = { kind: 'id', value: `controlled-id-${i}` };
      assert.equal(slots[i].kind, 'id');
      return slots[i].value;
    },
    useLayoutEffect(setup, dependencies) {
      const i = cursor++;
      if (!slots[i]) slots[i] = { kind: 'effect', deps: undefined, cleanup: undefined, setup: undefined };
      const slot = slots[i];
      assert.equal(slot.kind, 'effect');
      const changed = !slot.deps || dependencies.length !== slot.deps.length || dependencies.some((x, j) => !Object.is(x, slot.deps[j]));
      if (changed) effects.push({ slot, setup, deps: [...dependencies] });
    },
  };
  const jsx = (type, props, key) => ({ type, props: props || {}, key });
  const source = readFileSync(path, 'utf8');
  const compiled = ts.transpileModule(source, {
    fileName: path, reportDiagnostics: true,
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX },
  });
  assert.deepEqual((compiled.diagnostics || []).filter(x => x.category === ts.DiagnosticCategory.Error), []);
  const exports = {};
  const sandbox = {
    exports,
    require(name) {
      if (name === 'react') return hooks;
      if (name === 'react/jsx-runtime') return { jsx, jsxs: jsx };
      throw new Error(`Unapproved component import: ${name}`);
    },
  };
  vm.runInNewContext(compiled.outputText, sandbox, { filename: path, timeout: 1000 });
  assert.equal(typeof exports.EventDialog, 'function');
  function cleanup(slot) {
    const fn = slot.cleanup;
    slot.cleanup = undefined;
    if (typeof fn === 'function') fn();
  }
  return {
    calls, dialog,
    render(props) {
      cursor = 0;
      assert.equal(effects.length, 0);
      const node = exports.EventDialog(props);
      // Layout cleanup precedes ref detachment; new refs precede setup.
      for (const entry of effects) cleanup(entry.slot);
      if (mountedNode?.props.ref && mountedNode !== node) mountedNode.props.ref.current = null;
      if (node?.props.ref) node.props.ref.current = dialog;
      mountedNode = node;
      for (const { slot, setup, deps } of effects.splice(0)) {
        slot.setup = setup; slot.deps = deps; slot.cleanup = setup();
      }
      return node;
    },
    cleanupLayoutEffects() {
      for (const slot of slots) if (slot?.kind === 'effect') cleanup(slot);
    },
    setupLayoutEffects() {
      for (const slot of slots) if (slot?.kind === 'effect' && slot.setup) slot.cleanup = slot.setup();
    },
    replayLayoutEffects() {
      for (const slot of slots) if (slot?.kind === 'effect') cleanup(slot);
      for (const slot of slots) if (slot?.kind === 'effect' && slot.setup) slot.cleanup = slot.setup();
    },
    unmount() {
      for (const slot of slots) if (slot?.kind === 'effect') cleanup(slot);
      if (mountedNode?.props.ref) mountedNode.props.ref.current = null;
      mountedNode = null;
    },
  };
}

export function descendants(node, type) {
  const result = [];
  function visit(value) {
    if (Array.isArray(value)) return value.forEach(visit);
    if (value === null || typeof value !== 'object') return;
    if (value.type === type) result.push(value);
    visit(value.props?.children);
  }
  visit(node);
  return result;
}

import test from 'node:test';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

// BEGIN EXACT INDEPENDENT KEY
const KEY = {
  "schema": "uma.event-dialog.independent-expectations/1",
  "author": "e1438c9d058f/machine_worker",
  "contract": {
    "path": "/workspace/scratch/e1438c9d058f/uma-event-dialog/CONTRACT.md",
    "bytes": 8092,
    "sha256": "d0e9aa26297e98b10d772cf50b842b78f8b6fbf240592df2ac56bbea9bab8995"
  },
  "original_component": {
    "path": "/workspace/scratch/e1438c9d058f/uma-event-dialog/original/packages/uma-sim-ui/src/components/EventDialog.tsx",
    "bytes": 1018,
    "sha256": "ab3fc0b940f1795b1020fb19d1f8108b5111d35910e9ba534d0679949036b831"
  },
  "qualification": "Literal data frozen before candidate exposure. No component, compiler, React, DOM, browser or product invocation by this author. Future protocol tests do not establish native focus, inertness, Tab routing, restoration or actual React integration.",
  "inputs": {
    "missing": {
      "phase": "TRAINING",
      "awaitingChoice": false,
      "sentinel": {
        "turn": 7,
        "unchanged": [
          "雪",
          null
        ]
      }
    },
    "empty-awaiting": {
      "phase": "TRAINING",
      "awaitingChoice": true,
      "pendingEventOptions": [],
      "pendingEventTitle": "Pending without options"
    },
    "empty-event": {
      "phase": "eVeNt",
      "awaitingChoice": false,
      "pendingEventOptions": [],
      "pendingEventTitle": "Empty event"
    },
    "null-options": {
      "phase": "EVENT",
      "awaitingChoice": true,
      "pendingEventOptions": null,
      "pendingEventTitle": "Null options"
    },
    "one-by-options": {
      "phase": "TRAINING",
      "awaitingChoice": false,
      "pendingEventOptions": [
        "Keep going"
      ],
      "pendingEventTitle": "One choice",
      "sentinel": {
        "doNotAlter": [
          1,
          2
        ]
      }
    },
    "two-awaiting": {
      "phase": "REST",
      "awaitingChoice": true,
      "pendingEventOptions": [
        "First",
        "Second"
      ],
      "pendingEventTitle": "Choose deliberately"
    },
    "event-fallback": {
      "phase": "event",
      "awaitingChoice": false,
      "pendingEventOptions": [
        "First"
      ],
      "pendingEventTitle": null
    },
    "missing-title": {
      "phase": "TRAINING",
      "awaitingChoice": false,
      "pendingEventOptions": [
        "Only"
      ]
    },
    "empty-title": {
      "phase": "EVENT",
      "awaitingChoice": false,
      "pendingEventOptions": [
        "A"
      ],
      "pendingEventTitle": ""
    },
    "literal-duplicates": {
      "phase": "EVENT",
      "awaitingChoice": true,
      "pendingEventTitle": "<b>雪 & \"choice\"</b>\nNext",
      "pendingEventOptions": [
        "Same",
        "Same",
        "<img src=x> & 雪\n\"literal\""
      ],
      "sentinel": {
        "untouched": {
          "ids": [
            4,
            0,
            4
          ]
        }
      }
    }
  },
  "render_cases": [
    {
      "input": "missing",
      "busy": false,
      "expected": null
    },
    {
      "input": "empty-awaiting",
      "busy": false,
      "expected": null
    },
    {
      "input": "empty-event",
      "busy": true,
      "expected": null
    },
    {
      "input": "null-options",
      "busy": false,
      "expected": null
    },
    {
      "input": "one-by-options",
      "busy": false,
      "expected": {
        "title": "One choice",
        "labels": [
          "Keep going"
        ],
        "callback_ids": [
          "event_0"
        ],
        "disabled": [
          false
        ],
        "status": null
      }
    },
    {
      "input": "two-awaiting",
      "busy": true,
      "expected": {
        "title": "Choose deliberately",
        "labels": [
          "First",
          "Second"
        ],
        "callback_ids": [
          "event_0",
          "event_1"
        ],
        "disabled": [
          true,
          true
        ],
        "status": {
          "tag": "p",
          "role": "status",
          "text": "Working…"
        }
      }
    },
    {
      "input": "event-fallback",
      "busy": false,
      "expected": {
        "title": "Choose an option",
        "labels": [
          "First"
        ],
        "callback_ids": [
          "event_0"
        ],
        "disabled": [
          false
        ],
        "status": null
      }
    },
    {
      "input": "missing-title",
      "busy": false,
      "expected": {
        "title": "Choose an option",
        "labels": [
          "Only"
        ],
        "callback_ids": [
          "event_0"
        ],
        "disabled": [
          false
        ],
        "status": null
      }
    },
    {
      "input": "empty-title",
      "busy": false,
      "expected": {
        "title": "",
        "labels": [
          "A"
        ],
        "callback_ids": [
          "event_0"
        ],
        "disabled": [
          false
        ],
        "status": null
      }
    },
    {
      "input": "literal-duplicates",
      "busy": false,
      "expected": {
        "title": "<b>雪 & \"choice\"</b>\nNext",
        "labels": [
          "Same",
          "Same",
          "<img src=x> & 雪\n\"literal\""
        ],
        "callback_ids": [
          "event_0",
          "event_1",
          "event_2"
        ],
        "disabled": [
          false,
          false,
          false
        ],
        "status": null
      }
    }
  ],
  "visible_markup": {
    "root": "dialog",
    "required_class": "modal",
    "additional_event_class": true,
    "jsx_open_attribute": false,
    "ordinary_backdrop_divs": 0,
    "heading_tag": "h2",
    "heading_text": "Event",
    "stable_generated_heading_id": true,
    "aria_labelledby_equals_heading_id": true,
    "title_tag": "p",
    "choice_container_class": "choice-list",
    "button_class": "event",
    "dangerouslySetInnerHTML": false,
    "keydown_listener": false
  },
  "lifecycle_timelines": [
    {
      "id": "hidden-visible-updates-end-reopen",
      "steps": [
        {
          "action": "render missing/false; commit",
          "show": 0,
          "close": 0,
          "open": false,
          "rendered": false
        },
        {
          "action": "render one-by-options/false; attach closed dialog; commit",
          "show": 1,
          "close": 0,
          "open": true,
          "rendered": true
        },
        {
          "action": "render two-awaiting/true with new callback; commit same node",
          "show": 1,
          "close": 0,
          "open": true,
          "rendered": true,
          "status": "Working…"
        },
        {
          "action": "render literal-duplicates/false with latest callback; commit same node",
          "show": 1,
          "close": 0,
          "open": true,
          "rendered": true,
          "status": null
        },
        {
          "action": "invoke current option index 1",
          "show": 1,
          "close": 0,
          "open": true,
          "callback_log": [
            [
              "latest",
              "event_1"
            ]
          ]
        },
        {
          "action": "render empty-event/false; commit visibility cleanup before removal",
          "show": 1,
          "close": 1,
          "open": false,
          "rendered": false
        },
        {
          "action": "render two-awaiting/false; attach new closed node; commit",
          "show": 2,
          "close": 1,
          "open": true,
          "rendered": true
        },
        {
          "action": "unmount owned dialog",
          "show": 2,
          "close": 2,
          "open": false
        },
        {
          "action": "repeat the same cleanup",
          "show": 2,
          "close": 2,
          "open": false
        }
      ]
    },
    {
      "id": "already-open",
      "steps": [
        {
          "action": "render visible; attach already-open dialog; commit",
          "show": 0,
          "close": 0,
          "open": true
        },
        {
          "action": "update title/busy/options/callback; commit same visible node",
          "show": 0,
          "close": 0,
          "open": true
        },
        {
          "action": "unmount",
          "show": 0,
          "close": 1,
          "open": false
        },
        {
          "action": "repeat cleanup",
          "show": 0,
          "close": 1,
          "open": false
        }
      ]
    },
    {
      "id": "development-effect-replay",
      "steps": [
        {
          "action": "setup with attached closed dialog",
          "show": 1,
          "close": 0,
          "open": true
        },
        {
          "action": "development cleanup",
          "show": 1,
          "close": 1,
          "open": false
        },
        {
          "action": "development setup again on same node",
          "show": 2,
          "close": 1,
          "open": true
        },
        {
          "action": "unmount cleanup",
          "show": 2,
          "close": 2,
          "open": false
        }
      ]
    },
    {
      "id": "native-closed-before-cleanup",
      "steps": [
        {
          "action": "setup closed dialog",
          "show": 1,
          "close": 0,
          "open": true
        },
        {
          "action": "protocol marks native node already closed without calling component",
          "show": 1,
          "close": 0,
          "open": false
        },
        {
          "action": "unmount cleanup",
          "show": 1,
          "close": 0,
          "open": false
        }
      ]
    },
    {
      "id": "required-choice-cancel",
      "steps": [
        {
          "action": "mount literal-duplicates busy=false",
          "show": 1,
          "close": 0,
          "open": true,
          "prevent_default": 0,
          "callback_log": []
        },
        {
          "action": "dispatch native cancel",
          "show": 1,
          "close": 0,
          "open": true,
          "prevent_default": 1,
          "callback_log": []
        },
        {
          "action": "update busy=true; dispatch cancel again",
          "show": 1,
          "close": 0,
          "open": true,
          "prevent_default": 2,
          "callback_log": [],
          "status": "Working…"
        },
        {
          "action": "update busy=false",
          "show": 1,
          "close": 0,
          "open": true,
          "prevent_default": 2,
          "callback_log": [],
          "status": null
        }
      ]
    }
  ],
  "callback_cases": [
    {
      "input": "literal-duplicates",
      "indices": [
        2,
        0,
        1
      ],
      "expected_ids": [
        "event_2",
        "event_0",
        "event_1"
      ]
    },
    {
      "action": "replace callback A with B while visible, invoke current option 0",
      "expected": {
        "A": [],
        "B": [
          "event_0"
        ]
      }
    }
  ],
  "conservation": {
    "each_input_object": "complete deep equality before/after every render/effect/callback/cleanup",
    "onChoose": "only explicit choice callback activation; original event strings and order exact",
    "busy": "all options disabled; synthetic handler invocation is not a browser-disabled-button test",
    "forbidden_effects": [
      "network",
      "API/client",
      "storage",
      "timers",
      "downloads",
      "file access",
      "career mutation"
    ],
    "css": "exact old stylesheet prefix; only event dialog and its native backdrop appended; old backdrop color preserved"
  },
  "publication": "Unreferenced source commit and repair kit only; current unrestricted-push workflow forbids even a branch/tag publication. No PR/main/Actions/installed adoption."
};
// END EXACT INDEPENDENT KEY

const componentPath = resolve(dirname(fileURLToPath(import.meta.url)), '../src/components/EventDialog.tsx');
function deepFreeze(value) {
  if (value && typeof value === 'object') { Object.values(value).forEach(deepFreeze); Object.freeze(value); }
  return value;
}
deepFreeze(KEY);
const conserved = JSON.stringify(KEY.inputs);
function input(name) { return KEY.inputs[name]; }
function props(name, busy = false, onChoose = () => {}) { return { state: input(name), busy, onChoose }; }
function statuses(node) { return descendants(node, 'p').filter(x => x.props.role === 'status'); }
function assertConserved() { assert.equal(JSON.stringify(KEY.inputs), conserved); }
function assertVisible(node, expected) {
  assert.equal(node.type, KEY.visible_markup.root);
  const classes = node.props.className.split(/\s+/);
  assert.ok(classes.includes('modal') && classes.includes('event-modal'));
  assert.equal(Object.hasOwn(node.props, 'open'), false);
  assert.equal(descendants(node, 'div').filter(x => x.props.className === 'modal-backdrop').length, 0);
  const headings = descendants(node, 'h2');
  assert.equal(headings.length, 1);
  assert.equal(headings[0].props.children, 'Event');
  assert.equal(typeof headings[0].props.id, 'string');
  assert.ok(headings[0].props.id.length > 0);
  assert.equal(node.props['aria-labelledby'], headings[0].props.id);
  const ps = descendants(node, 'p').filter(x => x.props.role !== 'status');
  assert.equal(ps.length, 1); assert.equal(ps[0].props.children, expected.title);
  const containers = descendants(node, 'div').filter(x => x.props.className === 'choice-list');
  assert.equal(containers.length, 1);
  const buttons = descendants(containers[0], 'button');
  assert.deepEqual(buttons.map(x => x.props.children), expected.labels);
  assert.deepEqual(buttons.map(x => x.props.disabled), expected.disabled);
  assert.ok(buttons.every(x => x.props.className === 'event'));
  assert.deepEqual(statuses(node).map(x => ({ tag: x.type, role: x.props.role, text: x.props.children })), expected.status ? [expected.status] : []);
  function noRaw(value) {
    if (Array.isArray(value)) return value.forEach(noRaw);
    if (!value || typeof value !== 'object') return;
    assert.equal(Object.hasOwn(value.props, 'dangerouslySetInnerHTML'), false);
    assert.equal(Object.hasOwn(value.props, 'onKeyDown'), false);
    noRaw(value.props.children);
  }
  noRaw(node); assertConserved();
}
function tick(h, expected, node, callbacks, prevented) {
  assert.equal(h.calls.filter(x => x === 'showModal').length, expected.show, expected.action);
  assert.equal(h.calls.filter(x => x === 'close').length, expected.close, expected.action);
  assert.equal(h.dialog.open, expected.open, expected.action);
  if (Object.hasOwn(expected, 'rendered')) assert.equal(node !== null, expected.rendered);
  if (Object.hasOwn(expected, 'status')) assert.deepEqual(statuses(node).map(x => x.props.children), expected.status === null ? [] : [expected.status]);
  if (Object.hasOwn(expected, 'callback_log')) assert.deepEqual(callbacks, expected.callback_log);
  if (Object.hasOwn(expected, 'prevent_default')) assert.equal(prevented, expected.prevent_default);
  assertConserved();
}

test('controlled component: ten literal visibility, text, identity and busy cases', () => {
  for (const c of KEY.render_cases) {
    const h = controlledComponent(componentPath); const routed = [];
    const node = h.render(props(c.input, c.busy, id => routed.push(id)));
    if (c.expected === null) { assert.equal(node, null); assert.deepEqual(h.calls, []); }
    else {
      assertVisible(node, c.expected); assert.deepEqual(h.calls, ['showModal']);
      // Calling a disabled prop handler here establishes mapping only, not a browser click.
      descendants(node, 'button').forEach(button => button.props.onClick());
      assert.deepEqual(routed, c.expected.callback_ids);
    }
    h.unmount(); assertConserved();
  }
});

test('controlled component: visibility lifetime survives busy, title, options and callback updates', () => {
  const steps = KEY.lifecycle_timelines.find(x => x.id === 'hidden-visible-updates-end-reopen').steps;
  const h = controlledComponent(componentPath); const routed = []; let node;
  node = h.render(props('missing')); tick(h, steps[0], node, routed);
  node = h.render(props('one-by-options')); tick(h, steps[1], node, routed);
  const headingId = descendants(node, 'h2')[0].props.id;
  node = h.render(props('two-awaiting', true, id => routed.push(['intermediate', id]))); tick(h, steps[2], node, routed);
  node = h.render(props('literal-duplicates', false, id => routed.push(['latest', id]))); tick(h, steps[3], node, routed);
  assert.equal(descendants(node, 'h2')[0].props.id, headingId);
  descendants(node, 'button')[1].props.onClick(); tick(h, steps[4], node, routed);
  node = h.render(props('empty-event')); tick(h, steps[5], node, routed);
  node = h.render(props('two-awaiting')); tick(h, steps[6], node, routed);
  h.unmount(); tick(h, steps[7], node, routed);
  h.unmount(); tick(h, steps[8], node, routed);
});

test('controlled component: cleanup owns already-open and already-closed native nodes', () => {
  const already = KEY.lifecycle_timelines.find(x => x.id === 'already-open').steps;
  const h = controlledComponent(componentPath, true); let node = h.render(props('one-by-options'));
  tick(h, already[0], node);
  node = h.render(props('literal-duplicates', true, () => {})); tick(h, already[1], node);
  h.unmount(); tick(h, already[2], node);
  h.unmount(); tick(h, already[3], node);
  const closed = KEY.lifecycle_timelines.find(x => x.id === 'native-closed-before-cleanup').steps;
  const g = controlledComponent(componentPath); node = g.render(props('two-awaiting')); tick(g, closed[0], node);
  g.dialog.open = false; tick(g, closed[1], node);
  g.unmount(); tick(g, closed[2], node);
});

test('controlled component: development layout-effect setup cleanup setup remains balanced', () => {
  const steps = KEY.lifecycle_timelines.find(x => x.id === 'development-effect-replay').steps;
  const h = controlledComponent(componentPath); const node = h.render(props('two-awaiting'));
  tick(h, steps[0], node);
  h.cleanupLayoutEffects(); tick(h, steps[1], node);
  h.setupLayoutEffects(); tick(h, steps[2], node);
  h.unmount(); tick(h, steps[3], node);
});

test('controlled component: cancel prevents dismissal without choosing and busy feedback remains visible', () => {
  const steps = KEY.lifecycle_timelines.find(x => x.id === 'required-choice-cancel').steps;
  const h = controlledComponent(componentPath); const routed = []; let prevented = 0;
  const onChoose = id => routed.push(id); const event = { preventDefault() { prevented++; } };
  let node = h.render(props('literal-duplicates', false, onChoose)); tick(h, steps[0], node, routed, prevented);
  node.props.onCancel(event); tick(h, steps[1], node, routed, prevented);
  node = h.render(props('literal-duplicates', true, onChoose)); node.props.onCancel(event); tick(h, steps[2], node, routed, prevented);
  assert.ok(descendants(node, 'button').every(x => x.props.disabled));
  node = h.render(props('literal-duplicates', false, onChoose)); tick(h, steps[3], node, routed, prevented);
  assert.ok(descendants(node, 'button').every(x => !x.props.disabled));
  h.unmount(); assertConserved();
});

test('controlled component: duplicate options preserve ordinals and current callback ownership', () => {
  const h = controlledComponent(componentPath); const routed = [];
  let node = h.render(props('literal-duplicates', false, id => routed.push(id)));
  const c = KEY.callback_cases[0];
  for (const i of c.indices) descendants(node, 'button')[i].props.onClick();
  assert.deepEqual(routed, c.expected_ids);
  const logs = { A: [], B: [] };
  node = h.render(props('one-by-options', false, id => logs.A.push(id)));
  node = h.render(props('one-by-options', false, id => logs.B.push(id)));
  descendants(node, 'button')[0].props.onClick();
  assert.deepEqual(logs, KEY.callback_cases[1].expected);
  h.unmount(); assertConserved();
});
