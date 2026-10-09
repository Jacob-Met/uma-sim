#!/usr/bin/env python3
"""Bounded independent receiving of the frozen local Uma continuation command."""
from pathlib import Path
import hashlib, json, os, shutil, stat, subprocess, sys, time, traceback

ROOT = Path('/home/jacob/uma-continue-3dc74ef3b04d-root-receiving/receiver-v4')
REPO = Path('/home/jacob/uma-continue-3dc74ef3b04d-local-policy')
BINARY = Path('/home/jacob/uma-continue-3dc74ef3b04d-proof/local-policy-v1/candidate/candidate-uma-sim')
BINARY_SHA = '882cca7cec9081b715ff23b1eef2c79cb8da0facf2e3ef40f4ddf4238248605f'
SEED = 9007199254741123
PINS = {'uma-sim-core/src/bin/uma_sim/continue_run.rs': 'a8268730246e41088f614d10c94ecf40a2426d089cf55e4a305e8388dbefb448', 'uma-sim-core/src/bin/uma-sim.rs': '4201e9d1928177f9fa15ea72fcdadec0e4b2eb5750bfd02be11dbc32dcff14af', 'uma-sim-core/tests/cli_continue.rs': '47f0572cb8356f34a9d4ce17960db6bbf7c37ed23073343c872f0e78cf80b526', 'docs/CLI_CONTINUE.md': '98b816dd7109fda662bb40e355a77df8b12d05d55c9bdc62232b987b46e264ac', 'uma-sim-core/src/engine.rs': '14e464e0abd84f171cc751396c47d939f794a348580a21927f1795b2e57fc928', 'uma-sim-core/src/bot.rs': 'b2286220edfbbb171e06c4fd35118da988e7a9dfaf28b9a5aae2cb5116b37045', 'uma-sim-core/src/session.rs': '95653ec2bc94c22598443c9e2e1c9d0301f2ec0183f0856d8c5baaa88098d637', 'uma-sim-core/src/scenario/mod.rs': '212247478df1cc2211b8f428597bf7f33a1127af407a66a6d64d0c8004ee77ce', 'uma-sim-core/src/training.rs': 'ffcba9a98219819d82d078ac117022afc677562de3512fb915b52c12762c4590', 'uma-sim-core/src/state.rs': '19c65a56d9cc6e2b192b623f9f3232546b8c9a744157bab36db91faffc990833'}

sha = lambda b: hashlib.sha256(b).hexdigest()
def save(name, value):
    (ROOT/name).write_text(json.dumps(value, indent=2) + '\n')
def assertion(condition, message):
    if not condition: raise AssertionError(message)
def file_info(path):
    s = path.lstat()
    return {'kind': 'fifo' if stat.S_ISFIFO(s.st_mode) else 'regular',
            'inode': str(s.st_ino), 'device': str(s.st_dev),
            'mtime_ns': str(s.st_mtime_ns), 'mode': oct(stat.S_IMODE(s.st_mode)),
            'bytes': s.st_size}
def snapshot(path):
    return {'sha256': sha(path.read_bytes()), **file_info(path)}
def session_path(cwd): return cwd/'.uma-sim/session.json'
def read_session(cwd): return json.loads(session_path(cwd).read_bytes())
def directory(name):
    p = ROOT/name
    p.mkdir(exist_ok=False)
    (p/'.uma-sim').mkdir()
    return p

environment = os.environ.copy()
for key in ['UMA_POLICY_CMD', 'UMA_RACE_MODEL', 'LLVM_PROFILE_FILE']:
    environment.pop(key, None)
environment['UMA_REPO_ROOT'] = str(REPO)
commands, cases = [], []
marker = ROOT/'external-policy-invoked.txt'
policy_script = ROOT/'harmless-policy-marker.py'
policy_script.write_text("from pathlib import Path\nPath(" + repr(str(marker)) + ").write_text('unexpected invocation\\n')\nprint('{}')\n")
policy_env = {'UMA_RACE_MODEL': 'physics',
              'UMA_POLICY_CMD': sys.executable + ' ' + str(policy_script)}
def run(label, args, cwd, extra=None, timeout=15):
    env = environment.copy()
    env.update(extra or {})
    started = time.monotonic()
    try:
        result = subprocess.run([str(BINARY)] + args, cwd=cwd, env=env,
                                capture_output=True, text=True, timeout=timeout)
        record = {'label': label, 'command': [str(BINARY)] + args,
                  'cwd': str(cwd), 'explicit_environment': {'UMA_REPO_ROOT': str(REPO), **(extra or {})},
                  'unset_environment': ['LLVM_PROFILE_FILE'] + ([] if extra else ['UMA_POLICY_CMD', 'UMA_RACE_MODEL']),
                  'exit': result.returncode, 'seconds': time.monotonic()-started,
                  'stdout': result.stdout, 'stderr': result.stderr, 'timed_out': False}
    except subprocess.TimeoutExpired as error:
        def decode(value): return value.decode(errors='replace') if isinstance(value, bytes) else value or ''
        record = {'label': label, 'command': [str(BINARY)] + args,
                  'cwd': str(cwd), 'explicit_environment': {'UMA_REPO_ROOT': str(REPO), **(extra or {})},
                  'exit': None, 'seconds': time.monotonic()-started,
                  'stdout': decode(error.stdout), 'stderr': decode(error.stderr), 'timed_out': True}
    commands.append(record)
    save('commands-v1.json', commands)
    return record
def check(name, action):
    try:
        detail = action()
        cases.append({'name': name, 'pass': True, 'detail': detail})
    except Exception as error:
        cases.append({'name': name, 'pass': False, 'error': str(error), 'traceback': traceback.format_exc()})
    save('cases-v1.json', cases)
    return cases[-1]['pass']

assertion(sha(BINARY.read_bytes()) == BINARY_SHA, 'Frozen binary pin mismatch')
for path, expected in PINS.items():
    assertion(sha((REPO/path).read_bytes()) == expected, 'Source pin mismatch: ' + path)
save('intake-v1.json', {'binary': str(BINARY), 'binary_sha256': BINARY_SHA,
                      'source_root': str(REPO), 'source_sha256': PINS,
                      'expected_seed_decimal': str(SEED), 'uid': os.getuid(),
                      'boundary': 'Python integers and exact raw save copies; no JS numeric round trip; no compiler or network.'})
checkpoint = None
checkpoint_bytes = None
checkpoint_stat = None
support_key = None

def author_checkpoint():
    global checkpoint, checkpoint_bytes, checkpoint_stat, support_key
    cwd = directory('authored-checkpoint')
    original = ROOT.parent/'receiver-v2/authored-checkpoint/.uma-sim/session.json'
    checkpoint_bytes = original.read_bytes()
    assertion(sha(checkpoint_bytes) == 'aa52629d8c9b9367ca155f904e647116f47d9f273c709201c8534f84a6924a5b',
              'Pinned actual EVENT checkpoint changed')
    prior_commands = json.loads((ROOT.parent/'receiver-v2/commands-v1.json').read_text())
    steps = [command for command in prior_commands if command['command'][1] == 'step']
    assertion(1 <= len(steps) <= 32 and all(command['exit'] == 0 and not command['timed_out'] for command in steps),
              'Actual existing step transitions were not successful')
    session_path(cwd).write_bytes(checkpoint_bytes)
    current = read_session(cwd)
    assertion(current['meta']['seed'] == SEED and current['rngSeed'] == SEED, 'Large integer seed changed')
    assertion(current['meta']['scenarioId'] == 'unity', 'Unity scenario changed')
    assertion(current['settings']['raceModel'] == 'stub' and current['settings']['speedMultiplier'] == 2,
              'Observed saved stub/speed2 settings changed')
    assertion(current['state']['phase'] in ['EVENT', 'MANDATORY_RACE'], 'Checkpoint has no real pending event/race')
    assertion(not current['state']['careerComplete'], 'Checkpoint already completed')
    slots = current['state']['deck']['slots']
    support_key = 'supportId'
    assertion(len(slots) == 1 and slots[0][support_key] == 'support:10001', 'Qualified support identity changed')
    assertion(isinstance(current['state']['log'], list), 'Observed native log array type changed')
    checkpoint = current
    checkpoint_stat = snapshot(session_path(cwd))
    (ROOT/'checkpoint-original.json').write_bytes(checkpoint_bytes)
    save('REUSED-AUTHORED-CHECKPOINT.json', {
        'source': str(original), 'sha256': sha(checkpoint_bytes), 'source_metadata': snapshot(original),
        'original_start_commands': str(ROOT.parent/'commands-v1.json'),
        'actual_step_commands': steps,
        'boundary': 'Raw checkpoint from actual race/rest/rest transitions following the successful high-seed authored start. No manufactured session or repeated start/reset.'})
    return {'steps': len(steps), 'phase': current['state']['phase'], 'turn': current['state']['turn'],
            'checkpoint': checkpoint_stat, 'rng_seed_decimal': str(current['rngSeed']),
            'rng_calls': current['rngCalls'], 'prior_log_entries': len(current['state']['log']),
            'support_key_observed': support_key, 'support_ids': [slot[support_key] for slot in slots],
            'fixture_adapters': 'Observed speedMultiplier, qualified string supportId and array-prefix log semantics; earlier receiver negatives preserved.'}

completed_default = None
def continuation(policy, hardlink=False):
    global completed_default
    assertion(checkpoint is not None, 'No admitted authored checkpoint; dependent case not run')
    cwd = directory('continue-'+policy)
    path = session_path(cwd)
    path.write_bytes(checkpoint_bytes)
    backup = cwd/'checkpoint-hardlink-backup.json'
    before_backup = None
    if hardlink:
        os.link(path, backup)
        before_backup = snapshot(backup)
        assertion(path.stat().st_ino == backup.stat().st_ino, 'Fixture is not a real hard link')
    result = run('continue-'+policy, ['continue'] if policy == 'default' else ['continue', '--policy=bot'], cwd, policy_env)
    assertion(result['exit'] == 0 and not result['timed_out'], 'Continuation failed: ' + result['stderr'])
    after = read_session(cwd)
    assertion(after['state']['careerComplete'] is True, 'Career did not complete')
    assertion(after['meta'] == checkpoint['meta'], 'Saved top-level metadata changed')
    assertion(after['state']['meta'] == checkpoint['state']['meta'], 'Saved state metadata changed')
    assertion(after['settings'] == checkpoint['settings'], 'Saved settings replaced by environment or CLI defaults')
    assertion(after['meta']['seed'] == SEED and after['rngSeed'] == checkpoint['rngSeed'] == SEED, 'High seed changed')
    assertion(after['rngCalls'] > checkpoint['rngCalls'], 'RNG call checkpoint did not advance')
    assertion(after['rngState'] != checkpoint['rngState'], 'RNG words did not advance')
    assertion(after['state']['turn'] > checkpoint['state']['turn'], 'Career turn did not advance')
    assertion(after['state']['log'][:len(checkpoint['state']['log'])] == checkpoint['state']['log'], 'Prior native log was discarded or rewritten')
    assertion([s[support_key] for s in after['state']['deck']['slots']] ==
              [s[support_key] for s in checkpoint['state']['deck']['slots']], 'Saved deck identities changed')
    assertion(not marker.exists(), 'External policy marker was invoked')
    assertion((ROOT/'checkpoint-original.json').read_bytes() == checkpoint_bytes, 'Original checkpoint evidence changed')
    assertion(snapshot(session_path(ROOT/'authored-checkpoint')) == checkpoint_stat, 'Authored checkpoint inode/time/bytes changed')
    detail = {'turn_before': checkpoint['state']['turn'], 'turn_after': after['state']['turn'],
              'seed_decimal': str(after['rngSeed']), 'rng_calls_before': checkpoint['rngCalls'],
              'rng_calls_after': after['rngCalls'], 'completed': True,
              'meta_and_settings_retained': True, 'prior_log_prefix_retained': True,
              'external_policy_invocations': 0, 'save': snapshot(path)}
    if hardlink:
        assertion(snapshot(backup) == before_backup, 'Continuation mutated the hard-linked original backup')
        assertion(backup.read_bytes() == checkpoint_bytes, 'Backup checkpoint bytes changed')
        assertion(path.stat().st_ino != backup.stat().st_ino, 'Writer did not publish a separate inode')
        detail['hardlink_backup_before_after'] = before_backup
        detail['replacement_inode_differs_from_backup'] = True
    if policy == 'default': completed_default = path.read_bytes()
    return detail

def completed_noop():
    assertion(completed_default is not None, 'No completed default career; dependent case not run')
    cwd = directory('completed-readonly')
    path = session_path(cwd)
    path.write_bytes(completed_default)
    path.chmod(0o444)
    before = snapshot(path)
    result = run('completed-readonly-noop', ['continue', '--policy=bot'], cwd, policy_env)
    assertion(result['exit'] == 0 and not result['timed_out'], 'Completed no-op failed')
    assertion('already complete' in result['stdout'].lower(), 'No completed no-op acknowledgement')
    assertion(snapshot(path) == before, 'Completed save bytes/inode/mtime/mode changed')
    assertion(not marker.exists(), 'No-op invoked external policy')
    return {'before_after': before, 'stdout': result['stdout'], 'exact_noop': True}

def parse_before_fifo_load():
    cwd = directory('parser-fifo')
    fifo = session_path(cwd)
    os.mkfifo(fifo, 0o600)
    before = file_info(fifo)
    observations = []
    errors = []
    variants = [
        (['continue', '--help'], 0),
        (['continue', '--policy=external'], 2),
        (['continue', '--policy', 'bot'], 2),
        (['continue', '--policy=default', '--policy=bot'], 2),
        (['continue', '--seed=1'], 2),
        (['continue', '--race-model=physics'], 2),
        (['continue', '--help', '--policy=bot'], 2),
    ]
    try:
        for index, (args, expected) in enumerate(variants):
            result = run('fifo-parser-'+str(index+1), args, cwd, policy_env, timeout=2)
            same = file_info(fifo) == before
            observation = {'args': args, 'expected_exit': expected, 'actual_exit': result['exit'],
                           'timed_out': result['timed_out'], 'fifo_metadata_unchanged': same}
            observations.append(observation)
            if result['exit'] != expected or result['timed_out'] or not same:
                errors.append(observation)
        assertion(not marker.exists(), 'Parser invoked external policy')
    finally:
        save('fifo-metadata-v1.json', {'before': before, 'after': file_info(fifo),
              'observations': observations, 'removal_scope': 'Only this owned empty FIFO after metadata capture.'})
        fifo.unlink()
    assertion(not errors, 'Parser read/hung on FIFO or changed outcome: ' + json.dumps(errors))
    return {'variants': len(variants), 'all_parse_before_load': True,
            'fifo_metadata': before, 'owned_empty_fifo_removed_after_recording': True}

check('authored large-i64 unity pending checkpoint with support10001 and saved stub/speed2', author_checkpoint)
check('default continuation preserves checkpoint identity/settings/history and hard-linked backup', lambda: continuation('default', True))
check('bot continuation preserves checkpoint settings/history and ignores external-policy environment', lambda: continuation('bot'))
check('read-only completed save is an exact byte inode mtime no-op', completed_noop)
check('help and invalid continuation options finish before loading an owned FIFO', parse_before_fifo_load)

for path, expected in PINS.items():
    assertion(sha((REPO/path).read_bytes()) == expected, 'Source changed during receiving: ' + path)
assertion(sha(BINARY.read_bytes()) == BINARY_SHA, 'Binary changed during receiving')
regular_bytes = sum(p.stat().st_size for p in ROOT.rglob('*') if p.is_file())
receipt = {'actor': 'hamon-3dc74ef3b04d-20261008/estate_coordination',
           'kind': 'independent native Uma continuation receiving',
           'binary_sha256': BINARY_SHA, 'source_sha256': PINS,
           'expected_seed_decimal': str(SEED), 'integer_handling': 'Python arbitrary-precision integers; raw checkpoint copies.',
           'passed': sum(c['pass'] for c in cases), 'failed': sum(not c['pass'] for c in cases),
           'cases': cases, 'command_count': len(commands), 'artifact_bytes_before_receipt': regular_bytes,
           'artifact_budget_bytes': 2*1024*1024,
           'compiler_invoked': False, 'network_invoked': False, 'product_mutations': False,
           'policy_marker_exists': marker.exists(),
           'limits': ['No equality claim for complete outcomes across fresh processes; native factor HashMap nondeterminism retained.',
                      'Continuation cases begin from copies of one real authored pending checkpoint.',
                      'The original author24-case suite was not rerun.']}
if regular_bytes >= 2*1024*1024:
    receipt['artifact_budget_failure'] = True
    receipt['failed'] += 1
save('RECEIPT-v1.json', receipt)
print(json.dumps({'passed': receipt['passed'], 'failed': receipt['failed'],
                  'commands': len(commands), 'artifact_bytes': regular_bytes,
                  'receipt_sha256': sha((ROOT/'RECEIPT-v1.json').read_bytes()),
                  'failed_cases': [{'name': c['name'], 'error': c.get('error')} for c in cases if not c['pass']]}))
sys.exit(1 if receipt['failed'] else 0)
