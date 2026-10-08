/**
 * Put an exported snapshot into the existing import envelope without rewriting
 * JSON numbers through JavaScript Number. The parsed value is used only to
 * validate the outer shape; the native API validates the snapshot itself.
 */
export function checkpointImportBody(snapshotJson: string, name?: string): string {
  let snapshot: unknown;
  try {
    snapshot = JSON.parse(snapshotJson);
  } catch {
    throw new Error("Import: not valid JSON");
  }
  if (snapshot === null || typeof snapshot !== "object" || Array.isArray(snapshot)) {
    throw new Error("Import: checkpoint must be a JSON object");
  }
  const nameField = name === undefined ? "" : `,"name":${JSON.stringify(name)}`;
  return `{"snapshot":${snapshotJson}${nameField}}`;
}
