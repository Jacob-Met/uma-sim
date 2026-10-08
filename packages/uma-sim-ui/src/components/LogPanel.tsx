import { useId, useLayoutEffect, useMemo, useRef, useState } from "react";
import { historyChange, searchLog } from "./logView";
import { LogDownload } from "./LogDownload";
import "./logPanel.css";

interface Props {
  lines: string[];
  history: string[];
}

export function LogPanel({ lines, history }: Props) {
  const headingId = useId();
  const statusId = useId();
  const ref = useRef<HTMLDivElement>(null);
  const previous = useRef({ history, query: "" });
  const scrollTop = useRef(0);
  const [query, setQuery] = useState("");
  const [following, setFollowing] = useState(true);
  const view = useMemo(() => searchLog(history, query), [history, query]);

  useLayoutEffect(() => {
    const viewport = ref.current;
    if (!viewport) return;
    if (following) {
      viewport.scrollTop = viewport.scrollHeight;
    } else if (
      previous.current.query !== query ||
      historyChange(previous.current.history, history) === "replaced"
    ) {
      // A different filter or retained sequence has no reliable old position.
      viewport.scrollTop = 0;
    } else {
      viewport.scrollTop = scrollTop.current;
    }
    scrollTop.current = viewport.scrollTop;
    previous.current = { history, query };
  }, [history, query, following, view]);

  useLayoutEffect(() => {
    const viewport = ref.current;
    if (!viewport || !following) return;
    const observer = new ResizeObserver(() => {
      viewport.scrollTop = viewport.scrollHeight;
      scrollTop.current = viewport.scrollTop;
    });
    observer.observe(viewport);
    return () => observer.disconnect();
  }, [following]);

  function changeQuery(value: string) {
    setQuery(value);
    setFollowing(false);
  }

  function toggleFollowing() {
    if (following) {
      setFollowing(false);
    } else {
      setQuery("");
      setFollowing(true);
    }
  }

  return (
    <section className="card career-log" aria-labelledby={headingId}>
      <h2 id={headingId}>Career event log</h2>
      <p className="career-log-description">
        Retained history from the current career. Numbers show entry positions.
      </p>
      <div className="career-log-toolbar">
        <label className="career-log-search">
          <span>Search retained events</span>
          <input
            type="search"
            value={query}
            onChange={(event) => changeQuery(event.target.value)}
            placeholder="Search literal text…"
            aria-describedby={statusId}
          />
        </label>
        {query && <button onClick={() => changeQuery("")}>Clear search</button>}
        <button data-following={following} aria-describedby={statusId} onClick={toggleFollowing}>
          {following
            ? "Pause following"
            : query
              ? "Clear search & follow latest"
              : "Follow latest"}
        </button>
      </div>
      <p id={statusId} className="career-log-status" role="status" aria-atomic="true">
        {query
          ? `${view.entries.length} of ${view.totalEntries} entries match · ${view.matchCount} occurrences · Following paused`
          : `${view.totalEntries} retained ${view.totalEntries === 1 ? "entry" : "entries"} · ${following ? "Following latest" : "Following paused"}`}
      </p>
      <div
        className="career-log-viewport"
        ref={ref}
        role="region"
        aria-label="Retained career events"
        tabIndex={0}
        onScroll={(event) => {
          const viewport = event.currentTarget;
          scrollTop.current = viewport.scrollTop;
          if (viewport.scrollHeight - viewport.clientHeight - viewport.scrollTop > 8) {
            setFollowing(false);
          }
        }}
      >
        {view.entries.length > 0 ? (
          <ol>
            {view.entries.map((entry) => (
              <li key={entry.index} value={entry.index + 1}>
                {entry.parts.map((part, index) =>
                  part.matched ? <mark key={index}>{part.text}</mark> : part.text,
                )}
              </li>
            ))}
          </ol>
        ) : history.length === 0 ? (
          <p>No retained events yet.</p>
        ) : (
          <p>No retained entries match “{query}”.</p>
        )}
      </div>
      <LogDownload history={history} />
      <details className="career-log-summary">
        <summary>Current state summary</summary>
        <pre>{lines.length ? lines.join("\n") : "No current state summary."}</pre>
      </details>
    </section>
  );
}
