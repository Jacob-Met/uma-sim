import { useState } from "react";
import { downloadLogFile, retainedLogFile, type LogExportFormat } from "./logExport";

export function LogDownload({ history }: { history: readonly string[] }) {
  const [format, setFormat] = useState<LogExportFormat>("text");
  const [failed, setFailed] = useState(false);

  function download() {
    setFailed(false);
    try {
      downloadLogFile(retainedLogFile(history, format));
    } catch {
      setFailed(true);
    }
  }

  return (
    <div className="career-log-download">
      <label>
        <span>Log file format</span>
        <select value={format} onChange={(event) => setFormat(event.target.value as LogExportFormat)}>
          <option value="text">Plain text</option>
          <option value="json">JSON</option>
        </select>
      </label>
      <button disabled={history.length === 0} onClick={download}>
        Download all {history.length} {history.length === 1 ? "entry" : "entries"}
      </button>
      {failed && <p role="alert">Could not start the log download. Please try again.</p>}
    </div>
  );
}
