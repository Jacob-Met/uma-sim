#!/usr/bin/env python3
"""Bounded independent receiving of the frozen local Uma continuation command."""
from pathlib import Path
import hashlib, json, os, shutil, stat, subprocess, sys, time, traceback

ROOT = Path('/home/jacob/uma-continue-3dc74ef3b04d-root-receiving')
REPO = Path('/home/jacob/uma-continue-3dc74ef3b04d-candidate')
BINARY = Path('/home/jacob/uma-continue-3dc74ef3b04d-proof/scoped-v2/candidate-uma-sim')
BINARY_SHA = 'db18c2878f4627ed14d247ed26e1e94327a3d14b6aadc600d407c3292f67ab4a'
SEED = 9007199254741123
PINS = {
    'uma-sim-core/src/bin/uma_sim/continue_run.rs': '26caa3ab1c6253f21a4ee4ea77c2a0e8df6de35c0635baf3f379c7a8d4feb17a',
    'uma-sim-core/src/bin/uma-sim.rs': '4201e9d1928177f9fa15ea72fcdadec0e4b2eb5750bfd02be11dbc32dcff14af',
    'docs/CLI_CONTINUE.md': '72a8ef899afaf4adec856361e9e75c73d80c1a4b547654b80cb3c116671f3e9a',
    'uma-sim-core/tests/cli_continue.rs': 'cc6f6a63990c6464f3fc0903785eb28e1d0466346124aa687aa5429e135104e7',
}
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
    result = run('start-authored-career',
                 ['start', '--seed='+str(SEED), '--scenario=unity', '--race-model=stub',
                  '--speed=2', '--dialogue=off', '--deck=support:10001@speed:85', '--trace-rng'], cwd)
    assertion(result['exit'] == 0 and not result['timed_out'], 'Authored start failed: ' + result['stderr'])
    initial = read_session(cwd)
    save('initial-observation.json', {'meta': initial['meta'], 'settings': initial['settings'],
          'state_phase': initial['state']['phase'], 'state_turn': initial['state']['turn'],
          'deck_slot_keys': [list(slot) for slot in initial['state']['deck']['slots']],
          'rng_seed_decimal': str(initial['rngSeed']), 'rng_calls': initial['rngCalls']})
    assertion(initial['meta']['seed'] == SEED and initial['rngSeed'] == SEED, 'Large integer seed was changed at start')
    assertion(initial['meta']['scenarioId'] == 'unity', 'Requested unity scenario was not stored')
    settings_text = json.dumps(initial['settings']).lower()
    assertion('stub' in settings_text, 'Saved race model did not record stub')
    assertion(initial['settings'].get('speed') == 2, 'Saved speed is not the requested2')
    steps = 0
    current = initial
    while current['state']['phase'] not in ['EVENT', 'MANDATORY_RACE'] and steps < 32:
        phase = current['state']['phase']
        action = 'event_0' if phase == 'EVENT' else 'race' if phase == 'MANDATORY_RACE' else 'rest'
        result = run('checkpoint-step-'+str(steps+1), ['step', action], cwd)
        assertion(result['exit'] == 0 and not result['timed_out'], 'Existing step command failed')
        current = read_session(cwd)
        steps += 1
    assertion(current['state']['phase'] in ['EVENT', 'MANDATORY_RACE'], 'No real pending event/race within32 authored steps')
    assertion(not current['state']['careerComplete'], 'Checkpoint already completed')
    slots = current['state']['deck']['slots']
    assertion(len(slots) == 1, 'Authored deck slot count changed')
    support_key = next((key for key in ['supportId', 'support_id'] if key in slots[0]), None)
    assertion(support_key is not None and slots[0][support_key] == 10001, 'Authored support identity is not10001')
    assertion(isinstance(current['state']['log'], str), 'Native prior log is not text')
    checkpoint = current
    checkpoint_bytes = session_path(cwd).read_bytes()
    checkpoint_stat = snapshot(session_path(cwd))
    (ROOT/'checkpoint-original.json').write_bytes(checkpoint_bytes)
    return {'steps': steps, 'phase': current['state']['phase'], 'turn': current['state']['turn'],
            'checkpoint': checkpoint_stat, 'rng_seed_decimal': str(current['rngSeed']),
            'rng_calls': current['rngCalls'], 'prior_log_bytes': len(current['state']['log'].encode()),
            'support_key_observed': support_key, 'support_ids': [x[support_key] for x in slots]}

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
    assertion(after['state']['log'].startswith(checkpoint['state']['log']), 'Prior native log was discarded or rewritten')
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
