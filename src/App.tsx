import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  Mic,
  Download,
  Copy,
  Check,
  Keyboard,
  Cpu,
  ShieldAlert,
  Apple,
  Monitor,
} from "lucide-react";

interface WhistleStatus {
  model_ready: boolean;
  runtime_ready: boolean;
  model_path: string;
  runtime_path: string;
  platform_folder: string;
}

interface AppSettings {
  language: string;
  auto_paste: boolean;
  trigger_mode: "toggle" | "hold";
}

interface TranscriptionItem {
  id: string;
  text: string;
  latency_ms: number;
  timestamp: string;
}

// ── Desktop App Download Links ──
// Update GITHUB_REPO to your actual GitHub username/repo after publishing.
const GITHUB_REPO = "snowhypers/wisperSpeech";
const APP_VERSION = "0.1.0";
const DOWNLOAD_LINKS = {
  dmg: `https://github.com/${GITHUB_REPO}/releases/download/v${APP_VERSION}/WhistleFlow_${APP_VERSION}_aarch64.dmg`,
  exe: `https://github.com/${GITHUB_REPO}/releases/download/v${APP_VERSION}/WhistleFlow_${APP_VERSION}_x64-setup.exe`,
  appimage: `https://github.com/${GITHUB_REPO}/releases/download/v${APP_VERSION}/WhistleFlow_${APP_VERSION}_amd64.AppImage`,
  allReleases: `https://github.com/${GITHUB_REPO}/releases/latest`,
};

const WHISTLE_LANGUAGES = [
  { code: "auto", label: "Auto-detect (7 languages)" },
  { code: "en", label: "English" },
  { code: "de", label: "German" },
  { code: "fr", label: "French" },
  { code: "es", label: "Spanish" },
  { code: "it", label: "Italian" },
  { code: "nl", label: "Dutch" },
  { code: "pl", label: "Polish" },
];

export default function App() {
  const [status, setStatus] = useState<WhistleStatus | null>(null);
  const [settings, setSettings] = useState<AppSettings>({
    language: "auto",
    auto_paste: true,
    trigger_mode: "toggle",
  });
  const [downloading, setDownloading] = useState(false);
  const [recording, setRecording] = useState(false);
  const [history, setHistory] = useState<TranscriptionItem[]>([]);
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const refreshStatus = async () => {
    try {
      const s = await invoke<WhistleStatus>("get_whistle_status");
      setStatus(s);
    } catch (e) {
      setErrorMsg(String(e));
    }
  };

  useEffect(() => {
    refreshStatus();
    invoke<AppSettings>("get_settings").then(setSettings).catch(() => {});

    const unlistenTx = listen<TranscriptionItem>(
      "transcription-complete",
      (ev) => {
        setHistory((prev) => [ev.payload, ...prev].slice(0, 30));
      }
    );

    const unlistenState = listen<{ phase: string; message?: string }>(
      "dictation-state",
      (ev) => {
        setRecording(ev.payload.phase === "recording");
        if (ev.payload.phase === "error" && ev.payload.message) {
          setErrorMsg(ev.payload.message);
        }
      }
    );

    return () => {
      unlistenTx.then((f) => f());
      unlistenState.then((f) => f());
    };
  }, []);

  const handleDownload = async () => {
    setDownloading(true);
    setErrorMsg(null);
    try {
      const updated = await invoke<WhistleStatus>("download_whistle_assets");
      setStatus(updated);
    } catch (e) {
      setErrorMsg(`Download failed: ${String(e)}`);
    } finally {
      setDownloading(false);
    }
  };

  const updateSettings = async (patch: Partial<AppSettings>) => {
    const next = { ...settings, ...patch };
    setSettings(next);
    await invoke("update_settings", { settings: next });
  };

  const toggleTestRecording = async () => {
    setErrorMsg(null);
    try {
      await invoke("toggle_dictation");
    } catch (e) {
      setErrorMsg(String(e));
    }
  };

  const copyItem = async (item: TranscriptionItem) => {
    await navigator.clipboard.writeText(item.text);
    setCopiedId(item.id);
    setTimeout(() => setCopiedId(null), 1400);
  };

  const ready = Boolean(status?.model_ready && status?.runtime_ready);

  return (
    <div className="app-shell">
      <header className="header">
        <div className="brand">
          <div className="brand-icon">
            <Mic size={20} />
          </div>
          <div>
            <h1>WhistleFlow</h1>
            <p>Offline Voice-to-Text powered by Cactus Whistle (16.9 MB)</p>
          </div>
        </div>
        <div className="badge">
          <span className={`dot ${ready ? "green" : "amber"}`} />
          {ready ? "Whistle Ready (Local CPU)" : "Setup Required"}
        </div>
      </header>

      {/* ── Download Desktop App Banner ── */}
      <section className="card download-banner">
        <div className="card-title">
          <Download size={14} /> Download WhistleFlow Desktop App
        </div>
        <p style={{ fontSize: 13, color: "var(--muted)", lineHeight: 1.5 }}>
          Get the native desktop app for offline voice-to-text dictation.
          Works without internet — everything runs locally on your CPU.
        </p>
        <div className="download-buttons">
          <a
            href={DOWNLOAD_LINKS.dmg}
            className="btn primary download-btn"
            target="_blank"
            rel="noopener noreferrer"
          >
            <Apple size={16} />
            <div className="download-btn-text">
              <span className="download-btn-label">Download for macOS</span>
              <span className="download-btn-meta">.dmg — Apple Silicon</span>
            </div>
          </a>
          <a
            href={DOWNLOAD_LINKS.exe}
            className="btn download-btn"
            target="_blank"
            rel="noopener noreferrer"
          >
            <Monitor size={16} />
            <div className="download-btn-text">
              <span className="download-btn-label">Download for Windows</span>
              <span className="download-btn-meta">.exe installer</span>
            </div>
          </a>
          <a
            href={DOWNLOAD_LINKS.appimage}
            className="btn download-btn"
            target="_blank"
            rel="noopener noreferrer"
          >
            <Monitor size={16} />
            <div className="download-btn-text">
              <span className="download-btn-label">Download for Linux</span>
              <span className="download-btn-meta">.AppImage</span>
            </div>
          </a>
        </div>
        <a
          href={DOWNLOAD_LINKS.allReleases}
          target="_blank"
          rel="noopener noreferrer"
          style={{ fontSize: 12, color: "var(--accent-2)", textDecoration: "none" }}
        >
          View all releases on GitHub →
        </a>
      </section>

      {errorMsg && (
        <div
          className="notice"
          style={{
            borderColor: "rgba(239,68,68,0.4)",
            background: "rgba(239,68,68,0.1)",
            color: "#fca5a5",
          }}
        >
          {errorMsg}
        </div>
      )}

      <div className="grid">
        <section className="card">
          <div className="card-title">
            <Cpu size={14} /> Cactus Whistle Model (16.9 MB)
          </div>
          <div className="row">
            <span>Model weights (`whistle.cact`)</span>
            <span>{status?.model_ready ? "Installed" : "Missing"}</span>
          </div>
          <div className="row">
            <span>Cactus engine (`{status?.platform_folder ?? "..."}`)</span>
            <span>{status?.runtime_ready ? "Installed" : "Missing"}</span>
          </div>
          <button
            className="btn primary"
            onClick={handleDownload}
            disabled={downloading}
          >
            <Download size={15} />
            {downloading
              ? "Downloading Whistle & Engine..."
              : ready
              ? "Re-verify / Update Whistle"
              : "Download Whistle (16.9 MB)"}
          </button>
        </section>

        <section className="card">
          <div className="card-title">
            <Keyboard size={14} /> Hotkey & Input Insertion
          </div>
          <div className="row">
            <span>Global Shortcut</span>
            <span className="kbd">Option + Space / Alt + Space</span>
          </div>
          <div className="row">
            <span>Activation Mode</span>
            <select
              value={settings.trigger_mode}
              onChange={(e) =>
                updateSettings({
                  trigger_mode: e.target.value as "toggle" | "hold",
                })
              }
            >
              <option value="toggle">Press to Start / Press to Stop</option>
              <option value="hold">Hold to Talk (Push-to-Talk)</option>
            </select>
          </div>
          <div className="row">
            <span>Language</span>
            <select
              value={settings.language}
              onChange={(e) => updateSettings({ language: e.target.value })}
            >
              {WHISTLE_LANGUAGES.map((l) => (
                <option key={l.code} value={l.code}>
                  {l.label}
                </option>
              ))}
            </select>
          </div>
          <div className="row">
            <span>Auto-paste into active field</span>
            <input
              type="checkbox"
              checked={settings.auto_paste}
              onChange={(e) =>
                updateSettings({ auto_paste: e.target.checked })
              }
            />
          </div>
        </section>
      </div>

      <section className="card">
        <div className="row">
          <div className="card-title" style={{ margin: 0 }}>
            Recent Transcriptions
          </div>
          <button
            className="btn"
            onClick={toggleTestRecording}
            disabled={!ready}
          >
            <Mic size={14} />
            {recording ? "Stop & Transcribe" : "Test Dictation Now"}
          </button>
        </div>

        <div className="history-list">
          {history.length === 0 ? (
            <div className="notice">
              Focus any text box (browser, Slack, VS Code, Notes) and press{" "}
              <strong>Option + Space</strong> (macOS) or{" "}
              <strong>Alt + Space</strong> (Windows/Linux) to dictate directly
              into the input field.
            </div>
          ) : (
            history.map((item) => (
              <div key={item.id} className="history-item">
                <div>
                  <div className="history-text">{item.text}</div>
                  <div className="history-meta">
                    {item.timestamp} · {item.latency_ms} ms
                  </div>
                </div>
                <button className="btn" onClick={() => copyItem(item)}>
                  {copiedId === item.id ? (
                    <Check size={14} />
                  ) : (
                    <Copy size={14} />
                  )}
                </button>
              </div>
            ))
          )}
        </div>
      </section>

      <div className="notice" style={{ display: "flex", gap: 10, alignItems: "center" }}>
        <ShieldAlert size={16} style={{ flexShrink: 0 }} />
        <span>
          <strong>macOS Permissions:</strong> Grant <em>Microphone</em> and{" "}
          <em>Accessibility</em> permissions in System Settings → Privacy &amp;
          Security so WhistleFlow can auto-paste into active input fields.
        </span>
      </div>
    </div>
  );
}
