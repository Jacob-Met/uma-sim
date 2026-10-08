# Independent source composition: checkpoint replacement recovery

Contributor: `ac386303dce2 / runtime_execution`, separate from the checkpoint
implementation author. No source blocker was found in the frozen composition
`937f9663b85a5809160b92071f7065760872756346d3f4919054586855c29dd3`.

The reviewer independently retrieved `career_lab.rs` from canonical main
`b8e3da0c6e152f77aa5c3e083ffa7b248bd74f8c` through GitHub. Its Git blob is
`4360cd3dbca832a9a22be138ca13a5deb4dfeb0f` and SHA-256 is
`f12208e5796ac6ab42ba515d6d8e6734388810894b600420e4badfdeab250f08`.
Those retrieved bytes were compared with the original native baseline and the
previously reviewed checkpoint source `e84d58202d8b6fb47a16af7e8704fbccd723bb7f35a2648fc99f5134430732df`.
The final candidate was then independently copied by its frozen hash.

## What the composition preserves

The only delta from the previously reviewed recovery source is the received
main change: `OpenOptions`/`Write` imports, the broader `AlreadyExists` diagnostic,
branch ID generation, and immutable `BranchStore::save` publication. Each of
those changes matches main. The recovery helpers, `CareerLibrary::save`, its
orphan-snapshot refusal, the underlying `atomic_write`, and the added rollback
unit test retain their previously reviewed bytes.

The branch publication path no longer uses the checkpoint file writer. Its
counter and temporary name generation are separate from the recovery helper's
private backup counter. The added imports do not change the helper's qualified
`fs::OpenOptions` or `std::io::copy` calls. The wider already-exists message does
not change error variants or the preservation/rollback decisions. No newly
introduced source interaction was found that changes the exercised returned-I/O
recovery behavior.

An independent three-way merge of the exact original recovery source and the
independently fetched main source over their original native baseline is clean
and byte-identical to the frozen composition. `source-binding.json` records
those identities; `current-main-composition.diff` retains the reviewed delta.

## Receiving boundary

The independent three-case native HTTP/filesystem qualification remains bound
to the original `e84d5820...` source and the actually exercised API binary
`bb9768847308f943b298d28a6afefd6575092fcd0ebad21b115ee0284622977d`.
The exact final test is
`46ab3397ad67ee09ab317fbde7ecaf9d6fb14f928f19e710f40825e80ace5f81`:
the baseline has two failing preservation cases and one passing control; the
candidate passes all three with no skips. The retained original qualification
explains the fault injection, failed independent build, artifact custody, and
Linux-only limits.

This source composition receipt does not relabel that earlier artifact as a
build of the newer main. The integrating contributor must qualify the final
published head through the existing native build/domain/REST gates. There is
no claim here of a newer local binary run, cross-process exclusion for library
replacement, transactional two-file publication, power-loss recovery, Windows
qualification, or a deployed runtime update.
