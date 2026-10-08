# UMA branch-trace independent receiving contract

This contract is established from the pre-existing `LabBranchResult`,
`BranchStep`, and `api.labBranchGet` interfaces at source commit
`f5f9b29393731d18aee2d66a31d89c315aa79c60`, before inspecting the new hook or panel.
It evaluates branch inspection and export, with no claim to independently
validate the simulator's telemetry.

## Invariants

1. The branch being requested, any displayed trace, visible error, retry action,
   and downloadable JSON each have an unambiguous branch identity. A loading or
   failed request for B cannot label A's retained trace or export as B.
2. Opening B after A, closing the inspector, and retrying establish new request
   intent. A late response or error from an older request cannot replace a newer
   result, display an obsolete error, or reopen a closed inspector. Retry targets
   the branch whose failure the inspector identifies. The same applies after
   unmounting the lab view or acknowledging deletion of the inspected branch.
3. Inspection displays the stored branch configuration and actual per-step
   override flags. If a turn has a rejected step followed by an applied step,
   both observations remain visible in their respective step/phase context.
   The UI must not describe that whole override as exclusively rejected or
   successful, infer a rejection reason absent from the payload, or manufacture
   a causal benefit.
4. Configured override turns without a corresponding trace observation are
   distinguishable from explicit rejection. A configured turn outside or absent
   from the recorded timeline is described as unobserved rather than rejected;
   absence does not establish why the engine did not record it.
5. The JSON download contains the entire displayed `LabBranchResult` object,
   including configuration, timeline, outcome, and additional payload fields.
   It uses the displayed response identity rather than a mutable pending request
   or comparison selection. Downloaded JSON is parsed and compared structurally
   with the exact HTTP response supplied to the browser.
6. Inspection is read-only: opening, switching, closing, retrying, and downloading
   cause no mutation API requests. Branch IDs containing reserved URL characters
   must reach the existing retrieval endpoint unchanged after decoding.
   A separately requested, confirmed branch deletion invalidates any inspection
   of that branch; a failed deletion does not silently erase an otherwise valid
   trace or misrepresent the branch as deleted.

## Receiving method

Execute the actual built application in Chromium with an independent loopback
HTTP fixture. Deliver controlled response ordering and failures at the existing
branch retrieval endpoint. Exercise visible controls and inspect actual JSON
files downloaded by the browser. Preserve original-payload and build hashes,
the test definition, results, and the distinction between component verification
and final application integration.
