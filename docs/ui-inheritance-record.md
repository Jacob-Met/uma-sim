# Career inheritance record

The Run tab includes an **Inheritance record** card immediately after Aptitude. It is available while a career is running and remains available for a finished career.

The compact overview shows the retained parent names, whether the initial inheritance choice is complete, and the recorded count of completed mid-run inspirations. Open **Lineage and recorded sparks** to inspect the six recorded ancestor positions, factor names and IDs, recorded stars, compatibility score, and flat factor entries. Open **Retained factors and bonuses** to inspect retained factor occurrences, stat-cap additions, starting-stat bonuses, and inherited skill candidates.

The record is read-only. Its disclosures work with mouse or keyboard. Reading, scrolling, opening, or closing the card does not call the API, change a career, or save a checkpoint. Switching the active career through the existing Library & lab controls replaces the displayed record.

## Reading incomplete or older records

- A structured tree identifies Parent A, Grandparents A1/A2, Parent B, and Grandparents B1/B2. A missing ancestor or spark slot is identified explicitly.
- Flat entries remain literal strings. They do not identify ancestor positions, so the card does not reconstruct a tree or infer star counts from them.
- Missing values say **Not recorded**. Recorded empty arrays/maps have their own empty messages; a recorded zero is still shown as zero.
- Stars are displayed as recorded, including values outside the setup editor's normal range. The record does not reuse the editor's defaulting or clamping.
- A known factor shows its current catalog name and its retained ID. An unknown ID stays visible as text. Repeated factor IDs remain separate occurrences.
- Stat-cap additions and starting-stat bonuses are the values already stored by the native engine. The card does not recompute bonuses, predict future inspiration, or recalculate compatibility.
- **Initial inheritance choice: Completed** records completion only. It does not identify the chosen option.
- **Inherited skill candidates** are the IDs retained for the inheritance choice. The existing Skills panel is the place to inspect skills actually learned.

## Maintenance

The view model is `packages/uma-sim-ui/src/components/inheritanceView.ts`; the React card and its scoped CSS are adjacent. Optional TypeScript mirror fields reflect the native snapshot schema. Native inheritance mechanics, setup import/export, session routing, and save formats are unchanged.

The eight focused data-contract cases are included in the existing UI test command:

```sh
cd packages/uma-sim-ui
npm test
npm run typecheck
npm run build
```

The test suite covers complete, partial, missing and flat-only records; literal stars and repeated factors; stored bonus maps; unknown IDs; changed careers/catalogs; and input immutability. Exact native/browser receiving evidence and its source boundaries are documented in `docs/receiving/inheritance-record-401c5d17da79/README.md`.
