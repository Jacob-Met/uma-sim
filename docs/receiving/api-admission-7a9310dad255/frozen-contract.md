# Independent Uma API request-admission receiving contract

Frozen before the reviewer sees the correction. Baseline source:
Jacob-Met/uma-sim main 0ac14602addd1a4610aa8359896915a34cce5659,
api.rs Git blob 4ff7034bc5701d793cd93d645d89d40d73fc3273.
Only baseline API, README and operating rules have been read.

The beneficiary is a player or agent using the actual local REST career API.
A nonempty unreadable, malformed or nonobject JSON request must not turn into
a default game command. Blank and whitespace bodies retain their established
empty-object behavior. The #63 session-resolution semantics are not changed
or used as an acceptance requirement here.

## Frozen observations

1. In separate disposable API processes, reject malformed bodies for Start,
Action, Auto, Fast, session fork, checkpoint save and branch execution. Require
HTTP400, a JSON error with the existing CORS header, and no change to the
session inventory/active selection, complete named/active snapshots, readable
career/telemetry view, checkpoint/branch lists or stored file bytes.
2. Reject nonobject JSON (null, boolean, array and string) on Auto with the
same no-effect rule. This checks real default-command dispatch, not a parser
helper in isolation.
3. Reject an actual invalid UTF-8 body and a truncated JSON body over a
half-closed TCP request. Transport refusal may close the malformed request,
but the server must remain healthy and all game/storage state must be intact.
A well-formed JSON prefix with an incomplete declared Content-Length is also
a bounded receiving challenge; a 2xx result and mutation cannot count as
successful admission.
4. After each refusal, one valid explicit Auto command on each of two identical
seeded named careers must produce identical complete snapshots. This exposes
hidden random-stream advancement as well as visible turn changes.
5. Preserve valid empty and whitespace bodies, an explicit object, and UTF-8
object fields. Valid Auto advances exactly like the named sibling control;
valid Start preserves the exact UTF-8 label and returns a usable session.
6. Preserve existing request routing: valid-body unknown route404, wrong-method
known route405, and valid-object unknown session404, without state changes.

Each case launches the exact supplied API executable on an ephemeral loopback
port, in a fresh directory under the receiver's evidence path. Catalog data is
read from an explicitly supplied existing source directory. Only synthetic
seeded careers and that new directory's .uma-sim files are used. No live API,
existing career store, provider process, external policy or owner checkout is
modified. The client uses Node built-ins; requests go only to its child server.
Actual source/binary hashes, original failures, responses and complete before/
after snapshot evidence are retained. Setup/transport failures remain distinct
from behavioral acceptance. Full repository CI and merge gates are separate.
