import { useEffect, useRef } from "react";

interface Props {
  lines: string[];
}

export function LogPanel({ lines }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (ref.current) {
      ref.current.scrollTop = ref.current.scrollHeight;
    }
  }, [lines]);

  return (
    <div className="card" style={{ display: "flex", flexDirection: "column" }}>
      <h2>Event log</h2>
      <div className="log" ref={ref}>
        {lines.length === 0 ? "(no lines yet)" : lines.join("\n")}
      </div>
    </div>
  );
}
