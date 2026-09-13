export function Composer({
  value,
  running,
  disabled,
  onChange,
  onSend,
  onStop,
}: {
  value: string;
  running: boolean;
  disabled?: boolean;
  onChange: (v: string) => void;
  onSend: () => void;
  onStop: () => void;
}) {
  return (
    <div className="composer">
      <textarea
        placeholder="Ask Grok…"
        value={value}
        disabled={disabled}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && !e.shiftKey) {
            e.preventDefault();
            if (!running && value.trim()) onSend();
          }
        }}
      />
      {running ? (
        <button className="primary" onClick={onStop}>
          Stop
        </button>
      ) : (
        <button className="primary" disabled={disabled || !value.trim()} onClick={onSend}>
          Send
        </button>
      )}
    </div>
  );
}
