#!/usr/bin/env python3
"""Materialize, but never execute, the unchanged V3 receiver for a reviewed candidate."""
from pathlib import Path
import argparse, hashlib, json, re
ROOT = Path('/home/jacob/uma-continue-3dc74ef3b04d-root-receiving')
SOURCE = ROOT/'receiver-v3/receive-continue-v3.py'
EXPECTED_SOURCE_SHA = 'b01b3358258c3e9cecb34844df2b300f55d982a096f1d9c028379ac47164ce37'
def digest(path):
    h = hashlib.sha256()
    with path.open('rb') as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b''): h.update(chunk)
    return h.hexdigest()
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary', required=True)
parser.add_argument('--binary-sha256', required=True)
parser.add_argument('--source-root', required=True)
parser.add_argument('--source-pins-json', required=True,
                    help='JSON object mapping source-relative paths to reviewed SHA256 values.')
parser.add_argument('--output-directory', required=True,
                    help='A new direct child receiver-v4* directory under the existing receiving root.')
args = parser.parse_args()
output = Path(args.output_directory).absolute()
candidate = Path(args.source_root).absolute()
binary = Path(args.binary).absolute()
if output.parent != ROOT or not output.name.startswith('receiver-v4') or output.exists():
    raise SystemExit('Choose a fresh receiver-v4* direct child of the existing receiving root.')
if digest(SOURCE) != EXPECTED_SOURCE_SHA:
    raise SystemExit('Frozen V3 receiver changed.')
if not re.fullmatch('[0-9a-f]{64}', args.binary_sha256) or digest(binary) != args.binary_sha256:
    raise SystemExit('Reviewed binary pin mismatch.')
pins = json.loads(Path(args.source_pins_json).read_text())
required = {'uma-sim-core/src/bin/uma_sim/continue_run.rs',
            'uma-sim-core/src/bin/uma-sim.rs', 'docs/CLI_CONTINUE.md',
            'uma-sim-core/tests/cli_continue.rs'}
if not isinstance(pins, dict) or not required.issubset(pins):
    raise SystemExit('The reviewed source pins must include all original product paths.')
for relative, expected in pins.items():
    path = Path(relative)
    if path.is_absolute() or '..' in path.parts or not re.fullmatch('[0-9a-f]{64}', expected):
        raise SystemExit('Invalid source-relative pin.')
    if digest(candidate/path) != expected:
        raise SystemExit('Reviewed source pin mismatch: ' + relative)
original = SOURCE.read_text()
changed = original
assignments = {
    "ROOT = Path('/home/jacob/uma-continue-3dc74ef3b04d-root-receiving/receiver-v3')":
        'ROOT = Path(' + repr(str(output)) + ')',
    "REPO = Path('/home/jacob/uma-continue-3dc74ef3b04d-candidate')":
        'REPO = Path(' + repr(str(candidate)) + ')',
    "BINARY = Path('/home/jacob/uma-continue-3dc74ef3b04d-proof/scoped-v2/candidate-uma-sim')":
        'BINARY = Path(' + repr(str(binary)) + ')',
    "BINARY_SHA = 'db18c2878f4627ed14d247ed26e1e94327a3d14b6aadc600d407c3292f67ab4a'":
        'BINARY_SHA = ' + repr(args.binary_sha256),
}
for before, after in assignments.items():
    if changed.count(before) != 1:
        raise SystemExit('Expected constant assignment changed in frozen method.')
    changed = changed.replace(before, after)
start = changed.index('PINS = {')
end = changed.index('\nsha = lambda b:', start)
changed = changed[:start] + 'PINS = ' + repr(pins) + '\n' + changed[end:]
body = original[original.index('sha = lambda b:'):]
assert changed[changed.index('sha = lambda b:'):] == body
assert 'SEED = 9007199254741123' in changed
assert (ROOT/'receiver-v2/authored-checkpoint/.uma-sim/session.json').is_file()
assert digest(ROOT/'receiver-v2/authored-checkpoint/.uma-sim/session.json') == 'aa52629d8c9b9367ca155f904e647116f47d9f273c709201c8534f84a6924a5b'
output.mkdir()
method = output/'receive-continue-v4.py'
method.write_text(changed)
receipt = {'frozen_v3_sha256': EXPECTED_SOURCE_SHA,
           'new_method_sha256': digest(method),
           'unchanged_primary_body_sha256': hashlib.sha256(body.encode()).hexdigest(),
           'allowed_changes': ['candidate binary path/hash', 'candidate source root/path hashes', 'fresh output directory'],
           'substantive_assertions_unchanged': True,
           'checkpoint_sha256': 'aa52629d8c9b9367ca155f904e647116f47d9f273c709201c8534f84a6924a5b',
           'binary': str(binary), 'binary_sha256': args.binary_sha256,
           'source_root': str(candidate), 'source_sha256': pins,
           'executed': False,
           'boundary': 'Materialization only. Run only after independent source review and exact repaired-binary authorization.'}
(output/'METHOD-ADAPTER.json').write_text(json.dumps(receipt, indent=2)+'\n')
print(json.dumps({'method': str(method), 'sha256': digest(method), 'executed': False}))
