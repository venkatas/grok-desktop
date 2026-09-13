import type { AppStatus } from "../lib/types";

export function Setup({
  status,
  busy,
  error,
  onRecheck,
  onSignIn,
}: {
  status: AppStatus;
  busy?: boolean;
  error?: string | null;
  onRecheck: () => void;
  onSignIn: () => void;
}) {
  return (
    <div className="setup">
      <h1>Grok Build</h1>
      <p className="muted">Desktop command center for the Grok coding agent.</p>
      {!status.grok_bin ? (
        <div className="card">
          <h2>Install Grok CLI</h2>
          <pre className="cmd">curl -fsSL https://x.ai/cli/install.sh | bash</pre>
          <button className="primary" onClick={onRecheck} disabled={busy}>
            Recheck
          </button>
        </div>
      ) : !status.signed_in ? (
        <div className="card">
          <h2>Sign in</h2>
          <p>Uses your existing grok.com login. Tokens stay in auth.json.</p>
          <button className="primary" onClick={onSignIn} disabled={busy}>
            Sign in
          </button>
        </div>
      ) : null}
      {error ? <p className="err">{error}</p> : null}
    </div>
  );
}
