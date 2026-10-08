export type LogExportFormat = "text" | "json";

export interface LogExportFile {
  filename: string;
  contentType: string;
  content: string;
}

/** Capture every currently retained entry, independently of the reader's filter. */
export function retainedLogFile(history: readonly string[], format: LogExportFormat): LogExportFile {
  const entries = [...history];
  if (format === "json") {
    return {
      filename: "uma-sim-retained-log.json",
      contentType: "application/json;charset=utf-8",
      content: JSON.stringify({
        format: "uma-sim-retained-log/1",
        scope: "displayed-retained-history",
        entryCount: entries.length,
        entries,
      }, null, 2) + "\n",
    };
  }
  if (format === "text") {
    const header = [
      "Uma Sim — retained career event log",
      `${entries.length} retained ${entries.length === 1 ? "entry" : "entries"}`,
      "This file contains the entries retained by the displayed career.",
      "Earlier events may no longer be retained. Entry numbers are positions in this file.",
    ].join("\n");
    const body = entries.map((entry, index) => `[Entry ${index + 1}]\n${entry}`).join("\n\n");
    return {
      filename: "uma-sim-retained-log.txt",
      contentType: "text/plain;charset=utf-8",
      content: header + (body ? "\n\n" + body : "") + "\n",
    };
  }
  throw new Error("Unsupported log file format");
}

/** Request a local download; the browser owns the save decision and completion. */
export function downloadLogFile(file: LogExportFile): void {
  const blob = new Blob([file.content], { type: file.contentType });
  const url = URL.createObjectURL(blob);
  let link: HTMLAnchorElement | undefined;
  try {
    link = document.createElement("a");
    link.href = url;
    link.download = file.filename;
    document.body.appendChild(link);
    link.click();
  } finally {
    // Let the browser consume the click before releasing the temporary URL.
    setTimeout(() => URL.revokeObjectURL(url), 0);
    link?.remove();
  }
}
